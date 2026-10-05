use std::sync::Arc;

use gpui::{Bounds, Pixels};

use crate::model::{
    BoardItem, BoardSnapshot, CreateCalendarPageRequest, LoadCalendarItemsRequest,
    LoadCalendarItemsResult, NotionWorkspaceOperationFailure, NotionWorkspaceResult,
    SetCalendarPageDateRangeRequest, SetCalendarPageDateRequest,
};
use crate::ui::surface::DateUndatedDialogSource;
use crate::ui::{CivilDate, ViewTabKind};

#[derive(Clone)]
pub(in crate::ui::board_workspace) struct DateViewContext {
    pub(in crate::ui::board_workspace) active_view_identity: Option<String>,
    pub(in crate::ui::board_workspace) active_view_kind: Option<ViewTabKind>,
    pub(in crate::ui::board_workspace) board_locked: bool,
    pub(in crate::ui::board_workspace) calendar_date_property_id: Option<String>,
    pub(in crate::ui::board_workspace) workspace_available: bool,
}

impl DateViewContext {
    pub(in crate::ui::board_workspace) fn from_board(
        board: &BoardSnapshot,
        workspace_available: bool,
    ) -> Self {
        let active_view = board.active_date_view();
        Self {
            active_view_identity: active_view
                .map(|view| view.provider_view_id.as_str().to_string()),
            active_view_kind: active_view.map(|view| view.kind),
            board_locked: board.is_locked,
            calendar_date_property_id: board
                .calendar_view
                .as_ref()
                .map(|view| view.date_property_id.clone()),
            workspace_available,
        }
    }

    pub(in crate::ui::board_workspace) fn active_identity(&self) -> Option<&str> {
        self.active_view_identity.as_deref()
    }

    pub(in crate::ui::board_workspace) fn calendar_is_active(&self) -> bool {
        self.active_view_kind == Some(ViewTabKind::Calendar)
    }

    pub(in crate::ui::board_workspace) fn timeline_is_active(&self) -> bool {
        self.active_view_kind == Some(ViewTabKind::Timeline)
    }
}

pub(in crate::ui::board_workspace) struct CalendarPageCreation {
    pub(in crate::ui::board_workspace) request: CreateCalendarPageRequest,
    pub(in crate::ui::board_workspace) date_key: String,
    pub(in crate::ui::board_workspace) view_identity: String,
    pub(in crate::ui::board_workspace) session: Arc<()>,
}

pub(in crate::ui::board_workspace) struct CalendarMonthLoad {
    pub(in crate::ui::board_workspace) request: LoadCalendarItemsRequest,
    pub(in crate::ui::board_workspace) token: CalendarMonthLoadToken,
}

pub(in crate::ui::board_workspace) struct CalendarMonthLoadToken {
    pub(in crate::ui::board_workspace) month: CivilDate,
    pub(in crate::ui::board_workspace) generation: u64,
    pub(in crate::ui::board_workspace) view_identity: String,
    pub(in crate::ui::board_workspace) session: Arc<()>,
}

pub(super) enum CalendarMonthLoadCompletion {
    Stale,
    Loaded,
    Failed(NotionWorkspaceOperationFailure),
}

pub(in crate::ui::board_workspace) struct CalendarRangeResizeJob {
    pub(in crate::ui::board_workspace) request: SetCalendarPageDateRangeRequest,
    pub(in crate::ui::board_workspace) block_id: String,
    pub(in crate::ui::board_workspace) session: Arc<()>,
}

pub(in crate::ui::board_workspace) struct CalendarDateAssignmentJob {
    pub(in crate::ui::board_workspace) request: SetCalendarPageDateRequest,
    pub(in crate::ui::board_workspace) block_id: String,
    pub(in crate::ui::board_workspace) session: Arc<()>,
}

pub(in crate::ui::board_workspace) struct DateUndatedQueryJob {
    pub(in crate::ui::board_workspace) request: LoadCalendarItemsRequest,
    pub(in crate::ui::board_workspace) token: DateUndatedQueryToken,
}

