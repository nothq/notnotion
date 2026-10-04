use crate::ui::board_workspace::PageDocumentAction;
use gpui::Context;

use crate::ui::SurfaceState;

use super::action::{
    PageMutationAction, PageMutationEffect, PageMutationHostQueue, PageMutationHostStep,
};
use super::jobs::run_page_mutation_job;
use super::session::{PageMutationEnvironment, PageMutationSession};

impl SurfaceState {
    pub(crate) fn dispatch_page_mutation_action(
        &mut self,
        action: PageMutationAction,
        cx: &mut Context<Self>,
    ) {
        let mut pending = PageMutationHostQueue::new(action);
        while let Some(step) = pending.pop() {
            let effect = match step {
                PageMutationHostStep::Effect(effect) => effect,
                PageMutationHostStep::Action(action) => {
                    let environment = PageMutationEnvironment {
                        workspace_api: self.notion_startup.workspace_api(),
                        cached_workspace_visible: self.notion_startup.cached_workspace_visible(),
                    };
                    let effects = PageMutationSession::new(
                        &mut self.page_mutations,
                        &self.page_documents,
                        &mut self.notion_search,
                        &self.page_editor.page_link_icons,
                        environment,
                    )
                    .reduce(action);
                    pending.prepend_effects(effects);
                    continue;
                }
            };
            if !apply_page_mutation_host_effect(self, effect, &mut pending, cx) {
                return;
            }
        }
    }
}

fn apply_page_mutation_host_effect(
    surface: &mut SurfaceState,
    effect: PageMutationEffect,
    pending: &mut PageMutationHostQueue,
    cx: &mut Context<SurfaceState>,
) -> bool {
    match effect {
        PageMutationEffect::RunBackground(job) => surface.spawn_background_task(
            job,
            cx,
            run_page_mutation_job,
            SurfaceState::dispatch_page_mutation_action,
        ),
        PageMutationEffect::ScheduleRecovery(timer) => {
            let delay = timer.delay;
            surface.spawn_timer_task(
                timer.into_action(),
                delay,
                cx,
                SurfaceState::dispatch_page_mutation_action,
            );
        }
        PageMutationEffect::HandleWorkspaceFailure(failure) => {
            if surface.handle_notion_workspace_failure(failure.operation, failure.failure, cx) {
                return false;
            }
            pending.prepend_action(failure.continuation);
        }
        PageMutationEffect::ReplaceLoaded(replacement) => {
            if replacement.clear_history {
                surface
                    .page_editor
                    .page_edit_histories
                    .remove(&replacement.page_id);
            }
            surface.dispatch_page_document_action(
                PageDocumentAction::replace_loaded_and_authority(
                    replacement.page,
                    replacement.authority,
                ),
                cx,
            );
        }
        PageMutationEffect::AdvanceAuthority { page_id, authority } => surface
            .dispatch_page_document_action(
                PageDocumentAction::AdvanceAuthority { page_id, authority },
                cx,
            ),
        PageMutationEffect::RefreshSearch => {
            surface.refresh_notion_search_after_page_mutation(cx);
        }
        PageMutationEffect::PrintError(error) => surface.print_notion_error(error),
        PageMutationEffect::Continue(action) => {
            pending.prepend_action(*action);
        }
        PageMutationEffect::Notify => cx.notify(),
    }
    true
}
