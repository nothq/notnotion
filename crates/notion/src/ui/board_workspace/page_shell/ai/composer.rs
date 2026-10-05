use super::super::{
    alpha, div, img, point, px, render_svg_image, rgb, svg_from_body, AnyElement, BoxShadow, Div,
    FluentBuilder, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Styled,
};
use super::{NotionAiAction, NotionAiRenderer};
use gpui::App;

const AI_TEXT_PRIMARY: u32 = 0x2c2c2b;
const AI_TEXT_SECONDARY: u32 = 0x8e8b86;
const AI_TEXT_MUTED: u32 = 0xada9a3;

impl NotionAiRenderer<'_> {
    pub(super) fn render_notion_ai_composer(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-composer")
            .mx(px(16.0))
            .mb(px(16.0))
            .h(px(138.0))
            .flex_none()
            .rounded(px(16.0))
            .border_2()
            .border_color(rgb(if self.state.input_active {
                0x2383e2
            } else {
                0xe6e5e3
            }))
            .bg(rgb(self.app_bg))
            .shadow(notion_ai_composer_shadow())
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| NotionAiAction::ActivateInput),
            )
            .flex()
            .flex_col()
            .child(self.render_notion_ai_context_chip())
            .child(self.render_notion_ai_input())
            .child(self.render_notion_ai_composer_controls(cx))
            .into_any_element()
    }

    pub(super) fn render_notion_ai_full_screen_composer(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-full-screen-composer")
            .w(px(714.0))
            .h(px(112.0))
            .flex_none()
            .rounded(px(16.0))
            .border_2()
            .border_color(rgb(if self.state.input_active {
                0x2383e2
            } else {
                0xe6e5e3
            }))
            .bg(rgb(self.app_bg))
            .shadow(notion_ai_composer_shadow())
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| NotionAiAction::ActivateInput),
            )
            .flex()
            .flex_col()
            .child(self.render_notion_ai_full_screen_input())
            .child(self.render_notion_ai_composer_controls(cx))
            .into_any_element()
    }

    fn render_notion_ai_context_chip(&self) -> Div {
        div()
            .h(px(28.0))
            .flex()
            .gap(px(6.0))
            .mt(px(10.0))
            .ml(px(10.0))
            .child(self.render_notion_ai_context_chip_item(self.page_title.to_string()))
            .when_some(self.state.block_context, |row, block| {
                row.child(self.render_notion_ai_context_chip_item(block.to_string()))
            })
    }

    fn render_notion_ai_context_chip_item(&self, label: String) -> Div {
        div()
            .self_start()
            .max_w(px(240.0))
            .px(px(8.0))
            .rounded(px(14.0))
            .border_1()
            .border_color(alpha(0x2a1c00, 0.11))
            .flex()
            .items_center()
            .gap(px(5.0))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .text_color(rgb(AI_TEXT_SECONDARY))
            .child(img(self.page_icon.clone()).size(px(14.0)).opacity(0.55))
            .child(label)
    }

    fn render_notion_ai_composer_controls(&self, cx: &mut App) -> Div {
        div()
            .h(px(40.0))
            .px(px(8.0))
            .pt(px(4.0))
            .pb(px(8.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(self.render_notion_ai_add_context_button(cx))
                    .child(self.render_notion_ai_settings_button(cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(self.render_notion_ai_auto_chip(cx))
                    .child(self.render_notion_ai_voice_button(cx))
                    .child(self.render_notion_ai_submit_button(cx)),
            )
    }

    fn render_notion_ai_add_context_button(&self, cx: &mut App) -> AnyElement {
        let body = r##"<path d="M10 3.59a.66.66 0 0 1 .66.66v5.09h5.09a.66.66 0 0 1 0 1.32h-5.09v5.09a.66.66 0 0 1-1.32 0v-5.09H4.25a.66.66 0 0 1 0-1.32h5.09V4.25a.66.66 0 0 1 .66-.66" fill="#8e8b86"/>"##;
        self.render_notion_ai_composer_icon_button("notion-ai-add-context", body, 20.0, cx)
    }

    fn render_notion_ai_settings_button(&self, cx: &mut App) -> AnyElement {
        let body = r##"<path d="M3 7.375h6.829a2.501 2.501 0 0 0 4.842 0H17a.625.625 0 1 0 0-1.25h-2.329a2.501 2.501 0 0 0-4.842 0H3a.625.625 0 1 0 0 1.25M12.25 5.5a1.25 1.25 0 1 1 0 2.5 1.25 1.25 0 0 1 0-2.5" fill="#8e8b86"/><path fill-rule="evenodd" d="M7.75 15.75a2.5 2.5 0 0 0 2.421-1.875H17a.625.625 0 0 0 0-1.25h-6.829a2.5 2.5 0 0 0-4.842 0H3a.625.625 0 1 0 0 1.25h2.329A2.5 2.5 0 0 0 7.75 15.75m0-1.25a1.25 1.25 0 1 0 0-2.5 1.25 1.25 0 0 0 0 2.5" fill="#8e8b86"/>"##;
        self.render_notion_ai_composer_icon_button("notion-ai-settings", body, 20.0, cx)
    }

    fn render_notion_ai_voice_button(&self, cx: &mut App) -> AnyElement {
        let body = r##"<path d="M10 2.175A2.875 2.875 0 0 0 7.125 5.05v3.6a2.875 2.875 0 1 0 5.75 0v-3.6A2.875 2.875 0 0 0 10 2.175M8.375 5.05a1.625 1.625 0 0 1 3.25 0v3.6a1.625 1.625 0 1 1-3.25 0z" fill="#8e8b86"/><path d="M5.604 10.891a.625.625 0 1 0-1.028.71 6.58 6.58 0 0 0 4.799 2.82v1.929H5.95a.625.625 0 1 0 0 1.25h8.1a.625.625 0 0 0 0-1.25h-3.425v-1.93a6.58 6.58 0 0 0 4.799-2.818.625.625 0 1 0-1.029-.71 5.33 5.33 0 0 1-4.393 2.308h-.004a5.33 5.33 0 0 1-4.394-2.309" fill="#8e8b86"/>"##;
        self.render_notion_ai_composer_icon_button("notion-ai-voice", body, 20.0, cx)
    }

    fn render_notion_ai_auto_chip(&self, cx: &mut App) -> AnyElement {
        let chevron = r##"<path d="m6.75 5.25 3.25 3.5 3.25-3.5" fill="none" stroke="#8e8b86" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>"##;
        div()
            .id("notion-ai-auto")
            .w(px(55.0))
            .h(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(2.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2a1c00, 0.055)))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .text_color(rgb(AI_TEXT_SECONDARY))
            .child("Auto")
            .child(
                img(render_svg_image(svg_from_body("0 0 20 14", chevron), cx))
                    .w(px(12.0))
                    .h(px(10.0)),
            )
            .into_any_element()
    }

    fn render_notion_ai_composer_icon_button(
        &self,
        id: &'static str,
        body: &'static str,
        icon_size: f32,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(id)
            .size(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2a1c00, 0.055)))
            .child(img(render_svg_image(svg_from_body("0 0 20 20", body), cx)).size(px(icon_size)))
            .into_any_element()
    }
}

