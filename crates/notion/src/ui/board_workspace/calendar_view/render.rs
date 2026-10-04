use std::sync::Arc;

use gpui::App;

use super::query::handle_calendar_action;
use super::{
    div, month_name, px, rgb, AnyElement, CalendarAction, CalendarLayout, CalendarRangeResizeDrag,
    CalendarViewConfig, CalendarWeek, CivilDate, Context, Div, FluentBuilder, InteractiveElement,
    IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled, SurfaceState,
    BOARD_VIEWPORT_RIGHT_GUTTER, BOARD_VIEWPORT_X, CALENDAR_DAY_EVENT_TOP_INSET,
    CALENDAR_EVENT_ROW_PITCH, CALENDAR_WEEKDAY_HEADER_HEIGHT, INLINE_DATABASE_LEFT_OVERFLOW,
};
use crate::ui::board_workspace::inline_database::InlineDatabaseLayout;
use crate::ui::surface::{DatabaseSearchState, NotionDateViewState};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{IconSet, Theme};

use super::super::PageShellIconRenderer;

pub(super) struct CalendarRenderer<'a> {
    pub(super) calendar_view: &'a CalendarViewConfig,
    pub(super) date_view: &'a NotionDateViewState,
    pub(super) database_search: &'a DatabaseSearchState,
    pub(super) theme: Theme,
    pub(super) icons: Arc<IconSet>,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) mutation_available: bool,
    pub(super) creation_available: bool,
    pub(super) actions: ViewActionSink<CalendarAction>,
}

struct CalendarRendererResources {
    theme: Theme,
    icons: Arc<IconSet>,
    page_icons: PageShellIconRenderer,
    mutation_available: bool,
    creation_available: bool,
    actions: ViewActionSink<CalendarAction>,
}

impl<'a> CalendarRenderer<'a> {
    fn new(
        calendar_view: &'a CalendarViewConfig,
        date_view: &'a NotionDateViewState,
        database_search: &'a DatabaseSearchState,
        resources: CalendarRendererResources,
    ) -> Self {
        let CalendarRendererResources {
            theme,
            icons,
            page_icons,
            mutation_available,
            creation_available,
            actions,
        } = resources;
        Self {
            calendar_view,
            date_view,
            database_search,
            theme,
            icons,
            page_icons,
            mutation_available,
            creation_available,
            actions,
        }
    }

    pub(super) fn render_calendar_view(&self, cx: &mut App) -> Div {
        let layout = self.current_calendar_layout();
        self.ensure_calendar_day_focus_handles(&layout, cx);
        div()
            .h(px(layout.active_view_height()))
            .pl(px(BOARD_VIEWPORT_X))
            .pr(px(BOARD_VIEWPORT_RIGHT_GUTTER))
            .child(self.render_calendar_content(&layout, cx))
    }

    pub(super) fn render_inline_calendar_view(
        &self,
        database: InlineDatabaseLayout,
        cx: &mut App,
    ) -> Div {
        let layout = self.current_calendar_layout();
        self.ensure_calendar_day_focus_handles(&layout, cx);
        div()
            .h(px(layout.active_view_height()))
            .ml(px(INLINE_DATABASE_LEFT_OVERFLOW))
            .w(px(database.content_width()))
            .child(self.render_calendar_content(&layout, cx))
    }

