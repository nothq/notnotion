use std::sync::Arc;

use crate::model::{CardPage, PageMutationResult};
use crate::ui::surface::{CompletedPageMutationWrite, PageMutationRunToken};

use super::super::action::{
    PageMutationAction, PageMutationEffect, PageMutationInternalAction,
    PageMutationProjectionReplacement, PageMutationRecoveryDispatch, PageMutationRecoveryTimer,
    PageMutationWorkspaceFailure, PageMutationWriteCompletion, PageSuccessfulReplayFailure,
};
use super::super::rebase::{project_page_write, PageWriteReplayError};
use super::PageMutationSession;

struct SuccessfulPageReplay {
    authority: Arc<CardPage>,
    optimistic: Option<Arc<CardPage>>,
}

/// The rebase baseline and the error that stopped the replay.
type FailedPageReplay = (Arc<CardPage>, PageWriteReplayError);

impl PageMutationSession<'_> {
    pub(super) fn finish_write(
        &mut self,
        completion: PageMutationWriteCompletion,
    ) -> Vec<PageMutationEffect> {
        if !self
            .coordinator
            .is_writing(&completion.page_id, completion.token)
        {
            return Vec::new();
        }
        match completion.result {
            Ok(result) => {
                self.finish_successful_write(completion.page_id, completion.token, result)
            }
            Err(failure) => vec![PageMutationEffect::HandleWorkspaceFailure(Box::new(
                PageMutationWorkspaceFailure {
                    operation: "page save failed",
                    failure,
                    continuation: PageMutationAction::Internal(
                        PageMutationInternalAction::ContinueWriteFailure {
                            page_id: completion.page_id,
                            token: completion.token,
                        },
                    ),
                },
            ))],
        }
    }

    pub(super) fn continue_write_failure(
        &mut self,
        page_id: String,
        token: PageMutationRunToken,
    ) -> Vec<PageMutationEffect> {
        let (baseline, failed_write) = self
            .coordinator
            .writing_replay_input(&page_id, token)
            .expect("failed page mutation must retain its replay input");
        let replay_baseline = Arc::new(
            project_page_write(&baseline, &failed_write)
                .expect("a dispatched failed write must retain a projectable effect"),
        );
        let recovery = self
            .coordinator
            .begin_failed_write_recovery(&page_id, token, baseline, replay_baseline)
            .expect("current failed page mutation must begin recovery");
        vec![self.load_recovery_effect(page_id, recovery)]
    }

    fn finish_successful_write(
        &mut self,
        page_id: String,
        token: PageMutationRunToken,
        mut result: PageMutationResult,
    ) -> Vec<PageMutationEffect> {
        let completed = self
            .coordinator
            .finish_success(&page_id, token)
            .expect("current page mutation completion must retain its write");
        self.coordinator.promote_text_projection(
            &page_id,
            token,
            completed.write.text_projection.clone(),
        );
        let mut usable_authority = result.page.clone();
        self.coordinator
            .overlay_committed_text(&mut usable_authority, Some(token));
        let replay = self.replay_after_success(&page_id, completed, Arc::new(usable_authority));
        let replay = match replay {
            Ok(replay) => replay,
            Err((baseline, error)) => {
                return successful_replay_failure(page_id, baseline, error, result.page, token)
            }
        };
        assert!(self.coordinator.accept_idle_projection(
            &page_id,
            replay.authority.clone(),
            replay.optimistic.clone(),
        ));
        self.apply_successful_replay(page_id, token, &mut result, replay)
    }

    fn replay_after_success(
        &mut self,
        page_id: &str,
        completed: CompletedPageMutationWrite,
        authority: Arc<CardPage>,
    ) -> Result<SuccessfulPageReplay, FailedPageReplay> {
        if !self.coordinator.has_pending_writes(page_id) {
            return Ok(SuccessfulPageReplay {
                authority,
                optimistic: None,
            });
        }
        let expected = Arc::new(
            project_page_write(&completed.baseline, &completed.write)
                .expect("a dispatched page write must remain projectable after success"),
        );
        match self
            .coordinator
            .rebase_queued_page_writes(&expected, &authority)
        {
            Ok(optimistic) => Ok(SuccessfulPageReplay {
                authority,
                optimistic: optimistic.map(Arc::new),
            }),
            Err(error) => Err((expected, error)),
        }
    }

    pub(super) fn continue_successful_replay_failure(
        &mut self,
        mut failure: PageSuccessfulReplayFailure,
    ) -> Vec<PageMutationEffect> {
        let recovery = self
            .coordinator
            .begin_replay_recovery(
                &failure.page_id,
                self.environment.workspace_api.clone(),
                failure.baseline,
            )
            .expect("failed tail replay must enter recovery");
        self.coordinator
            .reconcile_mutation_result(&mut failure.page, failure.token, false);
        let authority = Arc::new(failure.page);
        vec![
            PageMutationEffect::AdvanceAuthority {
                page_id: failure.page_id.clone(),
                authority,
            },
            PageMutationEffect::ScheduleRecovery(PageMutationRecoveryTimer::new(
                failure.page_id,
                recovery,
            )),
            PageMutationEffect::Notify,
        ]
    }

    fn apply_successful_replay(
        &mut self,
        page_id: String,
        token: PageMutationRunToken,
        result: &mut PageMutationResult,
        replay: SuccessfulPageReplay,
    ) -> Vec<PageMutationEffect> {
        let has_pending_replay = replay.optimistic.is_some();
        let visible = self.documents.page_with_id(&page_id).is_some();
        self.coordinator
            .reconcile_mutation_result(&mut result.page, token, visible);
        let mut effects = Vec::new();
        if visible {
            let page = replay
                .optimistic
                .as_ref()
                .map_or_else(|| result.page.clone(), |page| page.as_ref().clone());
            effects.push(PageMutationEffect::ReplaceLoaded(Box::new(
                PageMutationProjectionReplacement {
                    page_id: page_id.clone(),
                    page,
                    authority: replay.authority,
                    clear_history: false,
                },
            )));
        } else {
            self.coordinator.retain_offscreen_result(
                &page_id,
                token,
                replay.authority,
                replay.optimistic,
            );
        }
        effects.push(next_lane_action(page_id, has_pending_replay));
        effects.push(PageMutationEffect::Notify);
        effects
    }

    pub(super) fn load_recovery_effect(
        &self,
        page_id: String,
        token: crate::ui::surface::PageMutationRecoveryToken,
    ) -> PageMutationEffect {
        let workspace_api = self
            .coordinator
            .recovery_api(&page_id, token)
            .expect("page mutation recovery must use its page-bound workspace API");
        PageMutationEffect::RunBackground(
            super::super::action::PageMutationBackgroundJob::LoadRecovery(
                PageMutationRecoveryDispatch {
                    page_id,
                    token,
                    workspace_api,
                },
            ),
        )
    }
}

fn successful_replay_failure(
    page_id: String,
    baseline: Arc<CardPage>,
    error: PageWriteReplayError,
    page: CardPage,
    token: PageMutationRunToken,
) -> Vec<PageMutationEffect> {
    vec![
        PageMutationEffect::PrintError(format!("could not replay queued page edits: {error}")),
        PageMutationEffect::Continue(Box::new(PageMutationAction::Internal(
            PageMutationInternalAction::ContinueSuccessfulReplayRecovery(Box::new(
                PageSuccessfulReplayFailure {
                    page_id,
                    baseline,
                    page,
                    token,
                },
            )),
        ))),
    ]
}

pub(super) fn next_lane_action(page_id: String, has_pending_replay: bool) -> PageMutationEffect {
    let action = if has_pending_replay {
        PageMutationAction::Internal(PageMutationInternalAction::StartPage(page_id))
    } else {
        PageMutationAction::FinishSearchMutation {
            lane_page_id: page_id,
        }
    };
    PageMutationEffect::Continue(Box::new(action))
}
