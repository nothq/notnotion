use super::renderer::InlineToolbarRenderer;
use super::{
    alpha, div, px, rgb, AnyElement, Bounds, FontWeight, InlineToolbarAction, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Role,
    StatefulInteractiveElement, Styled, INLINE_DIALOG_MARGIN,
};

impl InlineToolbarRenderer {
    pub(super) fn render_inline_templates_dialog(
        &self,
        anchor: Bounds<Pixels>,
        viewport_width: f32,
    ) -> AnyElement {
        let width = 360.0;
        let left = (anchor.right().as_f32() - width).clamp(
            INLINE_DIALOG_MARGIN,
            (viewport_width - width - INLINE_DIALOG_MARGIN).max(INLINE_DIALOG_MARGIN),
        );
        self.inline_dialog_surface(left, 82.0, width, 104.0)
            .px(px(16.0))
            .pt(px(12.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .child(self.render_inline_templates_header())
            .child(self.render_inline_templates_description())
            .child(self.render_inline_new_template_button())
            .into_any_element()
    }

    fn render_inline_templates_header(&self) -> AnyElement {
        div()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .child(format!("Templates for  {}", self.database_title))
            .into_any_element()
    }

    fn render_inline_templates_description(&self) -> AnyElement {
        div()
            .mt(px(2.0))
            .text_size(px(12.0))
            .line_height(px(17.0))
            .text_color(rgb(self.theme.text_muted))
            .child("Create a reusable page template for this database.")
            .into_any_element()
    }

    fn render_inline_new_template_button(&self) -> AnyElement {
        div()
            .id("notion-inline-new-template")
            .mt(px(8.0))
            .h(px(28.0))
            .rounded(px(6.0))
            .role(Role::Button)
            .aria_label("New template")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    InlineToolbarAction::Dismiss
                }),
            )
            .child("+")
            .child("New template")
            .into_any_element()
    }

    pub(super) fn inline_dialog_action_button(
        &self,
        label: &'static str,
        primary: bool,
    ) -> AnyElement {
        div()
            .id(if primary {
                "notion-inline-automation-enable"
            } else {
                "notion-inline-automation-cancel"
            })
            .h(px(32.0))
            .px(px(14.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if primary {
                alpha(0x2783de, 1.0)
            } else {
                alpha(self.theme.text_primary, 0.14)
            })
            .bg(rgb(if primary {
                0x2783de
            } else {
                self.theme.elevated_surface_bg
            }))
            .role(Role::Button)
            .aria_label(label)
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(if primary {
                0xffffff
            } else {
                self.theme.text_primary
            }))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    InlineToolbarAction::Dismiss
                }),
            )
            .child(label)
            .into_any_element()
    }
}
