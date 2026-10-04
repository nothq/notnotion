use std::collections::VecDeque;
use std::sync::Arc;

use crate::model::{CardPage, CardPageSimpleTableCellIndex, CardPageWritableSimpleTableCell};
use crate::ui::board_workspace::{PageTextProjectionTarget, PageWrite, PageWriteTextProjection};
use crate::ui::surface::{
    FailedPageMutationWrite, PageMutationCoordinator, PageMutationRecoveryCompletion,
    PageMutationRecoveryReplay, PageMutationRecoveryToken,
};
use crate::ui::NotionWorkspaceApi;

use super::super::action::{
    PageMutationAction, PageMutationEffect, PageMutationInternalAction,
    PageMutationProjectionReplacement, PageMutationRecoveryLoadCompletion,
    PageMutationRecoveryTimer, PageMutationWorkspaceFailure,
};
use super::super::rebase::{
    classify_failed_page_write, rebase_page_write_queue, FailedPageWriteDisposition,
    PageWriteReplayError, ReplayPageSide,
};
use super::success::next_lane_action;
use super::PageMutationSession;

struct RecoveredPageProjection {
    page_id: String,
    token: PageMutationRecoveryToken,
    page: CardPage,
    replay: PageMutationRecoveryReplay,
    queue: VecDeque<PageWrite>,
    optimistic: Option<Arc<CardPage>>,
}

impl PageMutationSession<'_> {
    pub(super) fn begin_queued_replay(
        &mut self,
        page_id: String,
        baseline: Arc<CardPage>,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    ) -> Vec<PageMutationEffect> {
        let recovery = self
            .coordinator
            .begin_replay_recovery(&page_id, workspace_api, baseline)
            .expect("queued replay conflict must enter recovery");
        vec![PageMutationEffect::ScheduleRecovery(
            PageMutationRecoveryTimer::new(page_id, recovery),
        )]
    }

    pub(super) fn finish_recovery_delay(
        &mut self,
        page_id: String,
        token: PageMutationRecoveryToken,
    ) -> Vec<PageMutationEffect> {
        if !self.coordinator.resume_recovery(&page_id, token) {
            return Vec::new();
        }
        vec![self.load_recovery_effect(page_id, token)]
    }

    pub(super) fn finish_recovery_load(
        &mut self,
        completion: PageMutationRecoveryLoadCompletion,
    ) -> Vec<PageMutationEffect> {
        if !self
            .coordinator
            .is_recovering_token(&completion.page_id, completion.token)
        {
            return Vec::new();
        }
        match completion.result {
            Ok(page) => self.finish_recovered_page(completion.page_id, completion.token, page),
            Err(failure) => vec![PageMutationEffect::HandleWorkspaceFailure(Box::new(
                PageMutationWorkspaceFailure {
                    operation: "page reload after a save failure failed",
                    failure,
                    continuation: PageMutationAction::Internal(
                        PageMutationInternalAction::ContinueRecoveryFailure {
                            page_id: completion.page_id,
                            token: completion.token,
                        },
                    ),
                },
            ))],
        }
    }

    pub(super) fn defer_recovery(
        &mut self,
        page_id: String,
        token: PageMutationRecoveryToken,
    ) -> Vec<PageMutationEffect> {
        let next = self
            .coordinator
            .defer_recovery(&page_id, token)
            .expect("current page mutation recovery must remain retryable");
        vec![
            PageMutationEffect::ScheduleRecovery(PageMutationRecoveryTimer::new(page_id, next)),
            PageMutationEffect::Notify,
        ]
    }

    fn finish_recovered_page(
        &mut self,
        page_id: String,
        token: PageMutationRecoveryToken,
        mut page: CardPage,
    ) -> Vec<PageMutationEffect> {
        let latest = self.coordinator.latest_committed_token(&page_id);
        self.coordinator.overlay_committed_text(&mut page, latest);
        let failed = self.coordinator.recovery_failed_write(&page_id, token);
        let disposition = match classify_recovered_failed_write(failed.as_ref(), &page) {
            Ok(disposition) => disposition,
            Err(error) => {
                return retry_recovery(
                    format!("could not classify failed page write from fresh authority: {error}"),
                    page_id,
                    token,
                )
            }
        };
        if disposition == Some(FailedPageWriteDisposition::Conflict) {
            return retry_recovery(
                "fresh authority conflicts with the failed page write".to_string(),
                page_id,
                token,
            );
        }
        apply_authority_confirmed_text(&mut page, failed.as_ref(), disposition);
        let replay = self
            .coordinator
            .recovery_replay_plan(&page_id, token, disposition)
            .expect("current recovery must retain an atomic replay plan");
        let (queue, optimistic) = match replay_recovery_queue(&replay, &page) {
            Ok(rebased) => rebased,
            Err(error) => {
                return retry_recovery(
                    format!("could not replay page edits after authoritative recovery: {error}"),
                    page_id,
                    token,
                )
            }
        };
        self.finish_recovered_projection(RecoveredPageProjection {
            page_id,
            token,
            page,
            replay,
            queue,
            optimistic,
        })
    }

    fn finish_recovered_projection(
        &mut self,
        projection: RecoveredPageProjection,
    ) -> Vec<PageMutationEffect> {
        let authority = Arc::new(projection.page);
        let visible = self.documents.page_with_id(&projection.page_id).is_some();
        assert!(self.coordinator.finish_recovery(
            &projection.page_id,
            projection.token,
            PageMutationRecoveryCompletion {
                replay: projection.replay,
                queue: projection.queue,
                authority: authority.clone(),
                optimistic: projection.optimistic.clone(),
                visible,
            },
        ));
        recovered_projection_effects(
            projection.page_id,
            authority,
            projection.optimistic,
            visible,
        )
    }
}

