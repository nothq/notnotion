use super::view::TimelineRenderer;
use super::{
    add_days, column_style, div, img, px, relative, rgb, rgba, timeline_grid_rule_color,
    timeline_header_month_labels, AnyElement, CivilDate, Div, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Styled, TIMELINE_CONTROLS_RIGHT_INSET, TIMELINE_CONTROLS_TOP_INSET,
    TIMELINE_DAY_LABEL_TOP_INSET, TIMELINE_DAY_MARKER_SIZE, TIMELINE_DAY_MARKER_TOP_INSET,
    TIMELINE_DAY_ROW_LEFT_INSET, TIMELINE_DAY_ROW_RIGHT_INSET, TIMELINE_HEADER_HEIGHT,
    TIMELINE_MONTH_LABEL_HEIGHT, TIMELINE_MONTH_LABEL_TOP_INSET, TIMELINE_ROW_HEIGHT,
    TIMELINE_TODAY_COLOR,
};
use crate::ui::BoardItem;

pub(crate) struct TimelineHeaderLayout {
    pub(crate) content_start: CivilDate,
    pub(crate) content_end: CivilDate,
    pub(crate) today_marker_date: CivilDate,
    pub(crate) grid_width: f32,
    pub(crate) day_cell_width: f32,
}

impl TimelineRenderer<'_> {
    pub(crate) fn render_timeline_header(&self, header: &TimelineHeaderLayout) -> AnyElement {
        let day_count =
            (header.content_end.ordinal() - header.content_start.ordinal()).max(1) as usize;

        (div()
            .relative()
            .h(px(TIMELINE_HEADER_HEIGHT))
            .border_b_1()
            .border_color(timeline_grid_rule_color(
                self.appearance_mode,
                self.theme.text_primary,
            ))
            .child(self.render_timeline_header_controls())
            .child(self.render_timeline_header_grid(header, day_count)))
        .into_any_element()
    }

    pub(crate) fn render_timeline_row(
        &self,
        item: &BoardItem,
        bar: (f32, f32),
        grid_width: f32,
    ) -> AnyElement {
        let (bar_left, bar_width) = bar;
        let tone = column_style(
            item.status.as_deref().unwrap_or("Backlog"),
            None,
            self.appearance_mode,
        );

        (div()
            .id(format!("notion-timeline-row-{}", item.block_id))
            .h(px(TIMELINE_ROW_HEIGHT))
            .child(
                (div().relative().h_full().w(px(grid_width)).child(
                    (div()
                        .absolute()
                        .left(px(bar_left.max(0.0)))
                        .top(px(6.0))
                        .h(px(22.0))
                        .w(px(bar_width.max(36.0)))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(rgba(tone.tone.action_border(self.appearance_mode)))
                        .bg(rgba(tone.pill))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .child(
                            (div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(tone.tone.action_text_color()))
                                .child(item.title.clone()))
                            .into_any_element(),
                        ))
                    .into_any_element(),
                ))
                .into_any_element(),
            ))
        .into_any_element()
    }
}

