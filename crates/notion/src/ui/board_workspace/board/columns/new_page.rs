use super::{
    alpha, div, px, relative, rgb, rgba, AnyElement, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Styled, Tone,
};
use super::{BoardColumnAction, BoardColumnsRenderer};
use gpui::App;

impl BoardColumnsRenderer<'_> {
    pub(crate) fn render_new_page_affordance(
        &self,
        column_index: usize,
        tone: Tone,
        _cx: &mut App,
    ) -> AnyElement {
        (div()
            .id(format!("new-page-row-{column_index}"))
            .h(px(40.0))
            .w_full()
            .px(px(10.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(tone.action_border(self.appearance_mode)))
            .bg(alpha(self.theme.app_bg, 0.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .gap(px(9.0))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                    BoardColumnAction::CreatePage(column_index)
                }),
            )
            .child(
                (div()
                    .id(format!("new-page-plus-{column_index}"))
                    .flex_none()
                    .text_size(px(20.0))
                    .line_height(px(16.0))
                    .text_color(rgb(tone.action_text_color()))
                    .mt(px(-1.0))
                    .child("+"))
                .into_any_element(),
            )
            .child(
                (div()
                    .id(format!("new-page-text-{column_index}"))
                    .text_size(px(15.0))
                    .line_height(relative(1.2))
                    .text_color(rgb(tone.action_text_color()))
                    .child("New page"))
                .into_any_element(),
            ))
        .into_any_element()
    }
}
