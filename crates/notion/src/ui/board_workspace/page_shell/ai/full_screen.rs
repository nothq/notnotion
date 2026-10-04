use super::super::{
    div, img, notion_ai_face_image, px, rgb, AnyElement, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled,
};
use super::{NotionAiAction, NotionAiRenderer};
use gpui::App;
use gpui_components::backdrop::blocking_backdrop;

const FULL_SCREEN_CONTENT_TOP: f32 = 314.0;

impl NotionAiRenderer<'_> {
    pub(super) fn render_notion_ai_full_screen(&self, cx: &mut App) -> AnyElement {
        blocking_backdrop(
            div()
                .absolute()
                .top(px(0.0))
                .left(px(0.0))
                .size_full()
                .bg(rgb(self.app_bg))
                .flex()
                .flex_col()
                .items_center()
                .pt(px(FULL_SCREEN_CONTENT_TOP))
                .child(img(notion_ai_face_image()).size(px(66.0)))
                .child(
                    div()
                        .mt(px(24.0))
                        .text_size(px(32.0))
                        .line_height(px(38.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x2c2c2b))
                        .child("How can I help you today?"),
                )
                .child(
                    div()
                        .mt(px(20.0))
                        .child(self.render_notion_ai_full_screen_composer(cx)),
                )
                .child(self.render_notion_ai_full_screen_actions()),
            cx,
        )
        .id("notion-ai-full-screen")
        .into_any_element()
    }

    fn render_notion_ai_full_screen_actions(&self) -> AnyElement {
        let actions = [
            ("▦", "Create Slides"),
            ("▤", "Spreadsheets"),
            ("▣", "Research"),
            ("✣", "Visualize"),
        ];
        div()
            .mt(px(28.0))
            .flex()
            .items_center()
            .gap(px(34.0))
            .children(
                actions
                    .into_iter()
                    .enumerate()
                    .map(|(index, (icon, label))| {
                        let prompt = label.to_string();
                        div()
                            .id(format!("notion-ai-full-screen-action-{index}"))
                            .h(px(28.0))
                            .px(px(4.0))
                            .rounded(px(6.0))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                                    NotionAiAction::SetPrompt(prompt.clone())
                                }),
                            )
                            .text_size(px(14.0))
                            .line_height(px(20.0))
                            .text_color(rgb(0x8e8b86))
                            .child(div().text_size(px(18.0)).child(icon))
                            .child(label)
                    }),
            )
            .into_any_element()
    }
}
