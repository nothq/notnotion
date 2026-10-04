use std::{cell::Cell, rc::Rc};

use crate::model::{
    BoardDateValue, BoardItem, CalendarViewConfig, CreateCalendarPageRequest,
    LoadCalendarItemsRequest, SetCalendarPageDateRangeRequest,
};
use crate::ui::board_workspace::inline_database::INLINE_DATABASE_LEFT_OVERFLOW;
use crate::ui::surface::CalendarRangeResizePreview;
use crate::ui::{
    add_days, alpha, civil_date_month_start, div, img, local_today_civil_date, month_name, point,
    px, rgb, rgba, AnyElement, BoxShadow, CivilDate, Context, Div, FontWeight, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState, Theme, Window, BOARD_VIEWPORT_RIGHT_GUTTER,
    BOARD_VIEWPORT_X, CALENDAR_DAY_BOTTOM_INSET, CALENDAR_DAY_CELL_HEIGHT, CALENDAR_DAY_COUNT,
    CALENDAR_DAY_EVENT_TOP_INSET, CALENDAR_EVENT_ROW_PITCH, CALENDAR_HEADER_BOTTOM_MARGIN,
    CALENDAR_HEADER_HEIGHT, CALENDAR_MIN_ACTIVE_VIEW_HEIGHT, CALENDAR_WEEKDAY_HEADER_HEIGHT,
    CALENDAR_WEEK_COUNT, TIMELINE_TODAY_COLOR,
};
use gpui::prelude::FluentBuilder;
use gpui::{AppContext, ClickEvent, Render, Role, SharedString};

mod controls;
mod day;
mod drag;
mod layout;
mod query;
mod render;
mod segment;

pub(in crate::ui::board_workspace) use query::{
    CalendarDateAssignmentJob, DateUndatedCountJob, DateUndatedCountRetry,
    DateUndatedDialogCommand, DateUndatedQueryJob, DateUndatedQueryToken, DateViewContext,
    DateViewEffect, DateViewRequest,
};

enum CalendarAction {
    OpenExternalCalendar,
    ShowPreviousMonth,
    ShowToday,
    ShowNextMonth,
    CreatePage(CivilDate),
    AssignPageDate {
        item: BoardItem,
        date: CivilDate,
    },
    OpenItem(BoardItem),
    DragStarted,
    BeginRangeResize(CalendarRangeResizeDrag),
    PreviewRangeResize {
        drag: CalendarRangeResizeDrag,
        target_date: CivilDate,
    },
    FinishRangeResize(CalendarRangeResizeDrag),
}

pub(super) struct CalendarLayout<'a> {
    month_start: CivilDate,
    weeks: Vec<CalendarWeek<'a>>,
}

struct CalendarWeek<'a> {
    height: f32,
    days: Vec<CalendarDay>,
    segments: Vec<CalendarWeekSegment<'a>>,
}

struct CalendarDay {
    date: CivilDate,
}

struct CalendarDatedItem<'a> {
    item: &'a BoardItem,
    start: CivilDate,
    end: CivilDate,
}

struct CalendarWeekSegment<'a> {
    item: &'a BoardItem,
    inclusive_start: CivilDate,
    inclusive_end: CivilDate,
    start_column: usize,
    column_count: usize,
    lane: usize,
    continues_from_previous_week: bool,
    continues_into_next_week: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CalendarRangeResizeEndpoint {
    Start,
    End,
}

#[derive(Clone)]
struct CalendarRangeResizeDrag {
    item: BoardItem,
    endpoint: CalendarRangeResizeEndpoint,
    inclusive_start: CivilDate,
    inclusive_end: CivilDate,
    target_date: Rc<Cell<CivilDate>>,
}

impl CalendarRangeResizeDrag {
    fn new(segment: &CalendarWeekSegment<'_>, endpoint: CalendarRangeResizeEndpoint) -> Self {
        let target_date = match endpoint {
            CalendarRangeResizeEndpoint::Start => segment.inclusive_start,
            CalendarRangeResizeEndpoint::End => segment.inclusive_end,
        };
        Self {
            item: segment.item.clone(),
            endpoint,
            inclusive_start: segment.inclusive_start,
            inclusive_end: segment.inclusive_end,
            target_date: Rc::new(Cell::new(target_date)),
        }
    }

    fn resized_range(&self, target_date: CivilDate) -> (CivilDate, CivilDate) {
        match self.endpoint {
            CalendarRangeResizeEndpoint::Start => (
                civil_date_min(target_date, self.inclusive_end),
                self.inclusive_end,
            ),
            CalendarRangeResizeEndpoint::End => (
                self.inclusive_start,
                civil_date_max(target_date, self.inclusive_start),
            ),
        }
    }
}

impl Render for CalendarRangeResizeDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size(px(1.0)).opacity(0.0)
    }
}