fn recovered_projection_effects(
    page_id: String,
    authority: Arc<CardPage>,
    optimistic: Option<Arc<CardPage>>,
    visible: bool,
) -> Vec<PageMutationEffect> {
    let has_pending_replay = optimistic.is_some();
    let mut effects = Vec::new();
    if visible {
        let page = optimistic.as_ref().unwrap_or(&authority).as_ref().clone();
        effects.push(PageMutationEffect::ReplaceLoaded(Box::new(
            PageMutationProjectionReplacement {
                page_id: page_id.clone(),
                page,
                authority,
                clear_history: optimistic.is_none(),
            },
        )));
    }
    effects.push(next_lane_action(page_id, has_pending_replay));
    effects.push(PageMutationEffect::Notify);
    effects
}

fn retry_recovery(
    message: String,
    page_id: String,
    token: PageMutationRecoveryToken,
) -> Vec<PageMutationEffect> {
    vec![
        PageMutationEffect::PrintError(message),
        PageMutationEffect::Continue(Box::new(PageMutationAction::Internal(
            PageMutationInternalAction::DeferRecovery { page_id, token },
        ))),
    ]
}

fn apply_authority_confirmed_text(
    page: &mut CardPage,
    failed: Option<&FailedPageMutationWrite>,
    disposition: Option<FailedPageWriteDisposition>,
) {
    if disposition != Some(FailedPageWriteDisposition::Applied) {
        return;
    }
    apply_text_projection(
        page,
        &failed
            .expect("applied failed write must retain its envelope")
            .write
            .text_projection,
    )
    .expect("an authority-confirmed write must retain valid projected text targets");
}

fn classify_recovered_failed_write(
    failed: Option<&FailedPageMutationWrite>,
    authority: &CardPage,
) -> Result<Option<FailedPageWriteDisposition>, PageWriteReplayError> {
    failed
        .map(|failed| {
            classify_failed_page_write(
                &failed.write,
                &failed.baseline,
                &failed.projected,
                authority,
            )
        })
        .transpose()
}

/// The rebased recovery queue and its optimistic page, if any writes remain.
type ReplayedRecoveryQueue = (VecDeque<PageWrite>, Option<Arc<CardPage>>);

fn replay_recovery_queue(
    replay: &PageMutationRecoveryReplay,
    authority: &CardPage,
) -> Result<ReplayedRecoveryQueue, PageWriteReplayError> {
    if replay.queue.is_empty() {
        return Ok((replay.queue.clone(), None));
    }
    let rebased = rebase_page_write_queue(&replay.baseline, authority, &replay.queue)?;
    Ok((rebased.queue, Some(Arc::new(rebased.optimistic_page))))
}

fn apply_text_projection(
    page: &mut CardPage,
    projection: &PageWriteTextProjection,
) -> Result<(), PageWriteReplayError> {
    let cell_index = projection
        .updates
        .iter()
        .any(|update| matches!(&update.target, PageTextProjectionTarget::SimpleTableCell(_)))
        .then(|| CardPageSimpleTableCellIndex::new(page))
        .transpose()
        .map_err(|detail| {
            PageWriteReplayError::invalid_request("failed write retained text", detail)
        })?;
    for update in &projection.updates {
        apply_projected_text(page, cell_index.as_ref(), update)?;
    }
    Ok(())
}

fn apply_projected_text(
    page: &mut CardPage,
    cell_index: Option<&CardPageSimpleTableCellIndex>,
    update: &crate::ui::board_workspace::PageProjectedText,
) -> Result<(), PageWriteReplayError> {
    match &update.target {
        PageTextProjectionTarget::Title => page.title.clone_from(&update.text),
        PageTextProjectionTarget::Block(block_id) => {
            let block = page
                .blocks
                .iter_mut()
                .find(|block| block.block_id == *block_id)
                .ok_or_else(|| PageWriteReplayError::MissingBlock {
                    operation: "failed write retained text",
                    block_id: block_id.clone(),
                    side: ReplayPageSide::Authority,
                })?;
            let editable = block.editable_content_mut().ok_or_else(|| {
                PageWriteReplayError::InvalidTextTarget {
                    operation: "failed write retained text",
                    block_id: block_id.clone(),
                    side: ReplayPageSide::Authority,
                }
            })?;
            editable.text.clone_from(&update.text);
            editable.annotations.clone_from(&update.annotations);
        }
        PageTextProjectionTarget::SimpleTableCell(target) => {
            let cell = CardPageWritableSimpleTableCell::new(
                update.text.clone(),
                update.annotations.clone(),
            )
            .map_err(|detail| {
                PageWriteReplayError::invalid_request("failed write retained text", detail)
            })?;
            cell_index
                .expect("cell text projection must build a table index")
                .replace(page, target, cell)
                .map_err(|detail| {
                    PageWriteReplayError::invalid_request("failed write retained text", detail)
                })?;
        }
    }
    Ok(())
}

impl PageMutationCoordinator {
    fn recovery_replay_plan(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
        disposition: Option<FailedPageWriteDisposition>,
    ) -> Option<PageMutationRecoveryReplay> {
        match disposition {
            Some(FailedPageWriteDisposition::Applied) => self.plan_applied_recovery(page_id, token),
            Some(
                FailedPageWriteDisposition::NotApplied | FailedPageWriteDisposition::RetrySafe,
            ) => self.plan_not_applied_recovery(page_id, token),
            Some(FailedPageWriteDisposition::Conflict) => None,
            None => self.plan_replay_recovery(page_id, token),
        }
    }
}
