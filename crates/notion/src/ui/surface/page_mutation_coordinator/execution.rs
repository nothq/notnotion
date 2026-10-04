use crate::model::CardPage;
use crate::ui::{Arc, NotionWorkspaceApi};

use super::{
    CompletedPageMutationWrite, FailedPageMutationWrite, PageMutationCoordinator,
    PageMutationDispatch, PageMutationExecution, PageMutationRecovery, PageMutationRecoveryCause,
    PageMutationRecoveryCompletion, PageMutationRecoveryOutcome, PageMutationRecoveryPhase,
    PageMutationRecoveryReplay, PageMutationRecoveryToken, PageMutationRunToken,
};

/// The queue baseline and the write in flight, for replaying that write.
type PageWriteReplayInput = (Arc<CardPage>, crate::ui::board_workspace::PageWrite);

impl PageMutationCoordinator {
    pub(crate) fn begin_next(
        &mut self,
        page_id: &str,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    ) -> Option<PageMutationDispatch> {
        let lane = self.lanes.get_mut(page_id)?;
        lane.bind_workspace_api(workspace_api);
        if !matches!(lane.execution, PageMutationExecution::Idle) {
            return None;
        }
        let write = lane.queue.front()?.clone();
        assert!(
            lane.queue_baseline.is_some(),
            "first page write must transfer its loaded authority into the lane"
        );
        let workspace_api = lane.workspace_api.clone()?;
        let token = self.next_token();
        self.lanes
            .get_mut(page_id)
            .expect("page mutation lane must remain present")
            .execution = PageMutationExecution::Writing(token);
        Some(PageMutationDispatch {
            page_id: page_id.to_string(),
            write,
            token,
            workspace_api,
        })
    }

    pub(crate) fn finish_success(
        &mut self,
        page_id: &str,
        token: PageMutationRunToken,
    ) -> Option<CompletedPageMutationWrite> {
        let lane = self.lanes.get_mut(page_id)?;
        if !matches!(lane.execution, PageMutationExecution::Writing(current) if current == token) {
            return None;
        }
        lane.execution = PageMutationExecution::Idle;
        let write = lane
            .queue
            .pop_front()
            .expect("successful page mutation must retain its in-flight write");
        let baseline = lane
            .queue_baseline
            .clone()
            .expect("dispatched page mutation must retain its queue baseline");
        lane.latest_committed_token = Some(token);
        Some(CompletedPageMutationWrite { write, baseline })
    }

    pub(crate) fn is_writing(&self, page_id: &str, token: PageMutationRunToken) -> bool {
        self.lanes.get(page_id).is_some_and(|lane| {
            matches!(lane.execution, PageMutationExecution::Writing(current) if current == token)
        })
    }

    pub(crate) fn writing_replay_input(
        &self,
        page_id: &str,
        token: PageMutationRunToken,
    ) -> Option<PageWriteReplayInput> {
        let lane = self.lanes.get(page_id)?;
        if !matches!(lane.execution, PageMutationExecution::Writing(current) if current == token) {
            return None;
        }
        Some((lane.queue_baseline.clone()?, lane.queue.front()?.clone()))
    }

    pub(crate) fn begin_failed_write_recovery(
        &mut self,
        page_id: &str,
        token: PageMutationRunToken,
        failed_write_baseline: Arc<CardPage>,
        replay_baseline: Arc<CardPage>,
    ) -> Option<PageMutationRecoveryToken> {
        let lane = self.lanes.get(page_id)?;
        if !matches!(lane.execution, PageMutationExecution::Writing(current) if current == token) {
            return None;
        }
        assert_eq!(page_id, replay_baseline.block_id);
        let recovery_token = self.next_recovery_token(0);
        let lane = self
            .lanes
            .get_mut(page_id)
            .expect("failed page mutation lane must remain present");
        let failed_write = lane
            .queue
            .pop_front()
            .expect("failed page mutation must retain its in-flight write");
        lane.execution = PageMutationExecution::Recovering(Box::new(PageMutationRecovery {
            token: recovery_token,
            phase: PageMutationRecoveryPhase::Loading,
            cause: PageMutationRecoveryCause::FailedWrite(token),
            replay_baseline,
            failed_write: Some(failed_write),
            failed_write_baseline: Some(failed_write_baseline),
        }));
        Some(recovery_token)
    }

