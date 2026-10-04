use gpui::{div, AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled};
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

use crate::ui::{alpha, SurfaceState};

impl SurfaceState {
    pub(crate) fn render_notion_comments_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let panel = self
            .comments
            .panel
            .as_ref()
            .expect("comments overlay requires panel state");
        div()
            .id("notion-comments-overlay")
            .absolute()
            .inset_0()
            .child(dismissible_backdrop(
                div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
                BackdropDismissal::new(|this: &mut Self, _, _, cx| {
                    this.comments.dismiss_notion_comments_panel(cx);
                }),
                cx,
            ))
            .child(panel.render_notion_comments_panel(
                self.theme,
                self.page_layout().consuming_ai_width() + 8.0,
                cx,
            ))
            .into_any_element()
    }
}
