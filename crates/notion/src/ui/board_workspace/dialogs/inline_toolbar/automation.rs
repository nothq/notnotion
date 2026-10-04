use super::renderer::InlineToolbarRenderer;
use super::{alpha, div, px, rgb, AnyElement, Div, FontWeight, IntoElement, ParentElement, Styled};

impl InlineToolbarRenderer {
    pub(super) fn render_inline_automation_dialog(&self, viewport_width: f32) -> AnyElement {
        self.inline_dialog_surface((viewport_width - 500.0) / 2.0, 88.0, 500.0, 600.0)
            .p(px(16.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(61.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(alpha(self.theme.text_primary, 0.10))
                    .px(px(12.0))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .text_size(px(13.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child("For all pages in")
                    .child(
                        div()
                            .mt(px(3.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(self.theme.text_primary))
                            .child(self.database_title.clone()),
                    ),
            )
            .child(self.inline_automation_section("When", "New trigger"))
            .child(self.inline_automation_section("Do", "New action"))
            .child(
                div()
                    .mt_auto()
                    .h(px(36.0))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .gap(px(8.0))
                    .child(self.inline_dialog_action_button("Cancel", false))
                    .child(self.inline_dialog_action_button("Enable", true)),
            )
            .into_any_element()
    }

    fn inline_automation_section(&self, label: &'static str, action: &'static str) -> Div {
        div()
            .mt(px(24.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_primary))
                    .child(label),
            )
            .child(
                div()
                    .h(px(44.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(alpha(self.theme.text_primary, 0.10))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(14.0))
                    .text_color(rgb(self.theme.text_primary))
                    .child("+")
                    .child(action),
            )
    }
}
