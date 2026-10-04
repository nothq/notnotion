use chrono::{Datelike, Duration, NaiveDate, Weekday};
use gpui::prelude::FluentBuilder;
use gpui::{
    Div, ElementId, FontWeight, Hsla, InteractiveElement, MouseButton, ParentElement, Role,
    SharedString, Stateful, StatefulInteractiveElement, Styled,
};

use super::super::super::actions::PageMentionPickerAction;
use super::super::{
    PAGE_MENTION_PICKER_CALENDAR_ROWS, PAGE_MENTION_PICKER_CELL, PAGE_MENTION_PICKER_DAY_BUTTON,
    PAGE_MENTION_PICKER_INSET, PAGE_MENTION_PICKER_MUTED_DAY, PAGE_MENTION_PICKER_SELECTED_DAY,
    PAGE_MENTION_PICKER_TODAY_BUTTON, PAGE_MENTION_PICKER_TODAY_MARK,
};
use super::{PageMentionPickerPresentation, PageMentionRenderer};
use crate::ui::{alpha, div, px, rgb, PageMentionPickerState};

const MONTH_ABBREVIATIONS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAY_ABBREVIATIONS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

impl PageMentionRenderer {
    pub(super) fn render_page_mention_picker_month_header(
        &self,
        presentation: &PageMentionPickerPresentation,
    ) -> Div {
        let month = presentation.picker.visible_month;
        let label = format!(
            "{} {}",
            MONTH_ABBREVIATIONS[month.month0() as usize],
            month.year()
        );
        div()
            .mx(px(PAGE_MENTION_PICKER_INSET))
            .mt(px(9.0))
            .h(px(25.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(21.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.resources.theme.text_primary))
                    .child(label),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(self.render_page_mention_picker_today_button(presentation))
                    .child(self.render_page_mention_picker_chevron(
                        "previous",
                        "‹",
                        presentation.command(PageMentionPickerAction::ShiftMonth(-1)),
                    ))
                    .child(self.render_page_mention_picker_chevron(
                        "next",
                        "›",
                        presentation.command(PageMentionPickerAction::ShiftMonth(1)),
                    )),
            )
    }

    fn render_page_mention_picker_today_button(
        &self,
        presentation: &PageMentionPickerPresentation,
    ) -> Stateful<Div> {
        div()
            .id("notion-mention-date-picker-today")
            .role(Role::Button)
            .aria_label("Today")
            .h(px(20.0))
            .px(px(8.0))
            .rounded(px(3.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(PAGE_MENTION_PICKER_TODAY_BUTTON))
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .on_mouse_down(
                MouseButton::Left,
                self.picker_action_handler(
                    presentation.command(PageMentionPickerAction::ShowToday),
                ),
            )
            .child("Today")
    }

    fn render_page_mention_picker_chevron(
        &self,
        id: &'static str,
        glyph: &'static str,
        command: super::PageMentionPickerCommand,
    ) -> Stateful<Div> {
        let previous = matches!(
            &command.action,
            PageMentionPickerAction::ShiftMonth(delta) if *delta < 0
        );
        div()
            .id(ElementId::Name(
                format!("notion-mention-date-picker-{id}-month").into(),
            ))
            .role(Role::Button)
            .aria_label(if previous {
                "Previous month"
            } else {
                "Next month"
            })
            .size(px(24.0))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_size(px(18.0))
            .text_color(rgb(PAGE_MENTION_PICKER_TODAY_BUTTON))
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .on_mouse_down(MouseButton::Left, self.picker_action_handler(command))
            .child(glyph)
    }

    pub(super) fn render_page_mention_picker_weekdays(&self) -> Div {
        div()
            .mx(px(PAGE_MENTION_PICKER_INSET))
            .h(px(PAGE_MENTION_PICKER_CELL))
            .flex()
            .children(WEEKDAY_ABBREVIATIONS.iter().map(|label| {
                div()
                    .size(px(PAGE_MENTION_PICKER_CELL))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .text_color(rgb(PAGE_MENTION_PICKER_MUTED_DAY))
                    .child(*label)
            }))
    }

    pub(super) fn render_page_mention_picker_grid(
        &self,
        presentation: &PageMentionPickerPresentation,
    ) -> Div {
        let today = self.resources.clock.today;
        let first_visible = calendar_grid_start(presentation.picker.visible_month);
        let mut grid = div().mx(px(PAGE_MENTION_PICKER_INSET)).flex().flex_col();
        for row in 0..PAGE_MENTION_PICKER_CALENDAR_ROWS {
            let mut week = div().h(px(PAGE_MENTION_PICKER_CELL)).flex();
            for column in 0..7 {
                let date = first_visible + Duration::days((row * 7 + column) as i64);
                week = week.child(self.render_page_mention_picker_day(presentation, date, today));
            }
            grid = grid.child(week);
        }
        grid
    }

    fn render_page_mention_picker_day(
        &self,
        presentation: &PageMentionPickerPresentation,
        date: NaiveDate,
        today: NaiveDate,
    ) -> Div {
        let picker = &presentation.picker;
        let selected = date == picker.draft.start_date || picker.draft.end_date == Some(date);
        let in_range = picker
            .draft
            .end_date
            .is_some_and(|end| date > picker.draft.start_date && date < end);
        let color = self.page_mention_picker_day_color(picker, date, today, selected);
        let button = div()
            .id(ElementId::Name(
                format!("notion-mention-date-picker-day-{date}").into(),
            ))
            .role(Role::Button)
            .aria_label(SharedString::from(date.format("%B %-d, %Y").to_string()))
            .size(px(PAGE_MENTION_PICKER_DAY_BUTTON))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_size(px(14.0))
            .line_height(px(16.8))
            .text_color(color)
            .when(selected, |day| {
                day.bg(rgb(PAGE_MENTION_PICKER_SELECTED_DAY))
            })
            .when(in_range, |day| {
                day.bg(alpha(PAGE_MENTION_PICKER_SELECTED_DAY, 0.12))
            })
            .when(!selected, |day| {
                day.hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            })
            .on_mouse_down(
                MouseButton::Left,
                self.picker_action_handler(
                    presentation.command(PageMentionPickerAction::SelectDay(date)),
                ),
            )
            .child(date.day().to_string());
        div()
            .size(px(PAGE_MENTION_PICKER_CELL))
            .flex()
            .items_center()
            .justify_center()
            .child(button)
    }

    fn page_mention_picker_day_color(
        &self,
        picker: &PageMentionPickerState,
        date: NaiveDate,
        today: NaiveDate,
        selected: bool,
    ) -> Hsla {
        if selected {
            return rgb(0xffffff).into();
        }
        if date.month() != picker.visible_month.month() {
            return rgb(PAGE_MENTION_PICKER_MUTED_DAY).into();
        }
        if date == today {
            return rgb(PAGE_MENTION_PICKER_TODAY_MARK).into();
        }
        rgb(self.resources.theme.text_primary).into()
    }
}

/// The Sunday that starts the calendar grid for `visible_month`.
fn calendar_grid_start(visible_month: NaiveDate) -> NaiveDate {
    let offset = visible_month.weekday().num_days_from_sunday();
    visible_month - Duration::days(i64::from(offset))
}

#[allow(dead_code)]
const fn weekday_column(weekday: Weekday) -> usize {
    weekday.num_days_from_sunday() as usize
}