impl TimelineRenderer<'_> {
    fn render_timeline_header_controls(&self) -> AnyElement {
        (div()
            .absolute()
            .top(px(TIMELINE_CONTROLS_TOP_INSET))
            .right(px(TIMELINE_CONTROLS_RIGHT_INSET))
            .flex()
            .items_center()
            .gap(px(14.0))
            .child(self.render_timeline_manage_calendar_button())
            .child(self.render_timeline_zoom_button())
            .child(self.render_timeline_jump_button()))
        .into_any_element()
    }

    fn render_timeline_manage_calendar_button(&self) -> AnyElement {
        (div()
            .h(px(24.0))
            .px(px(10.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(timeline_grid_rule_color(
                self.appearance_mode,
                self.theme.text_primary,
            ))
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .line_height(relative(1.2))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .child(img(self.property_date_icon.clone()).size(px(14.0)))
            .child("Manage in Calendar"))
        .into_any_element()
    }

    fn render_timeline_zoom_button(&self) -> AnyElement {
        (div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .text_size(px(14.0))
            .line_height(relative(1.2))
            .text_color(rgb(self.theme.text_secondary))
            .child("Month")
            .child("v"))
        .into_any_element()
    }

    fn render_timeline_jump_button(&self) -> AnyElement {
        (div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .text_size(px(14.0))
            .line_height(relative(1.2))
            .text_color(rgb(self.theme.text_primary))
            .child(div().text_color(rgb(self.theme.text_secondary)).child("<"))
            .child(div().text_color(rgb(self.theme.text_secondary)).child(">"))
            .child("Today"))
        .into_any_element()
    }

    fn render_timeline_header_grid(
        &self,
        header: &TimelineHeaderLayout,
        day_count: usize,
    ) -> AnyElement {
        (div()
            .relative()
            .h_full()
            .w(px(header.grid_width))
            .child(self.render_timeline_month_labels(header))
            .child(self.render_timeline_day_labels(header, day_count)))
        .into_any_element()
    }

    fn render_timeline_month_labels(&self, header: &TimelineHeaderLayout) -> AnyElement {
        let month_labels = timeline_header_month_labels(
            header.content_start,
            header.content_end,
            header.day_cell_width,
        );
        (div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w_full()
            .h(px(TIMELINE_MONTH_LABEL_HEIGHT))
            .children(month_labels.into_iter().map(|(label, left)| {
                (div()
                    .absolute()
                    .left(px(left))
                    .top(px(TIMELINE_MONTH_LABEL_TOP_INSET))
                    .h(px(TIMELINE_MONTH_LABEL_HEIGHT))
                    .flex()
                    .items_center()
                    .text_size(px(14.0))
                    .line_height(relative(1.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_primary))
                    .child(label))
                .into_any_element()
            })))
        .into_any_element()
    }

    fn render_timeline_day_labels(
        &self,
        header: &TimelineHeaderLayout,
        day_count: usize,
    ) -> AnyElement {
        (div()
            .absolute()
            .left(px(TIMELINE_DAY_ROW_LEFT_INSET))
            .right(px(TIMELINE_DAY_ROW_RIGHT_INSET))
            .bottom(px(0.0))
            .h(px(28.0))
            .flex()
            .children(
                (0..day_count).map(|day_index| self.render_timeline_day_label(header, day_index)),
            ))
        .into_any_element()
    }

    fn render_timeline_day_label(
        &self,
        header: &TimelineHeaderLayout,
        day_index: usize,
    ) -> AnyElement {
        let day = add_days(header.content_start, day_index as i64);
        let is_today_marker = day.ordinal() == header.today_marker_date.ordinal();
        (div()
            .w(px(header.day_cell_width))
            .h_full()
            .relative()
            .child(if is_today_marker {
                render_timeline_today_day_marker(day.day)
            } else {
                render_timeline_regular_day_marker(day.day, self.theme.topbar_chip_text)
            }))
        .into_any_element()
    }
}

fn render_timeline_today_day_marker(day: u32) -> Div {
    div()
        .absolute()
        .top(px(TIMELINE_DAY_MARKER_TOP_INSET))
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .child(
            div()
                .size(px(TIMELINE_DAY_MARKER_SIZE))
                .rounded_full()
                .bg(rgb(TIMELINE_TODAY_COLOR))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.0))
                .line_height(relative(28.0 / 12.0))
                .text_color(rgb(0xffffff))
                .child(day.to_string()),
        )
}

fn render_timeline_regular_day_marker(day: u32, text_color: u32) -> Div {
    div()
        .absolute()
        .top(px(TIMELINE_DAY_LABEL_TOP_INSET))
        .left(px(0.0))
        .right(px(0.0))
        .flex()
        .justify_center()
        .child(
            div()
                .text_size(px(12.0))
                .line_height(relative(28.0 / 12.0))
                .text_color(rgb(text_color))
                .child(day.to_string()),
        )
}