    fn render_calendar_content(&self, layout: &CalendarLayout<'_>, cx: &mut App) -> Div {
        let label = format!(
            "{} {}",
            month_name(layout.month_start.month),
            layout.month_start.year
        );
        div()
            .size_full()
            .min_w(px(0.0))
            .bg(rgb(self.theme.app_bg))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.render_calendar_header(&label, cx))
            .child(self.render_calendar_weekday_header())
            .child(self.render_calendar_grid(layout, label, cx))
    }

    fn render_calendar_grid(
        &self,
        layout: &CalendarLayout<'_>,
        label: String,
        cx: &mut App,
    ) -> AnyElement {
        let geometry = layout.grid_geometry();
        let move_actions = self.actions.clone();
        let drop_actions = self.actions.clone();
        div()
            .id("notion-calendar-grid")
            .role(Role::Grid)
            .aria_label(label)
            .w_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_t_1()
            .border_l_1()
            .border_color(self.calendar_grid_color())
            .can_drop(|dragged, _, _| dragged.downcast_ref::<CalendarRangeResizeDrag>().is_some())
            .on_drag_move::<CalendarRangeResizeDrag>(move |event, window, cx| {
                let drag = event.drag(cx).clone();
                let target_date =
                    geometry.date_at_grid_position(event.event.position, event.bounds);
                move_actions.emit(
                    CalendarAction::PreviewRangeResize { drag, target_date },
                    window,
                    cx,
                );
            })
            .on_drop(move |dragged: &CalendarRangeResizeDrag, window, cx| {
                cx.stop_propagation();
                drop_actions.emit(
                    CalendarAction::FinishRangeResize(dragged.clone()),
                    window,
                    cx,
                );
            })
            .opacity(if self.date_view.calendar_items.pending {
                0.52
            } else {
                1.0
            })
            .children(
                layout
                    .weeks
                    .iter()
                    .map(|week| self.render_calendar_week(week, layout.month_start, cx)),
            )
            .into_any_element()
    }

    fn render_calendar_weekday_header(&self) -> Div {
        div()
            .h(px(CALENDAR_WEEKDAY_HEADER_HEIGHT))
            .ml(px(1.0))
            .self_stretch()
            .flex_none()
            .grid()
            .grid_cols(7)
            .children(
                ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(|day| {
                    div()
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.0))
                        .text_color(rgb(self.theme.calendar_muted_text))
                        .child(day)
                }),
            )
    }

    fn render_calendar_week(
        &self,
        week: &CalendarWeek<'_>,
        month_start: CivilDate,
        cx: &mut App,
    ) -> AnyElement {
        let week_start = week
            .days
            .first()
            .expect("Notion Calendar weeks must contain seven days")
            .date;
        let lane_count = week
            .segments
            .iter()
            .map(|segment| segment.lane + 1)
            .max()
            .unwrap_or(0);
        div()
            .id(format!(
                "notion-calendar-week-{:04}-{:02}-{:02}",
                week_start.year, week_start.month, week_start.day
            ))
            .relative()
            .h(px(week.height))
            .w_full()
            .flex_none()
            .grid()
            .grid_cols(7)
            .children(
                week.days
                    .iter()
                    .map(|day| self.render_calendar_day(day, month_start, week.height, cx)),
            )
            .when(lane_count > 0, |week_row| {
                week_row.child(self.render_calendar_week_segments(week, week_start, lane_count, cx))
            })
            .into_any_element()
    }

    fn render_calendar_week_segments(
        &self,
        week: &CalendarWeek<'_>,
        week_start: CivilDate,
        lane_count: usize,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .absolute()
            .top(px(CALENDAR_DAY_EVENT_TOP_INSET))
            .left(px(0.0))
            .right(px(0.0))
            .h(px(lane_count as f32 * CALENDAR_EVENT_ROW_PITCH))
            .grid()
            .grid_cols(7)
            .grid_rows(lane_count as u16)
            .children(
                week.segments
                    .iter()
                    .map(|segment| self.render_calendar_week_segment(segment, week_start, cx)),
            )
            .into_any_element()
    }
}

impl SurfaceState {
    pub(crate) fn render_calendar_view(&self, cx: &mut Context<Self>) -> Div {
        self.schedule_date_view_queries(cx);
        if self
            .date_view
            .range_resize_preview_is_stale(cx.has_active_drag())
        {
            let surface = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = surface.update(cx, |surface, cx| {
                    if surface.date_view.clear_range_resize_preview() {
                        cx.notify();
                    }
                });
            });
        }
        let mutation_available = self.notion_startup.workspace_api().is_some();
        CalendarRenderer::new(
            self.board
                .calendar_view
                .as_ref()
                .expect("rendered Calendar view must include Calendar configuration"),
            &self.date_view,
            &self.database_search,
            CalendarRendererResources {
                theme: self.theme,
                icons: self.icons.clone(),
                page_icons: self.page_shell_icon_renderer(cx),
                mutation_available,
                creation_available: mutation_available
                    && !self.board.is_locked
                    && self.board.active_date_view().is_some(),
                actions: ViewActionSink::new(cx, handle_calendar_action),
            },
        )
        .render_calendar_view(cx)
    }

    pub(crate) fn render_inline_calendar_view(
        &self,
        database: InlineDatabaseLayout,
        cx: &mut Context<Self>,
    ) -> Div {
        self.schedule_date_view_queries(cx);
        if self
            .date_view
            .range_resize_preview_is_stale(cx.has_active_drag())
        {
            let surface = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = surface.update(cx, |surface, cx| {
                    if surface.date_view.clear_range_resize_preview() {
                        cx.notify();
                    }
                });
            });
        }
        let mutation_available = self.notion_startup.workspace_api().is_some();
        CalendarRenderer::new(
            self.board
                .calendar_view
                .as_ref()
                .expect("rendered Calendar view must include Calendar configuration"),
            &self.date_view,
            &self.database_search,
            CalendarRendererResources {
                theme: self.theme,
                icons: self.icons.clone(),
                page_icons: self.page_shell_icon_renderer(cx),
                mutation_available,
                creation_available: mutation_available
                    && !self.board.is_locked
                    && self.board.active_date_view().is_some(),
                actions: ViewActionSink::new(cx, handle_calendar_action),
            },
        )
        .render_inline_calendar_view(database, cx)
    }
}