impl CalendarLayout<'_> {
    fn grid_height(&self) -> f32 {
        self.weeks.iter().map(|week| week.height).sum()
    }

    pub(super) fn active_view_height(&self) -> f32 {
        (CALENDAR_HEADER_HEIGHT
            + CALENDAR_HEADER_BOTTOM_MARGIN
            + CALENDAR_WEEKDAY_HEADER_HEIGHT
            + 1.0
            + self.grid_height())
        .max(CALENDAR_MIN_ACTIVE_VIEW_HEIGHT)
    }
}

#[derive(Clone)]
pub(crate) struct CalendarPageDrag {
    pub(crate) item: BoardItem,
    title: SharedString,
    theme: Theme,
}

impl CalendarPageDrag {
    pub(crate) fn new(item: &BoardItem, theme: Theme) -> Self {
        Self {
            item: item.clone(),
            title: item.title.clone().into(),
            theme,
        }
    }
}

impl Render for CalendarPageDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(200.0))
            .h(px(28.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.14))
            .bg(rgb(self.theme.elevated_surface_bg))
            .px(px(8.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .child(self.title.clone())
    }
}

fn calendar_week_height(event_row_count: usize) -> f32 {
    (CALENDAR_DAY_EVENT_TOP_INSET
        + event_row_count as f32 * CALENDAR_EVENT_ROW_PITCH
        + CALENDAR_DAY_BOTTOM_INSET)
        .max(CALENDAR_DAY_CELL_HEIGHT)
}

fn build_calendar_week_segments<'a>(
    dated_items: &[CalendarDatedItem<'a>],
    week_start: CivilDate,
    week_end: CivilDate,
) -> (Vec<CalendarWeekSegment<'a>>, usize) {
    let week_start_ordinal = week_start.ordinal();
    let week_end_ordinal = week_end.ordinal();
    let mut occupied_lanes = Vec::<[bool; 7]>::new();
    let mut segments = Vec::new();
    for dated_item in dated_items {
        let start_ordinal = dated_item.start.ordinal();
        let end_ordinal = dated_item.end.ordinal();
        if start_ordinal > week_end_ordinal || end_ordinal < week_start_ordinal {
            continue;
        }
        let clipped_start = start_ordinal.max(week_start_ordinal);
        let clipped_end = end_ordinal.min(week_end_ordinal);
        let start_column = (clipped_start - week_start_ordinal) as usize;
        let column_count = (clipped_end - clipped_start + 1) as usize;
        let lane = occupied_lanes
            .iter()
            .position(|occupied| {
                occupied[start_column..start_column + column_count]
                    .iter()
                    .all(|occupied| !occupied)
            })
            .unwrap_or_else(|| {
                occupied_lanes.push([false; 7]);
                occupied_lanes.len() - 1
            });
        occupied_lanes[lane][start_column..start_column + column_count].fill(true);
        segments.push(CalendarWeekSegment {
            item: dated_item.item,
            inclusive_start: dated_item.start,
            inclusive_end: dated_item.end,
            start_column,
            column_count,
            lane,
            continues_from_previous_week: start_ordinal < week_start_ordinal,
            continues_into_next_week: end_ordinal > week_end_ordinal,
        });
    }
    (segments, occupied_lanes.len())
}

fn calendar_grid_start(month: CivilDate) -> CivilDate {
    let month_start = civil_date_month_start(month);
    let sunday_index = (month_start.ordinal() + 4).rem_euclid(7);
    add_days(month_start, -sunday_index)
}

fn civil_date_min(left: CivilDate, right: CivilDate) -> CivilDate {
    if left.ordinal() <= right.ordinal() {
        left
    } else {
        right
    }
}

fn civil_date_max(left: CivilDate, right: CivilDate) -> CivilDate {
    if left.ordinal() >= right.ordinal() {
        left
    } else {
        right
    }
}

pub(crate) fn calendar_query_date(date: CivilDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn calendar_day_is_weekend(date: CivilDate) -> bool {
    matches!((date.ordinal() + 4).rem_euclid(7), 0 | 6)
}

fn calendar_date_range_for_item<'a>(
    item: &'a BoardItem,
    calendar_view: &CalendarViewConfig,
) -> Option<&'a BoardDateValue> {
    item.properties
        .iter()
        .find(|property| property.property_id == calendar_view.date_property_id)
        .and_then(|property| property.date.as_ref())
}

fn previous_month(date: CivilDate) -> CivilDate {
    if date.month == 1 {
        CivilDate {
            year: date.year - 1,
            month: 12,
            day: 1,
        }
    } else {
        CivilDate {
            year: date.year,
            month: date.month - 1,
            day: 1,
        }
    }
}

fn short_month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

fn calendar_control_key(event: &KeyDownEvent) -> bool {
    !event.keystroke.modifiers.modified()
        && matches!(event.keystroke.key.as_str(), "enter" | "space")
}
