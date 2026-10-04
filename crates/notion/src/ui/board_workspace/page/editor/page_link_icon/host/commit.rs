use std::sync::Arc;

use gpui::Context;

use super::super::{
    commit::{PageLinkIconCommitCompletion, PreparedPageLinkIconSave},
    persistence::PageLinkIconPersistence,
    selection::PageLinkIconSetEffect,
    PageLinkIconTarget,
};
use super::jobs::PageLinkIconCommitJob;
use super::{
    PageLinkIconEffectHost, PageLinkIconFailureRecovery, PageLinkIconRootEffect,
    PageLinkIconWorkspaceFailure,
};
use crate::ui::SurfaceState;

struct PageLinkIconCommittedIcon {
    context: super::super::commit::PageLinkIconCommitContext,
    icon: crate::model::PageShellIcon,
}

impl PageLinkIconEffectHost<'_> {
    pub(super) fn save_upload(
        &mut self,
        save: PreparedPageLinkIconSave,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        match save {
            PreparedPageLinkIconSave::SetIcon(effect) => {
                self.prepare_set_icon(effect, PageLinkIconPersistence::Queue, cx)
            }
            PreparedPageLinkIconSave::Commit(start) => {
                let Some(workspace_api) = self.workspace_api.clone() else {
                    println!(
                        "notnotion: {} requires the live Notion workspace",
                        start.operation
                    );
                    return Vec::new();
                };
                let Some(pending) = self.editor.page_link_icons.start_commit(start) else {
                    return Vec::new();
                };
                let page_id = pending.context.page_id.clone();
                let block_id = pending.context.block_id.clone();
                vec![
                    PageLinkIconRootEffect::PageMutation(Box::new(
                        crate::ui::board_workspace::PageMutationAction::BeginSearchMutation {
                            lane_page_id: page_id.clone(),
                            block_id: page_id.clone(),
                        },
                    )),
                    PageLinkIconRootEffect::PageMutation(Box::new(
                        crate::ui::board_workspace::PageMutationAction::BeginSearchMutation {
                            lane_page_id: page_id.clone(),
                            block_id,
                        },
                    )),
                    PageLinkIconRootEffect::PageMutation(Box::new(
                        crate::ui::board_workspace::PageMutationAction::PersistSearchMutation {
                            lane_page_id: page_id,
                            workspace_api: Arc::clone(&workspace_api),
                        },
                    )),
                    PageLinkIconRootEffect::SpawnCommit(Box::new(PageLinkIconCommitJob {
                        workspace_api,
                        pending,
                    })),
                    PageLinkIconRootEffect::Notify,
                ]
            }
        }
    }

    pub(super) fn resolve_commit(
        &mut self,
        completion: PageLinkIconCommitCompletion,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        let resolution = self.editor.page_link_icons.resolve_commit(completion);
        let super::super::commit::PageLinkIconCommitResolution {
            context,
            result,
            commit_matches,
            picker_matches,
        } = resolution;
        let page_id = context.page_id.clone();
        let mut effects = match result {
            Ok(icon) => self.resolve_successful_commit(
                PageLinkIconCommittedIcon { context, icon },
                commit_matches,
                picker_matches,
                cx,
            ),
            Err(error) => vec![PageLinkIconRootEffect::WorkspaceFailure(Box::new(
                PageLinkIconWorkspaceFailure {
                    operation: context.operation,
                    error,
                    recovery: Some(PageLinkIconFailureRecovery::FinishFailedCommit {
                        picker_matches,
                    }),
                    notify_after_recovery: false,
                },
            ))],
        };
        if commit_matches {
            effects.push(PageLinkIconRootEffect::FinishSearchMutation { page_id });
        }
        effects.push(PageLinkIconRootEffect::Notify);
        effects
    }

    fn resolve_successful_commit(
        &mut self,
        committed: PageLinkIconCommittedIcon,
        commit_matches: bool,
        picker_matches: bool,
        cx: &mut Context<SurfaceState>,
    ) -> Vec<PageLinkIconRootEffect> {
        let PageLinkIconCommittedIcon { context, icon } = committed;
        self.resources.external_icon_cache().insert(
            icon.render_value().to_string(),
            Arc::clone(&context.rendered),
            cx,
        );
        self.editor
            .page_link_icons
            .finish_successful_commit(picker_matches);
        if !commit_matches {
            println!(
                "notnotion: {} completed without its pending-operation state",
                context.operation
            );
            return Vec::new();
        }
        self.prepare_set_icon(
            PageLinkIconSetEffect {
                target: PageLinkIconTarget {
                    page_id: context.page_id,
                    block_id: context.block_id,
                },
                icon: Some(icon),
                rendered: None,
                keep_picker_open: false,
            },
            PageLinkIconPersistence::AlreadyCommitted,
            cx,
        )
    }
}

impl PageLinkIconFailureRecovery {
    pub(super) fn apply(self, controller: &mut super::super::PageLinkIconController) {
        match self {
            Self::FinishFailedCommit { picker_matches } => {
                controller.finish_failed_commit(picker_matches);
            }
        }
    }
}
