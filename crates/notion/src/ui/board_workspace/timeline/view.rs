use super::timeline_header::TimelineHeaderLayout;
use super::{
    add_days, alpha, civil_date_is_weekend, div, px, rgb, timeline_bar, timeline_content_range,
    timeline_date_range_for_item, timeline_day_cell_width, timeline_grid_rule_color,
    timeline_grid_width, timeline_marker_date, timeline_today_line_color,
    timeline_weekend_band_color, AnyElement, AppearanceMode, CivilDate, Context, Div, FontWeight,
    InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    SurfaceState, ACTIVE_VIEW_HEIGHT, TIMELINE_DAY_ROW_LEFT_INSET, TIMELINE_EMPTY_BODY_HEIGHT,
    TIMELINE_ROW_HEIGHT, TIMELINE_TODAY_COLOR,
};
use crate::ui::surface::DatabaseSearchState;
use crate::ui::BoardDateValue;
use crate::ui::BoardItem;
use crate::ui::{Theme, TimelineViewConfig};
use gpui::{RenderImage, ScrollHandle};
use std::sync::Arc;

pub(super) struct TimelineRenderer<'a> {
    timeline_view: Option<&'a TimelineViewConfig>,
    items: &'a [BoardItem],
    database_search: &'a DatabaseSearchState,
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) property_date_icon: Arc<RenderImage>,
    scroll_handle: &'a ScrollHandle,
    viewport_width: f32,
}

impl SurfaceState {
    pub(crate) fn render_timeline_view(&self, cx: &mut Context<Self>) -> Div {
        TimelineRenderer {
            timeline_view: self.board.timeline_view.as_ref(),
            items: &self.board.items,
            database_search: &self.database_search,
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            property_date_icon: self.icons.property_date.render(cx),
            scroll_handle: &self.board_view.timeline_scroll_handle,
            viewport_width: self.board_viewport().width,
        }
        .render_timeline_view()
    }
}

struct TimelineLayout<'a> {
    header: TimelineHeaderLayout,
    visible_items: Vec<TimelineVisibleItem<'a>>,
    timeline_viewport_width: f32,
    grid_width: f32,
    day_count: usize,
    rows_height: f32,
    today_line_left: f32,
}

#[derive(Clone, Copy)]
struct TimelineVisibleItem<'a> {
    item: &'a BoardItem,
    bar: (f32, f32),
}

