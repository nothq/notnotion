use super::renderer::InlineToolbarRenderer;
use super::{
    alpha, div, inline_toolbar_shadow, px, rgb, Div, InteractiveElement, MouseButton,
    MouseDownEvent, Role, StatefulInteractiveElement, Styled,
};

impl InlineToolbarRenderer {
    pub(super) fn inline_dialog_surface(
        &self,
        left: f32,
        top: f32,
        width: f32,
        height: f32,
    ) -> gpui::Stateful<Div> {
        div()
            .id("notion-inline-toolbar-dialog")
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(width))
            .h(px(height))
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.10))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(inline_toolbar_shadow(self.appearance_mode))
            .role(Role::Dialog)
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
            })
    }
}