pub(in crate::ui::board_workspace) struct DateUndatedQueryToken {
    pub(in crate::ui::board_workspace) view_identity: String,
    pub(in crate::ui::board_workspace) query: String,
    pub(in crate::ui::board_workspace) generation: u64,
    pub(in crate::ui::board_workspace) session: Arc<()>,
    pub(in crate::ui::board_workspace) limit: usize,
}

pub(in crate::ui::board_workspace) struct DateUndatedCountJob {
    pub(in crate::ui::board_workspace) view_identity: String,
    pub(in crate::ui::board_workspace) generation: u64,
    pub(in crate::ui::board_workspace) session: Arc<()>,
}

pub(in crate::ui::board_workspace) struct DateUndatedCountRetry {
    pub(in crate::ui::board_workspace) view_identity: String,
    pub(in crate::ui::board_workspace) failed_generation: u64,
    pub(in crate::ui::board_workspace) retry_attempt: u8,
    pub(in crate::ui::board_workspace) session: Arc<()>,
}

pub(in crate::ui::board_workspace) struct DateUndatedDialogCommand {
    pub(in crate::ui::board_workspace) anchor: Bounds<Pixels>,
    pub(in crate::ui::board_workspace) source: DateUndatedDialogSource,
}

pub(in crate::ui::board_workspace) enum DateViewRequest {
    CalendarMonth(CivilDate),
    CreateCalendarPage(CalendarPageCreation),
    ResizeCalendarRange(CalendarRangeResizeJob),
    AssignCalendarDate(CalendarDateAssignmentJob),
    UndatedItems(DateUndatedQueryJob),
    UndatedCount(DateUndatedCountJob),
    UndatedCountRetry(DateUndatedCountRetry),
}

pub(in crate::ui::board_workspace) enum DateViewEffect {
    Notify,
    OpenExternalCalendar,
    OpenBoardItem(Box<BoardItem>),
    OpenCard(String),
    ClearUndatedCount,
    SetUndatedCount(usize),
    ClearUndatedDialog(Arc<()>),
    ToggleUndatedDialog(DateUndatedDialogCommand),
    CloseHostedUndatedDialog,
    CloseUndatedDialogFromSource,
    OpenUndatedSource(Arc<()>),
    CloseUndatedSource(Arc<()>),
    NotifyDateDragHosts,
    Request(Box<DateViewRequest>),
}

pub(in crate::ui::board_workspace) enum DateViewCompletion {
    CalendarMonth {
        token: CalendarMonthLoadToken,
        result: NotionWorkspaceResult<LoadCalendarItemsResult>,
    },
    CalendarPageCreation {
        creation: CalendarPageCreation,
        result: NotionWorkspaceResult<String>,
    },
    CalendarRangeResize {
        job: CalendarRangeResizeJob,
        result: NotionWorkspaceResult<()>,
    },
    CalendarDateAssignment {
        job: CalendarDateAssignmentJob,
        result: NotionWorkspaceResult<()>,
    },
    UndatedItems {
        token: DateUndatedQueryToken,
        result: NotionWorkspaceResult<LoadCalendarItemsResult>,
    },
    UndatedCount {
        job: DateUndatedCountJob,
        result: NotionWorkspaceResult<usize>,
    },
    UndatedCountRetry(DateUndatedCountRetry),
}

pub(in crate::ui::board_workspace) enum DateViewCompletionOutcome {
    Stale,
    Effects(Vec<DateViewEffect>),
    Failure(Box<DateViewFailure>),
}

pub(in crate::ui::board_workspace) struct DateViewFailure {
    pub(in crate::ui::board_workspace) operation: &'static str,
    pub(in crate::ui::board_workspace) error: NotionWorkspaceOperationFailure,
    pub(in crate::ui::board_workspace) recovery: DateViewFailureRecovery,
}

pub(in crate::ui::board_workspace) enum DateViewFailureRecovery {
    None,
    RollBackMonth(CivilDate),
    Notify,
    RefreshQueries,
    RetryUndatedCount(DateUndatedCountJob),
}
