use super::{
    alpha, div, img, px, rgb, AnyElement, Div, FontWeight, InlineDatabaseToolbarTarget,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled,
    ToolbarDialogKind,
};
use super::{App, BoardToolbarAction, BoardToolbarRenderer};

impl BoardToolbarRenderer<'_> {
    pub(crate) fn render_primary_new_button(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> Div {
        div().ml(px(4.0)).child(
            (div()
                .h(px(28.0))
                .rounded(px(6.0))
                .bg(rgb(0x2783de))
                .overflow_hidden()
                .flex()
                .items_center()
                .child(self.render_primary_new_button_label())
                .child(self.render_primary_new_button_chevron_cell(inline_target, cx)))
            .into_any_element(),
        )
    }

    fn render_primary_new_button_label(&self) -> AnyElement {
        (div()
            .px(px(8.0))
            .h_full()
            .bg(rgb(0x2783de))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| BoardToolbarAction::CreatePage),
            )
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xf3f9fd))
            .child((div().child("New")).into_any_element()))
        .into_any_element()
    }

    fn render_primary_new_button_chevron_cell(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &mut App,
    ) -> AnyElement {
        let parent_surface = inline_target.map(|target| target.parent_surface.clone());
        (div()
            .w(px(24.0))
            .h_full()
            .relative()
            .bg(rgb(0x2783de))
            .border_l_1()
            .border_color(alpha(0xfcfeff, 0.12))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    BoardToolbarAction::OpenDialog {
                        dialog: ToolbarDialogKind::Templates,
                        parent: parent_surface.clone(),
                    }
                }),
            )
            .child(self.render_primary_new_button_chevron(cx)))
        .into_any_element()
    }

    fn render_primary_new_button_chevron(&self, cx: &mut App) -> AnyElement {
        (img(self.icons.new_button_chevron.render(cx))
            .w(px(14.0))
            .h(px(14.0))
            .mt(px(2.0)))
        .into_any_element()
    }
}
