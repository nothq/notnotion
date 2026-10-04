use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use crate::model::{
    CardPage, NotionWorkspaceOperationFailure, NotionWorkspaceResult, PageMutationResult,
};
use crate::ui::surface::{PageMutationDispatch, PageMutationRecoveryToken, PageMutationRunToken};
use crate::ui::NotionWorkspaceApi;

use super::super::plan::PageMutationPlan;

const PAGE_MUTATION_RECOVERY_BASE_DELAY: Duration = Duration::from_secs(1);
const PAGE_MUTATION_RECOVERY_MAX_DELAY: Duration = Duration::from_secs(30);

pub(crate) enum PageMutationAction {
    ApplyPlan(PageMutationPlan),
    CaptureVisibleProjection,
    RestoreVisibleProjection,
    StartPending,
    BeginQueuedReplay {
        page_id: String,
        baseline: Arc<CardPage>,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    },
    BeginSearchMutation {
        lane_page_id: String,
        block_id: String,
    },
    PersistSearchMutation {
        lane_page_id: String,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
    },
    FinishSearchMutation {
        lane_page_id: String,
    },
    Internal(PageMutationInternalAction),
}

pub(crate) enum PageMutationInternalAction {
    StartPage(String),
    StartPages(VecDeque<String>),
    WriteCompleted(Box<PageMutationWriteCompletion>),
    ContinueWriteFailure {
        page_id: String,
        token: PageMutationRunToken,
    },
    ContinueSuccessfulReplayRecovery(Box<PageSuccessfulReplayFailure>),
    RecoveryDelayElapsed {
        page_id: String,
        token: PageMutationRecoveryToken,
    },
    RecoveryLoaded(Box<PageMutationRecoveryLoadCompletion>),
    ContinueRecoveryFailure {
        page_id: String,
        token: PageMutationRecoveryToken,
    },
    DeferRecovery {
        page_id: String,
        token: PageMutationRecoveryToken,
    },
}

pub(crate) struct PageMutationWriteCompletion {
    pub(super) page_id: String,
    pub(super) token: PageMutationRunToken,
    pub(super) result: NotionWorkspaceResult<PageMutationResult>,
}

pub(crate) struct PageMutationRecoveryLoadCompletion {
    pub(super) page_id: String,
    pub(super) token: PageMutationRecoveryToken,
    pub(super) result: NotionWorkspaceResult<CardPage>,
}

pub(crate) struct PageSuccessfulReplayFailure {
    pub(super) page_id: String,
    pub(super) baseline: Arc<CardPage>,
    pub(super) page: CardPage,
    pub(super) token: PageMutationRunToken,
}

pub(super) enum PageMutationEffect {
    RunBackground(PageMutationBackgroundJob),
    ScheduleRecovery(PageMutationRecoveryTimer),
    HandleWorkspaceFailure(Box<PageMutationWorkspaceFailure>),
    ReplaceLoaded(Box<PageMutationProjectionReplacement>),
    AdvanceAuthority {
        page_id: String,
        authority: Arc<CardPage>,
    },
    RefreshSearch,
    PrintError(String),
    Continue(Box<PageMutationAction>),
    Notify,
}

pub(super) enum PageMutationBackgroundJob {
    Write(Box<PageMutationDispatch>),
    LoadRecovery(PageMutationRecoveryDispatch),
}

pub(super) struct PageMutationRecoveryDispatch {
    pub(super) page_id: String,
    pub(super) token: PageMutationRecoveryToken,
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
}

pub(super) struct PageMutationRecoveryTimer {
    pub(super) page_id: String,
    pub(super) token: PageMutationRecoveryToken,
    pub(super) delay: Duration,
}

pub(super) struct PageMutationWorkspaceFailure {
    pub(super) operation: &'static str,
    pub(super) failure: NotionWorkspaceOperationFailure,
    pub(super) continuation: PageMutationAction,
}

pub(super) struct PageMutationProjectionReplacement {
    pub(super) page_id: String,
    pub(super) page: CardPage,
    pub(super) authority: Arc<CardPage>,
    pub(super) clear_history: bool,
}

pub(super) enum PageMutationHostStep {
    Action(PageMutationAction),
    Effect(PageMutationEffect),
}

pub(super) struct PageMutationHostQueue {
    pending: VecDeque<PageMutationHostStep>,
}

impl PageMutationHostQueue {
    pub(super) fn new(action: PageMutationAction) -> Self {
        Self {
            pending: VecDeque::from([PageMutationHostStep::Action(action)]),
        }
    }

    pub(super) fn pop(&mut self) -> Option<PageMutationHostStep> {
        self.pending.pop_front()
    }

    pub(super) fn prepend_effects(&mut self, effects: Vec<PageMutationEffect>) {
        for effect in effects.into_iter().rev() {
            self.pending
                .push_front(PageMutationHostStep::Effect(effect));
        }
    }

    pub(super) fn prepend_action(&mut self, action: PageMutationAction) {
        self.pending
            .push_front(PageMutationHostStep::Action(action));
    }
}

impl PageMutationRecoveryTimer {
    pub(super) fn new(page_id: String, token: PageMutationRecoveryToken) -> Self {
        let exponent = token.attempt.saturating_sub(1).min(5);
        let delay = PAGE_MUTATION_RECOVERY_BASE_DELAY
            .checked_mul(1u32 << exponent)
            .unwrap_or(PAGE_MUTATION_RECOVERY_MAX_DELAY)
            .min(PAGE_MUTATION_RECOVERY_MAX_DELAY);
        Self {
            page_id,
            token,
            delay,
        }
    }

    pub(super) fn into_action(self) -> PageMutationAction {
        PageMutationAction::Internal(PageMutationInternalAction::RecoveryDelayElapsed {
            page_id: self.page_id,
            token: self.token,
        })
    }
}
