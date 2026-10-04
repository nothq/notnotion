use gpui::prelude::FluentBuilder;
use gpui::{
    anchored, deferred, point, Anchor, AnchoredPositionMode, AnyElement, App, Div, ElementId,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Role,
    SharedString, Stateful, StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::ClickAwayBoundary;

use super::super::super::actions::PageMentionPickerAction;
use super::super::{
    PAGE_MENTION_PICKER_CALENDAR_ROWS, PAGE_MENTION_PICKER_CELL, PAGE_MENTION_PICKER_FIELD_HEIGHT,
    PAGE_MENTION_PICKER_INSET, PAGE_MENTION_PICKER_ROW_HEIGHT, PAGE_MENTION_PICKER_ROW_INSET,
    PAGE_MENTION_PICKER_SUBMENU_WIDTH, PAGE_MENTION_PICKER_WIDTH,
};
use super::{PageMentionPickerPresentation, PageMentionRenderer};
use crate::model::{PageMentionDateFormat, PageMentionReminder, PageMentionTimeFormat};
use crate::ui::{
    alpha, command_menu_shadow, div, px, rgb, PageMentionPickerState, PageMentionPickerSubmenu,
};

const OPTION_ROWS_TOP: f32 = PAGE_MENTION_PICKER_INSET
    + PAGE_MENTION_PICKER_FIELD_HEIGHT
    + 9.0
    + 25.0
    + PAGE_MENTION_PICKER_CELL
    + PAGE_MENTION_PICKER_CELL * PAGE_MENTION_PICKER_CALENDAR_ROWS as f32
    + 4.0
    + 1.0
    + 4.0;
const OPTION_ROW_STRIDE: f32 = PAGE_MENTION_PICKER_ROW_HEIGHT + 1.0;

impl PageMentionRenderer {
    pub(super) fn render_page_mention_picker_submenu(
        &self,
        presentation: &PageMentionPickerPresentation,
        submenu: PageMentionPickerSubmenu,
        click_away: &ClickAwayBoundary,
        cx: &mut App,
    ) -> AnyElement {
        let position = picker_submenu_position(&presentation.picker, submenu);
        let options = picker_submenu_options(&presentation.picker, submenu);
        let panel = self.render_page_mention_picker_submenu_panel(presentation, options);
        let panel = click_away.member(div().child(panel), cx);
        deferred(
            anchored()
                .position(position)
                .position_mode(AnchoredPositionMode::Window)
                .anchor(Anchor::TopLeft)
                .child(panel),
        )
        .with_priority(111)
        .into_any_element()
    }

    fn render_page_mention_picker_submenu_panel(
        &self,
        presentation: &PageMentionPickerPresentation,
        options: Vec<PickerSubmenuOption>,
    ) -> Stateful<Div> {
        div()
            .id("notion-mention-date-picker-submenu")
            .role(Role::ListBox)
            .w(px(PAGE_MENTION_PICKER_SUBMENU_WIDTH))
            .rounded(px(10.0))
            .bg(rgb(self.resources.theme.elevated_surface_bg))
            .shadow(command_menu_shadow(self.resources.theme))
            .p(px(PAGE_MENTION_PICKER_ROW_INSET))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
            .children(options.into_iter().enumerate().map(|(index, option)| {
                self.render_page_mention_picker_submenu_option(presentation, index, option)
            }))
    }

    fn render_page_mention_picker_submenu_option(
        &self,
        presentation: &PageMentionPickerPresentation,
        index: usize,
        option: PickerSubmenuOption,
    ) -> Stateful<Div> {
        let label = option.label;
        div()
            .id(ElementId::Name(
                format!("notion-mention-date-picker-option-{index}").into(),
            ))
            .role(Role::ListBoxOption)
            .aria_label(SharedString::from(label.clone()))
            .h(px(PAGE_MENTION_PICKER_ROW_HEIGHT))
            .rounded(px(6.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.resources.theme.text_primary, 0.06)))
            .on_mouse_down(
                MouseButton::Left,
                self.picker_action_handler(presentation.command(option.action)),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(self.resources.theme.text_primary))
                    .child(label),
            )
            .when(option.checked, |row| {
                row.child(
                    div()
                        .text_size(px(14.0))
                        .text_color(rgb(self.resources.theme.text_primary))
                        .child("✓"),
                )
            })
    }
}

struct PickerSubmenuOption {
    label: String,
    checked: bool,
    action: PageMentionPickerAction,
}

fn picker_submenu_position(
    picker: &PageMentionPickerState,
    submenu: PageMentionPickerSubmenu,
) -> gpui::Point<Pixels> {
    let row_index = match submenu {
        PageMentionPickerSubmenu::DateFormat => 1.0,
        PageMentionPickerSubmenu::TimeFormat => 3.0,
        PageMentionPickerSubmenu::Remind if picker.draft.has_time() => 4.0,
        PageMentionPickerSubmenu::Remind => 3.0,
    };
    point(
        picker.anchor.left() + px(PAGE_MENTION_PICKER_WIDTH + 4.0),
        picker.anchor.bottom() + px(OPTION_ROWS_TOP + OPTION_ROW_STRIDE * row_index),
    )
}

fn picker_submenu_options(
    picker: &PageMentionPickerState,
    submenu: PageMentionPickerSubmenu,
) -> Vec<PickerSubmenuOption> {
    match submenu {
        PageMentionPickerSubmenu::DateFormat => PageMentionDateFormat::ALL
            .iter()
            .map(|format| PickerSubmenuOption {
                label: format.menu_label().to_string(),
                checked: *format == picker.draft.date_format,
                action: PageMentionPickerAction::SetDateFormat(*format),
            })
            .collect(),
        PageMentionPickerSubmenu::TimeFormat => PageMentionTimeFormat::ALL
            .iter()
            .map(|format| PickerSubmenuOption {
                label: format.menu_label().to_string(),
                checked: *format == picker.draft.time_format,
                action: PageMentionPickerAction::SetTimeFormat(*format),
            })
            .collect(),
        PageMentionPickerSubmenu::Remind => picker_reminder_options(picker),
    }
}

type ReminderMenuOption = (&'static str, Option<PageMentionReminder>);

fn picker_reminder_options(picker: &PageMentionPickerState) -> Vec<PickerSubmenuOption> {
    let choices: Vec<ReminderMenuOption> = if picker.draft.has_time() {
        PageMentionReminder::time_options().into_iter().collect()
    } else {
        PageMentionReminder::day_options().into_iter().collect()
    };
    choices
        .into_iter()
        .map(|(label, reminder)| PickerSubmenuOption {
            label: label.to_string(),
            checked: reminder == picker.draft.reminder,
            action: PageMentionPickerAction::SetReminder(reminder),
        })
        .collect()
}