impl TimelineRenderer<'_> {
    pub(crate) fn render_timeline_view(&self) -> Div {
        let Some(timeline_view) = self.timeline_view else {
            return self.render_empty_timeline_view();
        };
        let layout = self.build_timeline_layout(timeline_view);

        div().h(px(ACTIVE_VIEW_HEIGHT)).child(
            (div()
                .h_full()
                .w_full()
                .rounded(px(12.0))
                .bg(rgb(self.theme.app_bg))
                .overflow_hidden()
                .child(
                    (div()
                        .h_full()
                        .w(px(layout.timeline_viewport_width))
                        .id("timeline-view-scroll")
                        .overflow_scroll()
                        .track_scroll(self.scroll_handle)
                        .child(
                            div()
                                .w(px(layout.grid_width))
                                .min_h_full()
                                .flex()
                                .flex_col()
                                .child(self.render_timeline_header(&layout.header))
                                .child(self.render_timeline_rows(&layout)),
                        ))
                    .into_any_element(),
                ))
            .into_any_element(),
        )
    }

    fn render_empty_timeline_view(&self) -> Div {
        div().h(px(ACTIVE_VIEW_HEIGHT)).child(
            div()
                .h_full()
                .w_full()
                .rounded(px(12.0))
                .bg(rgb(self.theme.app_bg)),
        )
    }

    fn build_timeline_layout<'a>(
        &'a self,
        timeline_view: &TimelineViewConfig,
    ) -> TimelineLayout<'a> {
        let dated_items = self.timeline_dated_items(timeline_view);
        let today_marker_date = timeline_marker_date(timeline_view);
        let timeline_content =
            timeline_content_range(timeline_view, &dated_items, today_marker_date);
        let timeline_viewport_width = self.viewport_width;
        let day_cell_width = timeline_day_cell_width(timeline_view);
        let day_count =
            (timeline_content.end.ordinal() - timeline_content.start.ordinal()).max(1) as usize;
        let grid_width =
            timeline_grid_width(day_count, day_cell_width).max(timeline_viewport_width);
        let visible_items = self.timeline_visible_items(
            &dated_items,
            timeline_content.start,
            timeline_content.end,
            day_cell_width,
        );
        let rows_height = self.timeline_rows_height(visible_items.len());
        let today_line_left = self.timeline_today_line_left(
            today_marker_date,
            timeline_content.start,
            day_count,
            day_cell_width,
        );
        TimelineLayout {
            header: TimelineHeaderLayout {
                content_start: timeline_content.start,
                content_end: timeline_content.end,
                today_marker_date,
                grid_width,
                day_cell_width,
            },
            visible_items,
            timeline_viewport_width,
            grid_width,
            day_count,
            rows_height,
            today_line_left,
        }
    }

    fn timeline_dated_items<'a>(
        &'a self,
        timeline_view: &TimelineViewConfig,
    ) -> Vec<(&'a BoardItem, &'a BoardDateValue)> {
        self.items
            .iter()
            .filter(|item| self.database_search.item_is_visible(item))
            .filter_map(|item| {
                timeline_date_range_for_item(item, timeline_view)
                    .map(|date_range| (item, date_range))
            })
            .collect()
    }

    fn timeline_visible_items<'a>(
        &'a self,
        dated_items: &[(&'a BoardItem, &'a BoardDateValue)],
        content_start: CivilDate,
        content_end: CivilDate,
        day_cell_width: f32,
    ) -> Vec<TimelineVisibleItem<'a>> {
        dated_items
            .iter()
            .filter_map(|(item, date_range)| {
                timeline_bar(date_range, content_start, content_end, day_cell_width)
                    .map(|bar| TimelineVisibleItem { item, bar })
            })
            .collect()
    }

    fn timeline_rows_height(&self, visible_item_count: usize) -> f32 {
        if visible_item_count == 0 {
            TIMELINE_EMPTY_BODY_HEIGHT
        } else {
            (((visible_item_count + 1) as f32) * TIMELINE_ROW_HEIGHT)
                .max(TIMELINE_EMPTY_BODY_HEIGHT)
        }
    }

    fn timeline_today_line_left(
        &self,
        today_marker_date: CivilDate,
        content_start: CivilDate,
        day_count: usize,
        day_cell_width: f32,
    ) -> f32 {
        let today_index = (today_marker_date.ordinal() - content_start.ordinal())
            .clamp(0, day_count.saturating_sub(1) as i64) as f32;
        TIMELINE_DAY_ROW_LEFT_INSET + today_index * day_cell_width + day_cell_width / 2.0
    }

    fn render_timeline_rows(&self, layout: &TimelineLayout<'_>) -> AnyElement {
        (div()
            .relative()
            .h(px(layout.rows_height))
            .bg(rgb(self.timeline_rows_background()))
            .border_t_1()
            .border_color(timeline_grid_rule_color(
                self.appearance_mode,
                self.theme.text_primary,
            ))
            .children((0..layout.day_count).map(|day_index| {
                self.render_timeline_weekend_band(&layout.header, day_index, layout.rows_height)
            }))
            .child(self.render_timeline_new_row())
            .child(self.render_timeline_today_marker(layout.today_line_left, layout.rows_height))
            .children(
                layout
                    .visible_items
                    .iter()
                    .map(|item| self.render_timeline_row(item.item, item.bar, layout.grid_width)),
            ))
        .into_any_element()
    }

    fn timeline_rows_background(&self) -> u32 {
        if self.appearance_mode == AppearanceMode::Light {
            0xfdfdfd
        } else {
            self.theme.app_bg
        }
    }

    fn render_timeline_weekend_band(
        &self,
        header: &TimelineHeaderLayout,
        day_index: usize,
        rows_height: f32,
    ) -> Div {
        let day = add_days(header.content_start, day_index as i64);
        if !civil_date_is_weekend(day) {
            return div();
        }
        div()
            .absolute()
            .left(px(
                TIMELINE_DAY_ROW_LEFT_INSET + day_index as f32 * header.day_cell_width
            ))
            .top(px(0.0))
            .w(px(header.day_cell_width))
            .h(px(rows_height))
            .bg(timeline_weekend_band_color(
                self.appearance_mode,
                self.theme.text_primary,
            ))
    }

    fn render_timeline_new_row(&self) -> AnyElement {
        (div()
            .absolute()
            .left(px(3.0))
            .top(px(13.0))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(alpha(self.theme.text_primary, 0.65))
            .child(div().mr(px(8.0)).text_size(px(18.0)).child("+"))
            .child("New"))
        .into_any_element()
    }

    fn render_timeline_today_marker(&self, today_line_left: f32, rows_height: f32) -> AnyElement {
        (div()
            .absolute()
            .left(px(today_line_left.max(0.0)))
            .top(px(0.0))
            .w(px(1.0))
            .h(px(rows_height))
            .bg(timeline_today_line_color(self.appearance_mode))
            .child(
                div()
                    .absolute()
                    .left(px(-2.0))
                    .top(px(0.0))
                    .size(px(5.0))
                    .rounded_full()
                    .bg(rgb(TIMELINE_TODAY_COLOR)),
            ))
        .into_any_element()
    }
}
