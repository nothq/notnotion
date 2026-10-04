use crate::ui::board_workspace::dialogs::filter::{prelude::*, types::*};

impl DatabaseFilterUiState {
    pub(super) fn render_database_filter_calendar(
        &self,
        resources: &DatabaseFilterRenderResources,
        draft: &DatabaseFilterDraft,
    ) -> AnyElement {
        let month = self.visible_date_month;
        let selection = database_calendar_selection(draft);
        let leading_days = month.weekday().num_days_from_monday() as i64;
        let grid_start = month - Duration::days(leading_days);
        let cells = (0_i64..42).map(|offset| grid_start + Duration::days(offset));
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(self.render_database_filter_calendar_header(resources, month))
            .child(database_filter_calendar_weekdays(resources.theme))
            .child(div().flex().flex_wrap().children(cells.map(|date| {
                self.render_database_filter_calendar_cell(resources, date, month, selection)
            })))
            .into_any_element()
    }

    fn render_database_filter_calendar_header(
        &self,
        resources: &DatabaseFilterRenderResources,
        month: NaiveDate,
    ) -> Div {
        div()
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .child(self.render_database_filter_calendar_navigation(
                resources,
                -1,
                "Previous month",
                "‹",
            ))
            .child(
                div()
                    .flex_grow(1.0)
                    .text_align(gpui::TextAlign::Center)
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(month.format("%B %Y").to_string()),
            )
            .child(self.render_database_filter_calendar_navigation(resources, 1, "Next month", "›"))
    }

    fn render_database_filter_calendar_navigation(
        &self,
        resources: &DatabaseFilterRenderResources,
        direction: i32,
        label: &'static str,
        glyph: &'static str,
    ) -> gpui::Stateful<Div> {
        let id = if direction < 0 { "previous" } else { "next" };
        let action = DatabaseFilterAction::MoveCalendarMonth(direction);
        div()
            .id(format!("notion-database-filter-date-{id}-month"))
            .size(px(28.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(label)
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .flex()
            .items_center()
            .justify_center()
            .child(glyph)
    }

    fn render_database_filter_calendar_cell(
        &self,
        resources: &DatabaseFilterRenderResources,
        date: NaiveDate,
        month: NaiveDate,
        selection: DatabaseCalendarSelection,
    ) -> gpui::Stateful<Div> {
        let is_selected = selection.selected_date == Some(date)
            || selection.range_start == Some(date)
            || selection.range_end == Some(date);
        let is_in_range = selection
            .range_start
            .zip(selection.range_end)
            .is_some_and(|(start, end)| date >= start && date <= end);
        let color = database_calendar_date_color(resources.theme, date, month, is_selected);
        let action = DatabaseFilterAction::SetDate(date);
        div()
            .id(format!("notion-database-filter-date-{date}"))
            .w(px(34.8))
            .h(px(30.0))
            .flex_none()
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(date.format("%B %-d, %Y").to_string())
            .focusable()
            .tab_stop(true)
            .cursor_pointer()
            .when(is_in_range && !is_selected, |cell| {
                cell.bg(alpha(0x2383e2, 0.14))
            })
            .when(is_selected, |cell| cell.bg(rgb(0x2383e2)))
            .hover(|style| style.bg(alpha(resources.theme.text_primary, 0.08)))
            .on_click(resources.filter_click_listener(action.clone()))
            .on_key_down(resources.filter_key_listener(action))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(color)
            .child(date.day().to_string())
    }
}

fn database_calendar_selection(draft: &DatabaseFilterDraft) -> DatabaseCalendarSelection {
    let is_range = draft.operator == DatabaseTextFilterOperator::DateIsBetween;
    let selected_date = match (&draft.date_point, is_range) {
        (DatabaseDatePoint::Exact(date), false) => Some(*date),
        (DatabaseDatePoint::Exact(_) | DatabaseDatePoint::Relative(_), true)
        | (DatabaseDatePoint::Relative(_), false) => None,
    };
    let (range_start, range_end) = match (&draft.date_range, is_range) {
        (
            DatabaseDateRange::Exact {
                start_date,
                end_date,
            },
            true,
        ) => (*start_date, *end_date),
        _ => (None, None),
    };
    DatabaseCalendarSelection {
        selected_date,
        range_start,
        range_end,
    }
}

fn database_filter_calendar_weekdays(theme: Theme) -> Div {
    div()
        .h(px(22.0))
        .flex_none()
        .flex()
        .children(["M", "T", "W", "T", "F", "S", "S"].map(|day| {
            div()
                .w(px(34.8))
                .flex_none()
                .text_align(gpui::TextAlign::Center)
                .text_size(px(11.0))
                .text_color(rgb(theme.text_hint))
                .child(day)
        }))
}

fn database_calendar_date_color(
    theme: Theme,
    date: NaiveDate,
    month: NaiveDate,
    selected: bool,
) -> gpui::Rgba {
    if selected {
        rgb(0xffffff)
    } else if date.month() == month.month() {
        rgb(theme.text_primary)
    } else {
        rgb(theme.text_hint)
    }
}
