use gpui::prelude::FluentBuilder;
use gpui::{
    Div, InteractiveElement, MouseButton, MouseDownEvent, ParentElement, Role, Stateful,
    StatefulInteractiveElement, Styled,
};

use super::super::super::actions::PageMentionPickerAction;
use super::super::{
    PAGE_MENTION_PICKER_FIELD_HEIGHT, PAGE_MENTION_PICKER_INSET, PAGE_MENTION_PICKER_ROW_HEIGHT,
    PAGE_MENTION_PICKER_ROW_INSET, PAGE_MENTION_PICKER_TODAY_BUTTON, PAGE_MENTION_PICKER_WIDTH,
};
use super::{PageMentionPickerPresentation, PageMentionRenderer};
use crate::ui::{command_menu_shadow, div, px, rgb, PageMentionPickerSubmenu};

impl PageMentionRenderer {
    pub(super) fn render_page_mention_picker_dialog(
        &self,
        presentation: &PageMentionPickerPresentation,
    ) -> Stateful<Div> {
        self.render_page_mention_picker_dialog_shell()
            .child(
                div()
                    .mx(px(PAGE_MENTION_PICKER_INSET))
                    .mt(px(PAGE_MENTION_PICKER_INSET))
                    .h(px(PAGE_MENTION_PICKER_FIELD_HEIGHT))
                    .when_some(presentation.input.clone(), |field, input| {
                        field.child(input)
                    }),
            )
            .child(self.render_page_mention_picker_month_header(presentation))
            .child(self.render_page_mention_picker_weekdays())
            .child(self.render_page_mention_picker_grid(presentation))
            .child(self.render_page_mention_picker_divider())
            .child(self.render_page_mention_picker_option_rows(presentation))
            .child(self.render_page_mention_picker_divider())
            .child(self.render_page_mention_picker_clear_row(presentation))
            .child(self.render_page_mention_picker_divider())
            .child(self.render_page_mention_picker_reminder_help())
    }

    fn render_page_mention_picker_dialog_shell(&self) -> Stateful<Div> {
        div()
            .id("notion-mention-date-picker")
            .role(Role::Dialog)
            .aria_label("Edit date")
            .w(px(PAGE_MENTION_PICKER_WIDTH))
            .rounded(px(10.0))
            .bg(rgb(self.resources.theme.elevated_surface_bg))
            .shadow(command_menu_shadow(self.resources.theme))
            .pb(px(PAGE_MENTION_PICKER_ROW_INSET))
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
    }

    fn render_page_mention_picker_option_rows(
        &self,
        presentation: &PageMentionPickerPresentation,
    ) -> Div {
        let picker = &presentation.picker;
        let has_time = picker.draft.has_time();
        let reminder_label = picker.draft.reminder.as_ref().map_or_else(
            || "None".to_string(),
            |reminder| reminder.menu_label(has_time),
        );
        div()
            .mx(px(PAGE_MENTION_PICKER_ROW_INSET))
            .mt(px(4.0))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .child(self.render_page_mention_picker_switch_row(
                "end-date",
                "End date",
                picker.draft.has_end(),
                presentation.command(PageMentionPickerAction::ToggleEndDate),
            ))
            .child(self.render_page_mention_picker_value_row(
                "date-format",
                "Date format",
                picker.draft.date_format.menu_label(),
                presentation.command(PageMentionPickerAction::ToggleSubmenu(
                    PageMentionPickerSubmenu::DateFormat,
                )),
            ))
            .child(self.render_page_mention_picker_switch_row(
                "include-time",
                "Include time",
                has_time,
                presentation.command(PageMentionPickerAction::ToggleIncludeTime),
            ))
            .when(has_time, |rows| {
                rows.child(self.render_page_mention_picker_value_row(
                    "time-format",
                    "Time format",
                    picker.draft.time_format.menu_label(),
                    presentation.command(PageMentionPickerAction::ToggleSubmenu(
                        PageMentionPickerSubmenu::TimeFormat,
                    )),
                ))
            })
            .child(self.render_page_mention_picker_value_row(
                "remind",
                "Remind",
                reminder_label,
                presentation.command(PageMentionPickerAction::ToggleSubmenu(
                    PageMentionPickerSubmenu::Remind,
                )),
            ))
    }

    fn render_page_mention_picker_clear_row(
        &self,
        presentation: &PageMentionPickerPresentation,
    ) -> Div {
        div()
            .mx(px(PAGE_MENTION_PICKER_ROW_INSET))
            .mt(px(4.0))
            .child(self.render_page_mention_picker_action_row(
                "clear",
                "Clear",
                presentation.command(PageMentionPickerAction::Clear),
            ))
    }

    fn render_page_mention_picker_reminder_help(&self) -> Div {
        div()
            .mx(px(PAGE_MENTION_PICKER_ROW_INSET))
            .mt(px(4.0))
            .h(px(PAGE_MENTION_PICKER_ROW_HEIGHT))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(14.0))
            .text_color(rgb(self.resources.theme.menu_secondary_text))
            .child(
                div()
                    .size(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(PAGE_MENTION_PICKER_TODAY_BUTTON))
                    .text_size(px(12.0))
                    .text_color(rgb(PAGE_MENTION_PICKER_TODAY_BUTTON))
                    .child("?"),
            )
            .child("Learn about reminders")
    }
}
