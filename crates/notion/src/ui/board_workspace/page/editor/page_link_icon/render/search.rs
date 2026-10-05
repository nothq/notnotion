use gpui::App;
use gpui::{
    AnyElement, Div, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Role, StatefulInteractiveElement, Styled,
};

use super::super::{PageLinkIconAction, PageLinkIconSelectionAction};
use crate::ui::board_workspace::{
    alpha, div, img,
    page::editor::{PageLinkIconPickerState, PageLinkIconPickerTab, PageLinkIconView},
    px, rgb, rgba,
};

impl PageLinkIconView {
    pub(super) fn render_page_link_icon_picker_search(
        &self,
        state: &PageLinkIconPickerState,
        cx: &mut App,
    ) -> impl IntoElement {
        div()
            .id("notion-page-link-icon-picker-search-row")
            .h(px(36.0))
            .mt(px(4.0))
            .mb(px(10.0))
            .px(px(8.0))
            .py(px(4.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(self.render_page_link_icon_picker_search_field(state, cx))
            .child(self.render_page_link_icon_picker_search_controls(state, cx))
    }

    fn render_page_link_icon_picker_search_field(
        &self,
        state: &PageLinkIconPickerState,
        cx: &mut App,
    ) -> Div {
        let search_focus = state.search_input.read(cx).focus_handle_clone();
        div()
            .w(px(322.0))
            .h(px(28.0))
            .flex_none()
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.0))
            .track_focus(&search_focus)
            .focus(|style| style.border_color(rgb(0x2383e2)))
            .flex()
            .items_center()
            .px(px(6.0))
            .bg(rgba(self.theme.dialog_input_bg))
            .child(
                img(self.icons.toolbar_search.render(cx))
                    .mr(px(6.0))
                    .size(px(16.0)),
            )
            .child(
                div()
                    .h(px(20.0))
                    .flex_grow(1.0)
                    .child(state.search_input.clone()),
            )
    }

    fn render_page_link_icon_picker_search_controls(
        &self,
        state: &PageLinkIconPickerState,
        cx: &mut App,
    ) -> Div {
        div()
            .w(px(62.0))
            .h(px(28.0))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_page_link_icon_picker_random_button(cx))
            .child(match state.tab {
                PageLinkIconPickerTab::Emoji => {
                    self.render_page_link_icon_picker_skin_tone_button(state)
                }
                PageLinkIconPickerTab::Icons => {
                    self.render_page_link_icon_picker_named_color_button(state)
                }
                PageLinkIconPickerTab::Upload => div().size(px(28.0)).into_any_element(),
            })
    }

    fn render_page_link_icon_picker_random_button(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-page-link-icon-picker-random")
            .role(Role::Button)
            .aria_label("Random")
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
            .on_mouse_down(
                MouseButton::Left,
                self.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.emit(
                        PageLinkIconAction::Selection(PageLinkIconSelectionAction::SetRandomIcon),
                        window,
                        cx,
                    );
                }),
            )
            .on_key_down(self.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Selection(PageLinkIconSelectionAction::SetRandomIcon),
                    window,
                    cx,
                );
            }))
            .child(img(self.icons.page_link_picker_random.render(cx)).size(px(16.0)))
            .into_any_element()
    }
}