    pub(crate) fn begin_replay_recovery(
        &mut self,
        page_id: &str,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
        replay_baseline: Arc<CardPage>,
    ) -> Option<PageMutationRecoveryToken> {
        let lane = self.lanes.get_mut(page_id)?;
        lane.bind_workspace_api(workspace_api);
        if !matches!(lane.execution, PageMutationExecution::Idle) || lane.queue.is_empty() {
            return None;
        }
        assert_eq!(page_id, replay_baseline.block_id);
        let token = self.next_recovery_token(1);
        self.lanes
            .get_mut(page_id)
            .expect("replay recovery lane must remain present")
            .execution = PageMutationExecution::Recovering(Box::new(PageMutationRecovery {
            token,
            phase: PageMutationRecoveryPhase::Waiting,
            cause: PageMutationRecoveryCause::ReplayConflict,
            replay_baseline,
            failed_write: None,
            failed_write_baseline: None,
        }));
        Some(token)
    }

    pub(crate) fn recovery_api(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> Option<Arc<dyn NotionWorkspaceApi>> {
        let recovery = self.recovery(page_id, token, PageMutationRecoveryPhase::Loading)?;
        let lane = self.lanes.get(page_id)?;
        let _cause = recovery.cause;
        lane.workspace_api.clone()
    }

    pub(crate) fn plan_applied_recovery(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> Option<PageMutationRecoveryReplay> {
        let recovery = self.recovery(page_id, token, PageMutationRecoveryPhase::Loading)?;
        recovery.failed_write.as_ref()?;
        Some(PageMutationRecoveryReplay {
            baseline: recovery.replay_baseline.clone(),
            queue: self.lanes.get(page_id)?.queue.clone(),
            token,
            outcome: PageMutationRecoveryOutcome::Applied,
        })
    }

    pub(crate) fn plan_not_applied_recovery(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> Option<PageMutationRecoveryReplay> {
        let recovery = self.recovery(page_id, token, PageMutationRecoveryPhase::Loading)?;
        let mut queue = self.lanes.get(page_id)?.queue.clone();
        queue.push_front(recovery.failed_write.clone()?);
        Some(PageMutationRecoveryReplay {
            baseline: recovery.failed_write_baseline.clone()?,
            queue,
            token,
            outcome: PageMutationRecoveryOutcome::Pending,
        })
    }

    pub(crate) fn plan_replay_recovery(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> Option<PageMutationRecoveryReplay> {
        let recovery = self.recovery(page_id, token, PageMutationRecoveryPhase::Loading)?;
        if recovery.failed_write.is_some() {
            return None;
        }
        Some(PageMutationRecoveryReplay {
            baseline: recovery.replay_baseline.clone(),
            queue: self.lanes.get(page_id)?.queue.clone(),
            token,
            outcome: PageMutationRecoveryOutcome::Pending,
        })
    }

    pub(crate) fn recovery_failed_write(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> Option<FailedPageMutationWrite> {
        let recovery = self.recovery(page_id, token, PageMutationRecoveryPhase::Loading)?;
        Some(FailedPageMutationWrite {
            write: recovery.failed_write.clone()?,
            baseline: recovery.failed_write_baseline.clone()?,
            projected: recovery.replay_baseline.clone(),
        })
    }

    pub(crate) fn defer_recovery(
        &mut self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> Option<PageMutationRecoveryToken> {
        self.recovery(page_id, token, PageMutationRecoveryPhase::Loading)?;
        let next = PageMutationRecoveryToken {
            attempt: token.attempt.saturating_add(1),
            ..token
        };
        let PageMutationExecution::Recovering(recovery) = &mut self
            .lanes
            .get_mut(page_id)
            .expect("deferred recovery lane must remain present")
            .execution
        else {
            return None;
        };
        recovery.token = next;
        recovery.phase = PageMutationRecoveryPhase::Waiting;
        Some(next)
    }

    pub(crate) fn resume_recovery(
        &mut self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> bool {
        if self
            .recovery(page_id, token, PageMutationRecoveryPhase::Waiting)
            .is_none()
        {
            return false;
        }
        let PageMutationExecution::Recovering(recovery) = &mut self
            .lanes
            .get_mut(page_id)
            .expect("resumed recovery lane must remain present")
            .execution
        else {
            return false;
        };
        recovery.phase = PageMutationRecoveryPhase::Loading;
        true
    }

    pub(crate) fn finish_recovery(
        &mut self,
        page_id: &str,
        token: PageMutationRecoveryToken,
        completion: PageMutationRecoveryCompletion,
    ) -> bool {
        let PageMutationRecoveryCompletion {
            replay,
            queue,
            authority,
            optimistic,
            visible,
        } = completion;
        if replay.token != token
            || self
                .recovery(page_id, token, PageMutationRecoveryPhase::Loading)
                .is_none()
        {
            return false;
        }
        assert_eq!(page_id, authority.block_id);
        assert_eq!(queue.is_empty(), optimistic.is_none());
        let lane = self
            .lanes
            .get_mut(page_id)
            .expect("finished recovery lane must remain present");
        let committed = (replay.outcome == PageMutationRecoveryOutcome::Applied)
            .then(|| applied_recovery_commit(lane));
        lane.queue = queue;
        lane.execution = PageMutationExecution::Idle;
        if let Some((run_token, write)) = committed {
            lane.committed_text
                .promote(run_token, write.text_projection);
            lane.latest_committed_token = Some(run_token);
        }
        lane.queue_baseline = (!lane.queue.is_empty()).then(|| authority.clone());
        lane.optimistic_page = optimistic;
        lane.deferred_authoritative_page = (!visible)
            .then(|| {
                lane.latest_committed_token
                    .map(|run_token| (run_token, authority))
            })
            .flatten();
        true
    }

    pub(crate) fn accept_idle_projection(
        &mut self,
        page_id: &str,
        authority: Arc<CardPage>,
        optimistic: Option<Arc<CardPage>>,
    ) -> bool {
        let Some(lane) = self.lanes.get(page_id) else {
            return false;
        };
        if !matches!(lane.execution, PageMutationExecution::Idle) {
            return false;
        }
        self.accept_projection(page_id, authority, optimistic)
    }

    pub(crate) fn is_recovering_token(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
    ) -> bool {
        self.lanes.get(page_id).is_some_and(|lane| {
            matches!(
                &lane.execution,
                PageMutationExecution::Recovering(recovery) if recovery.token == token
            )
        })
    }

    fn accept_projection(
        &mut self,
        page_id: &str,
        authority: Arc<CardPage>,
        optimistic: Option<Arc<CardPage>>,
    ) -> bool {
        let Some(lane) = self.lanes.get_mut(page_id) else {
            return false;
        };
        assert_eq!(page_id, authority.block_id);
        assert_eq!(lane.queue.is_empty(), optimistic.is_none());
        lane.execution = PageMutationExecution::Idle;
        lane.queue_baseline = (!lane.queue.is_empty()).then(|| authority.clone());
        lane.optimistic_page = optimistic;
        true
    }

    fn recovery(
        &self,
        page_id: &str,
        token: PageMutationRecoveryToken,
        phase: PageMutationRecoveryPhase,
    ) -> Option<&PageMutationRecovery> {
        let lane = self.lanes.get(page_id)?;
        let PageMutationExecution::Recovering(recovery) = &lane.execution else {
            return None;
        };
        (recovery.token == token && recovery.phase == phase).then_some(recovery.as_ref())
    }

    fn next_recovery_token(&mut self, attempt: u32) -> PageMutationRecoveryToken {
        let token = PageMutationRecoveryToken {
            epoch: self.epoch,
            generation: self.next_recovery_generation,
            attempt,
        };
        self.next_recovery_generation = self
            .next_recovery_generation
            .checked_add(1)
            .expect("Notion page mutation recovery generation overflowed");
        token
    }
}

fn applied_recovery_commit(
    lane: &super::PageMutationLane,
) -> (PageMutationRunToken, crate::ui::board_workspace::PageWrite) {
    let PageMutationExecution::Recovering(recovery) = &lane.execution else {
        panic!("applied recovery commit must retain its recovery state");
    };
    let PageMutationRecoveryCause::FailedWrite(run_token) = recovery.cause else {
        panic!("applied recovery commit must originate from a failed write");
    };
    let write = recovery
        .failed_write
        .clone()
        .expect("applied recovery commit must retain its failed write");
    (run_token, write)
}
