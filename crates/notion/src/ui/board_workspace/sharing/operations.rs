use std::sync::Arc;

use gpui::Context;
use gpui_components::spawn_background_task_for_entity;

use super::ShareEvent;
use crate::model::{
    LoadNotionPageSharingRequest, MutateNotionPageSharingRequest, NotionPageSharingSnapshot,
    NotionShareTargetId, NotionWorkspaceApi, NotionWorkspaceOperationFailure,
    NotionWorkspaceResult,
};
use crate::ui::surface::NotionChromeState;
use crate::ui::SurfaceState;

enum ShareOperation {
    Load(NotionShareTargetId),
    Mutate(MutateNotionPageSharingRequest),
}

pub(super) struct ShareOperationJob {
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    operation: ShareOperation,
    identity: ShareOperationIdentity,
}

pub(crate) struct ShareOperationResult {
    identity: ShareOperationIdentity,
    result: NotionWorkspaceResult<NotionPageSharingSnapshot>,
}

struct ShareOperationIdentity {
    session: Arc<()>,
    target: NotionShareTargetId,
    kind: ShareOperationKind,
}

#[derive(Clone, Copy)]
enum ShareOperationKind {
    Load,
    Mutation,
}

pub(super) enum ShareOperationOutcome {
    Ignored,
    Applied,
    Failed {
        context: &'static str,
        error: NotionWorkspaceOperationFailure,
    },
}

impl ShareOperationJob {
    pub(super) fn load(
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        session: Arc<()>,
        target: NotionShareTargetId,
    ) -> Self {
        Self {
            workspace_api,
            operation: ShareOperation::Load(target.clone()),
            identity: ShareOperationIdentity {
                session,
                target,
                kind: ShareOperationKind::Load,
            },
        }
    }

    pub(super) fn mutation(
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        session: Arc<()>,
        target: NotionShareTargetId,
        request: MutateNotionPageSharingRequest,
    ) -> Self {
        Self {
            workspace_api,
            operation: ShareOperation::Mutate(request),
            identity: ShareOperationIdentity {
                session,
                target,
                kind: ShareOperationKind::Mutation,
            },
        }
    }

    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        let Self {
            workspace_api,
            operation,
            identity,
        } = self;
        spawn_background_task_for_entity(
            operation,
            cx,
            move |operation| operation.execute(workspace_api.as_ref()),
            move |surface, result, cx| {
                surface.dispatch_notion_share_event(
                    ShareEvent::Completion(Box::new(ShareOperationResult { identity, result })),
                    cx,
                );
            },
        );
    }
}

impl ShareOperation {
    fn execute(
        self,
        api: &dyn NotionWorkspaceApi,
    ) -> NotionWorkspaceResult<NotionPageSharingSnapshot> {
        match self {
            Self::Load(target) => api.load_page_sharing(LoadNotionPageSharingRequest::new(target)),
            Self::Mutate(request) => api.mutate_page_sharing(request),
        }
    }
}

impl NotionChromeState {
    pub(super) fn complete_share_operation(
        &mut self,
        completion: ShareOperationResult,
        active_target: Option<&NotionShareTargetId>,
    ) -> ShareOperationOutcome {
        let ShareOperationResult { identity, result } = completion;
        if active_target != Some(&identity.target) {
            return ShareOperationOutcome::Ignored;
        }
        let Some(dialog) = self.share_dialog.as_mut().filter(|dialog| {
            Arc::ptr_eq(&dialog.session, &identity.session) && dialog.target_id == identity.target
        }) else {
            return ShareOperationOutcome::Ignored;
        };
        match result {
            Ok(snapshot) => {
                if dialog.target_id != snapshot.target_id {
                    return ShareOperationOutcome::Ignored;
                }
                dialog.replace_snapshot(snapshot);
                ShareOperationOutcome::Applied
            }
            Err(error) => {
                let context = match identity.kind {
                    ShareOperationKind::Load => {
                        self.share_dialog = None;
                        "sharing state load failed"
                    }
                    ShareOperationKind::Mutation => {
                        dialog.mutation_in_flight = false;
                        "sharing update failed"
                    }
                };
                ShareOperationOutcome::Failed { context, error }
            }
        }
    }
}
