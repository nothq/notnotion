use super::super::{
    alpha, div, img, point, px, render_svg_image, rgb, svg_from_body, AnyElement, BoxShadow,
    FluentBuilder, InteractiveElement, IntoElement, NotionAiMode, ParentElement,
    StatefulInteractiveElement, Styled,
};
use super::{NotionAiAction, NotionAiRenderer};
use gpui::App;
use gpui::ClickEvent;
use gpui_components::backdrop::ClickAwayBoundary;

const AI_TEXT_PRIMARY: u32 = 0x2c2c2b;
const MODE_MENU_WIDTH: f32 = 220.0;

impl NotionAiRenderer<'_> {
    pub(super) fn render_notion_ai_mode_menu(&self, cx: &mut App) -> AnyElement {
        let menu = div()
            .id("notion-ai-mode-menu")
            .size_full()
            .overflow_hidden()
            .rounded(px(10.0))
            .bg(rgb(self.app_bg))
            .shadow(notion_ai_mode_menu_shadow())
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(65.0))
                    .p(px(4.0))
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(self.render_notion_ai_mode_row(NotionAiMode::Sidebar, cx))
                    .child(self.render_notion_ai_mode_row(NotionAiMode::Floating, cx)),
            )
            .child(
                div()
                    .mx(px(12.0))
                    .h(px(1.0))
                    .flex_none()
                    .bg(alpha(0x2a1c00, 0.07)),
            )
            .child(
                div()
                    .mt(px(1.0))
                    .h(px(36.0))
                    .p(px(4.0))
                    .child(self.render_notion_ai_mode_row(NotionAiMode::FullScreen, cx)),
            );
        ClickAwayBoundary::new()
            .dismissible_with_handler(
                div()
                    .absolute()
                    .right(px(12.0))
                    .top(px(36.0))
                    .w(px(MODE_MENU_WIDTH))
                    .h(px(102.0))
                    .child(menu),
                self.actions
                    .listener(|_, _, _| NotionAiAction::DismissModeMenu),
                cx,
            )
            .into_any_element()
    }
}

impl NotionAiRenderer<'_> {
    fn render_notion_ai_mode_row(&self, mode: NotionAiMode, cx: &mut App) -> AnyElement {
        let active = self.state.mode_menu_active == mode;
        div()
            .id(notion_ai_mode_row_id(mode))
            .w_full()
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .when(active, |this| this.bg(alpha(0x211b17, 0.05098)))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .on_hover(self.actions.listener(move |hovered: &bool, _, _| {
                NotionAiAction::HoverMode {
                    mode,
                    hovered: *hovered,
                }
            }))
            .on_click(self.actions.listener(move |_: &ClickEvent, _, cx| {
                cx.stop_propagation();
                NotionAiAction::SelectMode(mode)
            }))
            .child(notion_ai_mode_icon(mode, cx))
            .child(
                div()
                    .flex_grow(1.0)
                    .text_size(px(14.0))
                    .line_height(px(16.8))
                    .text_color(rgb(AI_TEXT_PRIMARY))
                    .child(notion_ai_mode_label(mode)),
            )
            .when(self.state.mode == mode, |this| {
                this.child(notion_ai_selected_check(cx))
            })
            .into_any_element()
    }
}

pub(super) fn notion_ai_mode_icon_body(mode: NotionAiMode) -> String {
    let inset = match mode {
        NotionAiMode::Sidebar => {
            "M10.392 6.125a.5.5 0 0 0-.5.5v6.75a.5.5 0 0 0 .5.5h4.683a.5.5 0 0 0 .5-.5v-6.75a.5.5 0 0 0-.5-.5z"
        }
        NotionAiMode::Floating => {
            "M11.93 9.125a.5.5 0 0 0-.5.5v3.75a.5.5 0 0 0 .5.5h3.145a.5.5 0 0 0 .5-.5v-3.75a.5.5 0 0 0-.5-.5z"
        }
        NotionAiMode::FullScreen => {
            "M4.93 6.125a.5.5 0 0 0-.5.5v6.75a.5.5 0 0 0 .5.5h10.145a.5.5 0 0 0 .5-.5v-6.75a.5.5 0 0 0-.5-.5z"
        }
    };
    format!(
        r##"<path d="{inset}" fill="#{AI_TEXT_PRIMARY:06x}"/><path d="M4.5 4.125A2.125 2.125 0 0 0 2.375 6.25v7.5c0 1.174.951 2.125 2.125 2.125h11a2.125 2.125 0 0 0 2.125-2.125v-7.5A2.125 2.125 0 0 0 15.5 4.125zM3.625 6.25c0-.483.392-.875.875-.875h11c.483 0 .875.392.875.875v7.5a.875.875 0 0 1-.875.875h-11a.875.875 0 0 1-.875-.875z" fill="#{AI_TEXT_PRIMARY:06x}"/>"##
    )
}

fn notion_ai_mode_icon(mode: NotionAiMode, cx: &mut App) -> AnyElement {
    img(render_svg_image(
        svg_from_body("0 0 20 20", notion_ai_mode_icon_body(mode)),
        cx,
    ))
    .size(px(20.0))
    .into_any_element()
}

fn notion_ai_selected_check(cx: &mut App) -> AnyElement {
    let body = format!(
        r##"<path d="M11.834 3.309a.625.625 0 0 1 1.072.642l-5.244 8.74a.625.625 0 0 1-1.01.085L3.155 8.699a.626.626 0 0 1 .95-.813l2.93 3.419z" fill="#{AI_TEXT_PRIMARY:06x}"/>"##
    );
    img(render_svg_image(svg_from_body("0 0 16 16", body), cx))
        .size(px(16.0))
        .into_any_element()
}

fn notion_ai_mode_menu_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.05),
            offset: point(px(0.0), px(20.0)),
            blur_radius: px(24.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: point(px(0.0), px(5.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x2a1c00, 0.07),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
    ]
}

fn notion_ai_mode_label(mode: NotionAiMode) -> &'static str {
    match mode {
        NotionAiMode::Sidebar => "Sidebar",
        NotionAiMode::Floating => "Floating",
        NotionAiMode::FullScreen => "Full screen",
    }
}

fn notion_ai_mode_row_id(mode: NotionAiMode) -> &'static str {
    match mode {
        NotionAiMode::Sidebar => "notion-ai-mode-sidebar",
        NotionAiMode::Floating => "notion-ai-mode-floating",
        NotionAiMode::FullScreen => "notion-ai-mode-full-screen",
    }
}