impl NotionAiRenderer<'_> {
    fn render_notion_ai_input(&self) -> Div {
        div()
            .h(px(60.0))
            .px(px(14.0))
            .pt(px(12.0))
            .text_size(px(14.0))
            .line_height(px(20.0))
            .text_color(rgb(if self.state.input.is_empty() {
                AI_TEXT_MUTED
            } else {
                AI_TEXT_PRIMARY
            }))
            .child(if self.state.input.is_empty() {
                "Do anything with AI…".to_string()
            } else {
                self.state.input.to_string()
            })
    }

    fn render_notion_ai_full_screen_input(&self) -> Div {
        div()
            .h(px(72.0))
            .px(px(18.0))
            .pt(px(17.0))
            .text_size(px(14.0))
            .line_height(px(20.0))
            .text_color(rgb(if self.state.input.is_empty() {
                AI_TEXT_MUTED
            } else {
                AI_TEXT_PRIMARY
            }))
            .child(if self.state.input.is_empty() {
                "Do anything with AI…".to_string()
            } else {
                self.state.input.to_string()
            })
    }

    fn render_notion_ai_submit_button(&self, cx: &mut App) -> AnyElement {
        let disabled = self.state.input.trim().is_empty();
        let icon_color = if disabled { 0xada9a3 } else { 0xffffff };
        let body = format!(
            r##"<path d="M12.33 7.33a.75.75 0 0 0 0-1.06l-3.8-3.8a.75.75 0 0 0-1.06 0l-3.8 3.8a.75.75 0 0 0 1.06 1.06l2.52-2.52V13a.75.75 0 0 0 1.5 0V4.81l2.52 2.52a.75.75 0 0 0 1.06 0" fill="#{icon_color:06x}"/>"##
        );
        div()
            .id("notion-ai-submit")
            .size(px(28.0))
            .rounded_full()
            .bg(if disabled {
                alpha(0x2a1c00, 0.07)
            } else {
                alpha(0x2383e2, 1.0)
            })
            .flex()
            .items_center()
            .justify_center()
            .when(disabled, |this| this.opacity(0.4))
            .child(img(render_svg_image(svg_from_body("0 0 16 16", body), cx)).size(px(16.0)))
            .into_any_element()
    }
}

fn notion_ai_composer_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: point(px(0.0), px(8.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(6.0),
            spread_radius: px(0.0),
            inset: false,
        },
    ]
}
