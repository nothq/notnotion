use std::collections::{HashMap, VecDeque};

use crate::model::CardPage;
use crate::ui::board_workspace::{PageWrite, PageWriteTextProjection};
use crate::ui::{Arc, NotionWorkspaceApi};

mod enqueue;
mod execution;
mod reconciliation;
mod retained_text;

use retained_text::PageCommittedTextOverlays;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PageMutationRunToken {
    pub(crate) epoch: u64,
    pub(crate) write_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PageMutationRecoveryToken {
    pub(crate) epoch: u64,
    pub(crate) generation: u64,
    pub(crate) attempt: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageMutationQueueHead {
    InFlight,
    Pending,
}

impl PageMutationQueueHead {
    pub(crate) fn tail_is_pending(self, queue_len: usize) -> bool {
        matches!(self, Self::Pending) || queue_len > 1
    }
}

pub(crate) struct PageMutationDispatch {
    pub(crate) page_id: String,
    pub(crate) write: PageWrite,
    pub(crate) token: PageMutationRunToken,
    pub(crate) workspace_api: Arc<dyn NotionWorkspaceApi>,
}

pub(crate) struct CompletedPageMutationWrite {
    pub(crate) write: PageWrite,
    pub(crate) baseline: Arc<CardPage>,
}

pub(crate) struct FailedPageMutationWrite {
    pub(crate) write: PageWrite,
    pub(crate) baseline: Arc<CardPage>,
    pub(crate) projected: Arc<CardPage>,
}

pub(crate) struct PageMutationRecoveryReplay {
    pub(crate) baseline: Arc<CardPage>,
    pub(crate) queue: VecDeque<PageWrite>,
    pub(super) token: PageMutationRecoveryToken,
    pub(super) outcome: PageMutationRecoveryOutcome,
}

pub(crate) struct PageMutationRecoveryCompletion {
    pub(crate) replay: PageMutationRecoveryReplay,
    pub(crate) queue: VecDeque<PageWrite>,
    pub(crate) authority: Arc<CardPage>,
    pub(crate) optimistic: Option<Arc<CardPage>>,
    pub(crate) visible: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum PageMutationRecoveryOutcome {
    Applied,
    Pending,
}

#[derive(Default)]
pub(crate) struct PageMutationAuthoritySnapshot {
    tokens_by_page: HashMap<String, PageMutationRunToken>,
}

impl PageMutationAuthoritySnapshot {
    pub(crate) fn token_for(&self, page_id: &str) -> Option<PageMutationRunToken> {
        self.tokens_by_page.get(page_id).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageMutationRecoveryPhase {
    Loading,
    Waiting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageMutationRecoveryCause {
    FailedWrite(PageMutationRunToken),
    ReplayConflict,
}

pub(super) struct PageMutationRecovery {
    pub(super) token: PageMutationRecoveryToken,
    pub(super) phase: PageMutationRecoveryPhase,
    pub(super) cause: PageMutationRecoveryCause,
    pub(super) replay_baseline: Arc<CardPage>,
    pub(super) failed_write: Option<PageWrite>,
    pub(super) failed_write_baseline: Option<Arc<CardPage>>,
}

#[derive(Default)]
pub(super) enum PageMutationExecution {
    #[default]
    Idle,
    Writing(PageMutationRunToken),
    Recovering(Box<PageMutationRecovery>),
}

/// The authoritative page a committed write returned while its page was
/// offscreen, with that write's run token.
type DeferredAuthoritativePage = (PageMutationRunToken, Arc<CardPage>);

#[derive(Default)]
pub(super) struct PageMutationLane {
    pub(super) workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    pub(super) queue: VecDeque<PageWrite>,
    pub(super) execution: PageMutationExecution,
    pub(super) queue_baseline: Option<Arc<CardPage>>,
    committed_text: PageCommittedTextOverlays,
    pub(super) latest_committed_token: Option<PageMutationRunToken>,
    pub(super) deferred_authoritative_page: Option<DeferredAuthoritativePage>,
    pub(super) optimistic_page: Option<Arc<CardPage>>,
}

#[derive(Default)]
pub(crate) struct PageMutationCoordinator {
    pub(super) epoch: u64,
    next_write_id: u64,
    pub(super) next_recovery_generation: u64,
    pub(super) lanes: HashMap<String, PageMutationLane>,
}

impl PageMutationCoordinator {
    pub(crate) fn invalidate_session(&mut self) {
        self.epoch = self
            .epoch
            .checked_add(1)
            .expect("Notion page mutation epoch overflowed");
        self.lanes.clear();
    }

    pub(crate) fn needs_first_write_baseline(&self, page_id: &str) -> bool {
        self.lanes.get(page_id).is_none_or(|lane| {
            lane.queue.is_empty() && matches!(lane.execution, PageMutationExecution::Idle)
        })
    }

    pub(crate) fn pending_page_ids(&self) -> Vec<String> {
        self.lanes
            .iter()
            .filter(|(_, lane)| !lane.queue.is_empty())
            .map(|(page_id, _)| page_id.clone())
            .collect()
    }

    pub(crate) fn authority_snapshot(&self) -> PageMutationAuthoritySnapshot {
        PageMutationAuthoritySnapshot {
            tokens_by_page: self
                .lanes
                .iter()
                .filter_map(|(page_id, lane)| {
                    lane.latest_committed_token
                        .map(|token| (page_id.clone(), token))
                })
                .collect(),
        }
    }

    pub(crate) fn has_pending_writes(&self, page_id: &str) -> bool {
        self.lanes
            .get(page_id)
            .is_some_and(|lane| !lane.queue.is_empty())
    }

    pub(crate) fn pending_writes_mut(&mut self, page_id: &str) -> Option<&mut VecDeque<PageWrite>> {
        self.lanes.get_mut(page_id).map(|lane| &mut lane.queue)
    }

    pub(crate) fn is_idle(&self, page_id: &str) -> bool {
        self.lanes
            .get(page_id)
            .is_none_or(PageMutationLane::is_idle)
    }

    pub(crate) fn is_recovering(&self, page_id: &str) -> bool {
        self.lanes
            .get(page_id)
            .is_some_and(|lane| matches!(lane.execution, PageMutationExecution::Recovering(_)))
    }

    pub(crate) fn has_work(&self, page_id: &str) -> bool {
        !self.is_idle(page_id)
    }

    pub(crate) fn latest_committed_token(&self, page_id: &str) -> Option<PageMutationRunToken> {
        self.lanes.get(page_id)?.latest_committed_token
    }

    pub(crate) fn promote_text_projection(
        &mut self,
        page_id: &str,
        token: PageMutationRunToken,
        projection: PageWriteTextProjection,
    ) {
        if projection.updates.is_empty() && projection.retirements.is_empty() {
            return;
        }
        let lane = self
            .lanes
            .get_mut(page_id)
            .expect("committed page text projection must retain its lane");
        lane.committed_text.promote(token, projection);
    }

    pub(crate) fn overlay_committed_text(
        &mut self,
        page: &mut CardPage,
        authority: Option<PageMutationRunToken>,
    ) {
        if let Some(lane) = self.lanes.get_mut(&page.block_id) {
            lane.committed_text.overlay(page, authority);
        }
    }

    fn next_token(&mut self) -> PageMutationRunToken {
        let token = PageMutationRunToken {
            epoch: self.epoch,
            write_id: self.next_write_id,
        };
        self.next_write_id = self
            .next_write_id
            .checked_add(1)
            .expect("Notion page mutation write ID overflowed");
        token
    }
}

impl PageMutationLane {
    fn bind_workspace_api(&mut self, workspace_api: Option<Arc<dyn NotionWorkspaceApi>>) {
        if self.workspace_api.is_none() {
            self.workspace_api = workspace_api;
        }
    }

    fn bind_first_write_baseline(&mut self, baseline: Option<Arc<CardPage>>) {
        if self.queue.is_empty() && matches!(self.execution, PageMutationExecution::Idle) {
            self.queue_baseline = baseline;
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self.execution, PageMutationExecution::Idle) && self.queue.is_empty()
    }

    fn clear_cancelled_idle_queue_state(&mut self) {
        if !self.is_idle() {
            return;
        }
        self.queue_baseline = None;
        self.optimistic_page = None;
    }

    fn can_remove(&self) -> bool {
        self.is_idle()
            && self.committed_text.is_empty()
            && self.latest_committed_token.is_none()
            && self.deferred_authoritative_page.is_none()
            && self.optimistic_page.is_none()
            && self.queue_baseline.is_none()
    }
}
