use gpui::{px, ElementId, Hsla, MouseButton, MouseDownEvent, ParentElement, Role, Styled};

use super::super::highlights::{page_background_color, page_text_color};
use super::super::PAGE_TEXT_COLORS;
use crate::model::PageTextColor;
use crate::ui::{
    alpha, div, rgb, AnyElement, Div, FluentBuilder, InteractiveElement, IntoElement,
    StatefulInteractiveElement,
};

use super::{PageRichTextRenderer, ToolbarAction};

impl PageRichTextRenderer {
    pub(super) fn render_page_rich_text_color_menu(&self) -> AnyElement {
        self.page_rich_text_surface()
            .w(px(190.0))
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(self.render_page_rich_text_color_section("Color", false))
            .child(self.render_page_rich_text_color_section("Background", true))
            .into_any_element()
    }

    fn render_page_rich_text_color_section(&self, label: &'static str, background: bool) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child(label),
            )
            .child(div().flex().flex_wrap().gap(px(3.0)).children(
                PAGE_TEXT_COLORS.into_iter().map(|(color, color_label)| {
                    self.render_page_rich_text_swatch(color, color_label, background)
                }),
            ))
    }

    fn render_page_rich_text_swatch(
        &self,
        color: PageTextColor,
        label: &'static str,
        background: bool,
    ) -> AnyElement {
        let swatch = if background {
            page_background_color(color, self.appearance_mode)
                .unwrap_or_else(|| rgb(self.theme.elevated_surface_bg).into())
        } else {
            page_text_color(color, self.appearance_mode)
        };
        div()
            .id(ElementId::Name(
                format!(
                    "notion-rich-text-{}-{}",
                    if background { "background" } else { "color" },
                    label.to_lowercase()
                )
                .into(),
            ))
            .size(px(30.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label(format!(
                "{label} {}",
                if background { "background" } else { "text" }
            ))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    ToolbarAction::SetColor { color, background }
                }),
            )
            .child(self.render_page_rich_text_swatch_sample(swatch, background))
            .into_any_element()
    }

    fn render_page_rich_text_swatch_sample(&self, swatch: Hsla, background: bool) -> Div {
        div()
            .size(px(26.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.10))
            .bg(swatch)
            .when(!background, |sample| {
                sample
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.0))
                    .text_color(swatch)
                    .bg(rgb(self.theme.elevated_surface_bg))
                    .child("A")
            })
    }
}
