use gpui::{
    AnyElement, Div, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Role, StatefulInteractiveElement, Styled,
};

use crate::ui::{
    board_workspace::{
        alpha, div,
        page::editor::{PageLinkIconPickerState, PageLinkIconView, PageLinkNamedIconPreference},
        px, rgb,
    },
    NotionNamedIconColor,
};

use super::super::super::{
    PageLinkIconAction, PageLinkIconControlAction, PageLinkIconPickerAction,
};
use super::super::catalog::skin_tone_glyph;

impl PageLinkIconView {
    pub(in crate::ui::board_workspace::page::editor::page_link_icon::render) fn render_page_link_icon_picker_named_color_button(
        &self,
        state: &PageLinkIconPickerState,
    ) -> AnyElement {
        let preference = state.named_icon_preference;
        let color = preference.preview_color();
        div()
            .relative()
            .size(px(28.0))
            .flex_none()
            .child(
                div()
                    .id("notion-page-link-icon-picker-named-color")
                    .role(Role::Button)
                    .aria_label(named_color_button_label(preference))
                    .focusable()
                    .tab_stop(true)
                    .size(px(28.0))
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(alpha(self.theme.text_primary, 0.10))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .on_mouse_down(MouseButton::Left, self.named_color_button_click())
                    .on_key_down(self.named_color_button_key_down())
                    .child(self.render_named_color_button_indicator(preference, color)),
            )
            .into_any_element()
    }

    fn render_named_color_button_indicator(
        &self,
        preference: PageLinkNamedIconPreference,
        color: NotionNamedIconColor,
    ) -> Div {
        div()
            .size(px(14.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(rgb(0x7f7f7f))
            .bg(match preference {
                PageLinkNamedIconPreference::AskEveryTime => alpha(self.theme.text_primary, 0.20),
                PageLinkNamedIconPreference::Color(_) => {
                    rgb(color.picker_color(self.appearance_mode)).into()
                }
            })
    }

    fn named_color_button_click(
        &self,
    ) -> impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(|this, _: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::ToggleNamedColorMenu),
                window,
                cx,
            );
        })
    }

    fn named_color_button_key_down(
        &self,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(|this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key.as_str() == "escape" {
                window.prevent_default();
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Picker(PageLinkIconPickerAction::DismissLayer),
                    window,
                    cx,
                );
                return;
            }
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::ToggleNamedColorMenu),
                window,
                cx,
            );
        })
    }

    pub(in crate::ui::board_workspace::page::editor::page_link_icon::render) fn render_page_link_icon_picker_skin_tone_button(
        &self,
        state: &PageLinkIconPickerState,
    ) -> AnyElement {
        div()
            .relative()
            .size(px(28.0))
            .flex_none()
            .child(
                div()
                    .id("notion-page-link-icon-picker-skin-tone")
                    .role(Role::Button)
                    .aria_label("Select emoji skin tone")
                    .focusable()
                    .tab_stop(true)
                    .size(px(28.0))
                    .rounded(px(5.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
                    .text_size(px(19.0))
                    .on_mouse_down(MouseButton::Left, self.skin_tone_button_click())
                    .on_key_down(self.skin_tone_button_key_down())
                    .child(skin_tone_glyph(state.skin_tone)),
            )
            .into_any_element()
    }

    fn skin_tone_button_click(
        &self,
    ) -> impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(|this, _: &MouseDownEvent, window, cx| {
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::ToggleSkinToneMenu),
                window,
                cx,
            );
        })
    }

    fn skin_tone_button_key_down(
        &self,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(|this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key.as_str() == "escape" {
                window.prevent_default();
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Picker(PageLinkIconPickerAction::DismissLayer),
                    window,
                    cx,
                );
                return;
            }
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Controls(PageLinkIconControlAction::ToggleSkinToneMenu),
                window,
                cx,
            );
        })
    }
}

fn named_color_button_label(preference: PageLinkNamedIconPreference) -> String {
    match preference {
        PageLinkNamedIconPreference::AskEveryTime => {
            "Select icon color, no color selected".to_string()
        }
        PageLinkNamedIconPreference::Color(color) => {
            format!("Select icon color, {} selected", color.label())
        }
    }
}
