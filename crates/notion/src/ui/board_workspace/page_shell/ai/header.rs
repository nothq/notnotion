use super::{NotionAiAction, NotionAiRenderer};
use gpui::App;
use gpui::{ClickEvent, StatefulInteractiveElement};

use super::super::{
    alpha, div, img, px, render_svg_image, rgb, svg_from_body, AnyElement, Div, FluentBuilder,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled,
};

const AI_TEXT_PRIMARY: u32 = 0x2c2c2b;
const AI_HEADER_ICON: u32 = 0x383836;

impl NotionAiRenderer<'_> {
    pub(super) fn render_notion_ai_panel_header(&self, cx: &mut App) -> Div {
        div()
            .relative()
            .h(px(44.0))
            .w_full()
            .px(px(12.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .ml(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(AI_TEXT_PRIMARY))
                    .child("New AI chat")
                    .child(self.render_notion_ai_chevron(cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(self.render_notion_ai_new_chat_button(cx))
                    .child(self.render_notion_ai_mode_button(cx))
                    .child(self.render_notion_ai_hide_button(cx)),
            )
            .when(self.state.mode_menu_open, |this| {
                this.child(self.render_notion_ai_mode_menu(cx))
            })
    }

    fn render_notion_ai_chevron(&self, cx: &mut App) -> AnyElement {
        let body = format!(
            r##"<path d="m12.76 6.52-4.32 4.32a.62.62 0 0 1-.44.18.62.62 0 0 1-.44-.18L3.24 6.52a.63.63 0 0 1 0-.88c.24-.24.64-.24.88 0L8 9.52l3.88-3.88c.24-.24.64-.24.88 0s.24.64 0 .88" fill="#{AI_TEXT_PRIMARY:06x}"/>"##
        );
        img(render_svg_image(svg_from_body("3.06 0 9.88 16", body), cx))
            .size(px(16.0))
            .into_any_element()
    }

    fn render_notion_ai_new_chat_button(&self, cx: &mut App) -> AnyElement {
        let body = format!(
            r##"<path d="M16.938 9.353c0-2.97-2.539-5.54-6.545-5.697L10 3.648c-4.232 0-6.938 2.638-6.938 5.705 0 1.437.583 2.752 1.617 3.759a.63.63 0 0 1 .18.547 7.3 7.3 0 0 1-.89 2.528c1.108-.13 2.12-.614 3.01-1.344l.063-.045a.63.63 0 0 1 .505-.072 9 9 0 0 0 2.454.333l.392-.008c4.006-.158 6.545-2.727 6.545-5.698m1.25 0c0 3.803-3.234 6.766-7.747 6.947l-.44.01c-.88 0-1.712-.107-2.485-.3-1.349 1.022-2.985 1.619-4.826 1.428a.625.625 0 0 1-.406-1.033c.712-.818 1.096-1.737 1.284-2.642-1.116-1.198-1.756-2.734-1.756-4.41 0-3.926 3.447-6.955 8.189-6.955l.44.009c4.512.181 7.747 3.143 7.747 6.946" fill="#{AI_HEADER_ICON:06x}"/><path d="M10 5.875c.346 0 .626.28.626.625v2.375H13a.625.625 0 1 1 0 1.25h-2.375v2.374a.625.625 0 1 1-1.25 0v-2.375H7a.625.625 0 1 1 0-1.25h2.375V6.5c0-.346.28-.625.626-.625" fill="#{AI_HEADER_ICON:06x}"/>"##
        );
        div()
            .id("notion-ai-new-chat")
            .size(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2a1c00, 0.055)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| NotionAiAction::NewChat),
            )
            .child(img(render_svg_image(svg_from_body("0 0 20 20", body), cx)).size(px(20.0)))
            .into_any_element()
    }

    fn render_notion_ai_hide_button(&self, cx: &mut App) -> AnyElement {
        let body = format!(
            r##"<path d="m5.492 4.158 5.4 5.4a.625.625 0 0 1 0 .884l-5.4 5.4a.625.625 0 1 1-.884-.884L9.566 10 4.608 5.042a.625.625 0 1 1 .884-.884" fill="#{AI_HEADER_ICON:06x}"/><path d="m16.392 10.442-5.4 5.4a.625.625 0 0 1-.884-.884L15.066 10l-4.958-4.958a.625.625 0 0 1 .884-.884l5.4 5.4a.625.625 0 0 1 0 .884" fill="#{AI_HEADER_ICON:06x}"/>"##
        );
        div()
            .id("notion-ai-hide-chat")
            .size(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2a1c00, 0.055)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| NotionAiAction::Hide),
            )
            .child(img(render_svg_image(svg_from_body("0 0 20 20", body), cx)).size(px(20.0)))
            .into_any_element()
    }
}

impl NotionAiRenderer<'_> {
    fn render_notion_ai_mode_button(&self, cx: &mut App) -> AnyElement {
        let body = super::mode_menu::notion_ai_mode_icon_body(self.state.mode);
        div()
            .id("notion-ai-switch-mode")
            .size(px(28.0))
            .rounded(px(6.0))
            .when(self.state.mode_menu_open, |this| {
                this.bg(alpha(0x211b17, 0.05098))
            })
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2a1c00, 0.055)))
            .on_click(self.actions.listener(|_: &ClickEvent, _, cx| {
                cx.stop_propagation();
                NotionAiAction::OpenModeMenu
            }))
            .child(img(render_svg_image(svg_from_body("0 0 20 20", body), cx)).size(px(20.0)))
            .into_any_element()
    }
}
