use crate::model::{BoardItem, LoadCalendarItemsRequest, SetCalendarPageDateRequest};
use crate::ui::board_workspace::PageShellIconRenderer;
use crate::ui::surface::{
    DateUndatedDialogSource, DateUndatedDialogState, DateUndatedItemsState, DateUndatedRow,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{alpha, point, px, rgb, AppearanceMode, Arc, BoxShadow, Theme};
use gpui::{Bounds, Pixels};

use super::super::calendar_view::{
    calendar_query_date, CalendarDateAssignmentJob, CalendarPageDrag, DateUndatedCountJob,
    DateUndatedCountRetry, DateUndatedDialogCommand, DateUndatedQueryJob, DateUndatedQueryToken,
    DateViewContext, DateViewEffect, DateViewRequest,
};

mod controller;
mod dialog;
mod host;
mod layout;
mod query;
mod render;
mod row;
mod state;

const DATE_UNDATED_DIALOG_WIDTH: f32 = 320.0;
const DATE_UNDATED_DIALOG_MAX_HEIGHT: f32 = 665.0;
const DATE_UNDATED_DIALOG_MARGIN: f32 = 12.0;
const DATE_UNDATED_SEARCH_HEADER_HEIGHT: f32 = 48.0;
const DATE_UNDATED_INSTRUCTION_HEIGHT: f32 = 33.4;
const DATE_UNDATED_ROW_HEIGHT: f32 = 28.0;
const DATE_UNDATED_ROW_GAP: f32 = 1.0;
const DATE_UNDATED_VISIBLE_ROW_COUNT: usize = 20;
const DATE_UNDATED_PAGE_SIZE: usize = 20;
const DATE_UNDATED_MAX_LIMIT: usize = 200;
const DATE_UNDATED_BOTTOM_PADDING: f32 = 8.0;
const DATE_UNDATED_COUNT_MAX_RETRIES: u8 = 2;

impl DateUndatedDialogSource {
    fn session(&self) -> &Arc<()> {
        match self {
            Self::Full { session } | Self::Inline { session, .. } => session,
        }
    }

    fn matches_session(&self, session: &Arc<()>) -> bool {
        Arc::ptr_eq(self.session(), session)
    }
}

#[derive(Clone)]
enum DateUndatedAction {
    Dismiss,
    SearchChanged(String),
    ArmPagination,
    LoadMore,
    Assign(BoardItem),
    Open(BoardItem),
    DragStarted,
}

enum DateUndatedEffect {
    None,
    Notify,
    BeginQuery { limit: usize, preserve_rows: bool },
    BeginAssignment(BoardItem),
    Close,
    CloseAndOpen(BoardItem),
    NotifyDragHosts,
}

#[derive(Clone)]
struct DateUndatedRowView {
    row: DateUndatedRow,
    assignment_pending: bool,
}

struct DateUndatedDialogView<'a> {
    state: &'a DateUndatedItemsState,
    anchor: Bounds<Pixels>,
    viewport_width: f32,
    viewport_height: f32,
    instruction: &'static str,
    theme: Theme,
    appearance_mode: AppearanceMode,
    page_icons: PageShellIconRenderer,
    drag_enabled: bool,
    actions: ViewActionSink<DateUndatedAction>,
}

#[derive(Clone)]
struct DateUndatedRenderer {
    theme: Theme,
    appearance_mode: AppearanceMode,
    page_icons: PageShellIconRenderer,
    drag_enabled: bool,
    actions: ViewActionSink<DateUndatedAction>,
}

#[derive(Clone, Copy)]
struct DateUndatedDialogLayout {
    left: f32,
    top: f32,
    height: f32,
    list_height: f32,
}
