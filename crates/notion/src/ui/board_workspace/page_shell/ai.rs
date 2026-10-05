use super::{
    alpha, div, img, notion_ai_button_image, notion_ai_face_image, point, px, rgb, AnyElement,
    BoxShadow, Div, FontWeight, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    NotionAiMode, ParentElement, Styled,
};
use crate::ui::view_actions::ViewActionSink;
use gpui::{App, RenderImage};

mod actions;
mod composer;
mod full_screen;
mod header;
mod host;
mod keyboard;
mod mode_menu;
mod suggestions;

pub(super) const AI_PANEL_WIDTH_RATIO: f32 = 0.28;
pub(super) const AI_PANEL_MIN_WIDTH: f32 = 320.0;
pub(super) const AI_PANEL_MAX_WIDTH: f32 = 600.0;

use actions::NotionAiAction;

pub(super) struct NotionAiViewState<'a> {
    mode: NotionAiMode,
    mode_menu_open: bool,
    mode_menu_active: NotionAiMode,
    input: &'a str,
    input_active: bool,
    block_context: Option<&'a str>,
}

pub(super) struct NotionAiRenderer<'a> {
    state: NotionAiViewState<'a>,
    app_bg: u32,
    page_title: &'a str,
    page_icon: std::sync::Arc<RenderImage>,
    panel_width: f32,
    actions: ViewActionSink<NotionAiAction>,
}

impl NotionAiRenderer<'_> {
    pub(crate) fn render_notion_ai_button(&self) -> AnyElement {
        div()
            .id("notion-ai-button")
            .absolute()
            .right(px(16.0))
            .bottom(px(16.0))
            .size(px(40.0))
            .rounded_full()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| NotionAiAction::Open),
            )
            .child(img(notion_ai_button_image()).size_full())
            .into_any_element()
    }

    pub(crate) fn render_notion_ai_panel(&self, cx: &mut App) -> AnyElement {
        match self.state.mode {
            NotionAiMode::Sidebar => self.render_notion_ai_sidebar_panel(cx),
            NotionAiMode::Floating => self.render_notion_ai_floating_panel(cx),
            NotionAiMode::FullScreen => self.render_notion_ai_full_screen(cx),
        }
    }

    fn render_notion_ai_sidebar_panel(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-panel")
            .w(px(self.panel_width))
            .h_full()
            .min_h(px(0.0))
            .flex_none()
            .border_l_1()
            .border_color(alpha(0x2a1c00, 0.07))
            .bg(rgb(self.app_bg))
            .child(self.render_notion_ai_standard_panel_contents(cx))
            .into_any_element()
    }

    fn render_notion_ai_floating_panel(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-panel")
            .absolute()
            .right(px(15.0))
            .bottom(px(15.0))
            .w(px(450.0))
            .h(px(500.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(alpha(0x2a1c00, 0.07))
            .bg(rgb(self.app_bg))
            .shadow(notion_ai_floating_shadow())
            .child(self.render_notion_ai_standard_panel_contents(cx))
            .into_any_element()
    }

    fn render_notion_ai_standard_panel_contents(&self, cx: &mut App) -> Div {
        div()
            .size_full()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(self.render_notion_ai_panel_header(cx))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_h(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex_grow(1.0)
                            .min_h(px(0.0))
                            .px(px(16.0))
                            .pb(px(23.0))
                            .flex()
                            .flex_col()
                            .justify_end()
                            .child(img(notion_ai_face_image()).size(px(50.0)))
                            .child(
                                div()
                                    .ml(px(8.0))
                                    .mt(px(19.0))
                                    .text_size(px(17.0))
                                    .line_height(px(22.1))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0x2c2c2b))
                                    .child("How can I help you today?"),
                            )
                            .child(self.render_notion_ai_suggestions(cx)),
                    )
                    .child(self.render_notion_ai_composer(cx)),
            )
    }
}

fn notion_ai_floating_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.08),
            offset: point(px(0.0), px(18.0)),
            blur_radius: px(36.0),
            spread_radius: px(-8.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.05),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
            inset: false,
        },
    ]
}
