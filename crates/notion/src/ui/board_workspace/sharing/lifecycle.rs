use std::sync::Arc;

use gpui::Context;

use super::operations::{ShareOperationJob, ShareOperationOutcome};
use super::{ShareEvent, ShareMutation};
use crate::model::{NotionShareTargetId, NotionWorkspaceApi, NotionWorkspaceOperationFailure};
use crate::ui::surface::{NotionChromeState, NotionShareDialogState, PageDocuments};
use crate::ui::SurfaceState;

struct ShareEventContext {
    active_target: Result<NotionShareTargetId, String>,
    workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
}

enum ShareEffect {
    OpenDialog(Box<NotionShareDialogState>),
    Spawn(Box<ShareOperationJob>),
    Failure(Box<ShareFailure>),
    Error(String),
    Notify,
}

struct ShareFailure {
    context: &'static str,
    error: NotionWorkspaceOperationFailure,
}

impl SurfaceState {
    pub(crate) fn dispatch_notion_share_event(
        &mut self,
        event: ShareEvent,
        cx: &mut Context<Self>,
    ) {
        let active_target = self
            .page_documents
            .active_share_target(self.board.share_target_id.as_ref());
        let context = ShareEventContext {
            active_target,
            workspace_api: self.notion_startup.workspace_api(),
        };
        let effects = self.notion_chrome.reduce_share_event(event, context);
        for effect in effects {
            match effect {
                ShareEffect::OpenDialog(dialog) => {
                    self.page_editor.reset_page_composer();
                    self.notion_chrome
                        .begin_share_dialog(*dialog, &mut self.database_search);
                    self.comments.dismiss_notion_comments_panel_without_notify();
                }
                ShareEffect::Spawn(job) => (*job).spawn(cx),
                ShareEffect::Failure(failure) => {
                    if !self.handle_notion_workspace_failure(failure.context, failure.error, cx) {
                        cx.notify();
                    }
                }
                ShareEffect::Error(error) => self.print_notion_error(error),
                ShareEffect::Notify => cx.notify(),
            }
        }
    }
}

impl NotionChromeState {
    fn reduce_share_event(
        &mut self,
        event: ShareEvent,
        context: ShareEventContext,
    ) -> Vec<ShareEffect> {
        match event {
            ShareEvent::Open => self.open_share_dialog(context),
            ShareEvent::Dismiss => self
                .dismiss_notion_share_dialog()
                .then_some(ShareEffect::Notify)
                .into_iter()
                .collect(),
            ShareEvent::Update(update) => self
                .share_dialog
                .as_mut()
                .is_some_and(|dialog| dialog.apply_view_action(update))
                .then_some(ShareEffect::Notify)
                .into_iter()
                .collect(),
            ShareEvent::Mutate(mutation) => self.begin_share_mutation(mutation, context),
            ShareEvent::Completion(completion) => {
                match self
                    .complete_share_operation(*completion, context.active_target.as_ref().ok())
                {
                    ShareOperationOutcome::Ignored => Vec::new(),
                    ShareOperationOutcome::Applied => vec![ShareEffect::Notify],
                    ShareOperationOutcome::Failed { context, error } => {
                        vec![ShareEffect::Failure(Box::new(ShareFailure {
                            context,
                            error,
                        }))]
                    }
                }
            }
        }
    }

    fn open_share_dialog(&mut self, context: ShareEventContext) -> Vec<ShareEffect> {
        let target_id = match context.active_target {
            Ok(target_id) => target_id,
            Err(error) => return vec![ShareEffect::Error(error)],
        };
        if self.share_dialog_is_open_for(&target_id) {
            self.share_dialog = None;
            return vec![ShareEffect::Notify];
        }
        let Some(workspace_api) = context.workspace_api else {
            return vec![ShareEffect::Error(
                "sharing requires a live Notion workspace API".to_string(),
            )];
        };
        let dialog = NotionShareDialogState::loading(target_id.clone());
        let job = ShareOperationJob::load(workspace_api, dialog.session.clone(), target_id);
        vec![
            ShareEffect::OpenDialog(Box::new(dialog)),
            ShareEffect::Notify,
            ShareEffect::Spawn(Box::new(job)),
        ]
    }

    fn begin_share_dialog(
        &mut self,
        dialog: NotionShareDialogState,
        database_search: &mut crate::ui::surface::DatabaseSearchState,
    ) {
        self.toolbar_dialog = None;
        self.inline_toolbar_dialog = None;
        self.date_undated_dialog = None;
        self.inline_database_view_menu = None;
        self.ai_autofill_dialog = None;
        self.status_property_picker = None;
        database_search.close();
        self.notion_search_open = false;
        self.share_dialog = Some(dialog);
    }

    fn begin_share_mutation(
        &mut self,
        mutation: ShareMutation,
        context: ShareEventContext,
    ) -> Vec<ShareEffect> {
        let Some(dialog) = self
            .share_dialog
            .as_mut()
            .filter(|dialog| dialog.can_mutate())
        else {
            return Vec::new();
        };
        let request = match mutation.request(dialog.target_id.clone()) {
            Ok(request) => request,
            Err(error) => return vec![ShareEffect::Error(error)],
        };
        let Some(workspace_api) = context.workspace_api else {
            return vec![ShareEffect::Error(
                "sharing requires a live Notion workspace API".to_string(),
            )];
        };
        dialog.mutation_in_flight = true;
        dialog.role_picker = None;
        let job = ShareOperationJob::mutation(
            workspace_api,
            dialog.session.clone(),
            dialog.target_id.clone(),
            request,
        );
        vec![ShareEffect::Notify, ShareEffect::Spawn(Box::new(job))]
    }

    fn share_dialog_is_open_for(&self, target_id: &NotionShareTargetId) -> bool {
        self.share_dialog
            .as_ref()
            .is_some_and(|dialog| &dialog.target_id == target_id)
    }

    pub(crate) fn dismiss_notion_share_dialog(&mut self) -> bool {
        self.share_dialog.take().is_some()
    }

    pub(crate) fn notion_share_dialog_handles_key_down(
        &self,
        event: &crate::ui::KeyDownEvent,
    ) -> bool {
        event.keystroke.key == "escape" && self.share_dialog.is_some()
    }
}

impl PageDocuments {
    fn active_share_target(
        &self,
        board_target: Option<&NotionShareTargetId>,
    ) -> Result<NotionShareTargetId, String> {
        if let Some(selected_page) = self.selected_page.as_ref() {
            return selected_page.block_id().parse().map_err(str::to_string);
        }
        if let Some(target_id) = board_target {
            return Ok(target_id.clone());
        }
        if let Some(page) = self.standalone.as_ref() {
            return page.data.page.block_id.parse().map_err(str::to_string);
        }
        Err("current Notion surface has no typed share target".to_string())
    }
}
