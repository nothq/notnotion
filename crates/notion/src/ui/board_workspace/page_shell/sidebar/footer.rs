use super::{
    alpha, div, point, px, rgb, AnyElement, AppearanceMode, BoxShadow, Div,
    DraggedNotionSidebarResize, FluentBuilder, FontWeight, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, NotionSidebarTab, NotionSidebarUpdate, ParentElement,
    SidebarAction, SidebarRenderer, StatefulInteractiveElement, Styled, SIDEBAR_FOOTER_HEIGHT,
    SIDEBAR_RESIZE_HITBOX_WIDTH,
};
use gpui::{App, AppContext};

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_footer(&self, cx: &mut App) -> Div {
        div()
            .h(px(SIDEBAR_FOOTER_HEIGHT))
            .flex_none()
            .pt(px(8.0))
            .px(px(16.0))
            .pb(px(16.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.render_notion_sidebar_new_chat(cx))
            .child(self.render_notion_sidebar_new_page(cx))
    }

    fn render_notion_sidebar_new_chat(&self, cx: &mut App) -> AnyElement {
        self.notion_sidebar_footer_button()
            .id("notion-sidebar-new-chat")
            .flex_grow(1.0)
            .min_w(px(0.0))
            .px(px(14.0))
            .justify_start()
            .gap(px(8.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, _| {
                    SidebarAction::Update(NotionSidebarUpdate::ActivateTab(NotionSidebarTab::Chat))
                }),
            )
            .child(self.render_notion_sidebar_builtin_icon_at_size("new-chat", 20.0, cx))
            .child(self.render_notion_sidebar_new_chat_label())
            .when(self.width >= 250.0, |this| {
                this.child(self.render_notion_sidebar_new_chat_shortcut())
            })
            .into_any_element()
    }

    fn render_notion_sidebar_new_chat_label(&self) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.notion_sidebar_top_text_color()))
            .child("New chat")
    }

    fn render_notion_sidebar_new_chat_shortcut(&self) -> Div {
        div()
            .flex_none()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(match self.appearance_mode {
                AppearanceMode::Light => 0xa19e99,
                AppearanceMode::Dark => 0x9b9a97,
            }))
            .child("⌘O")
    }

    fn render_notion_sidebar_new_page(&self, cx: &mut App) -> AnyElement {
        self.notion_sidebar_footer_button()
            .id("notion-sidebar-new-page")
            .w(px(40.0))
            .min_w(px(40.0))
            .max_w(px(40.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| SidebarAction::TogglePageMenu),
            )
            .child(self.render_notion_sidebar_builtin_icon_at_size("new-page", 20.0, cx))
            .into_any_element()
    }

    fn notion_sidebar_footer_button(&self) -> Div {
        div()
            .h(px(40.0))
            .rounded_full()
            .border_1()
            .border_color(alpha(0x211b17, 0.09))
            .bg(rgb(match self.appearance_mode {
                AppearanceMode::Light => 0xffffff,
                AppearanceMode::Dark => 0x2b2a29,
            }))
            .shadow(vec![BoxShadow {
                color: alpha(0x211b17, 0.08),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(3.0),
                spread_radius: px(-1.0),
                inset: false,
            }])
            .flex()
            .items_center()
            .justify_center()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
    }

    pub(super) fn render_notion_sidebar_resize_handle(&self, _cx: &mut App) -> AnyElement {
        div()
            .id("notion-sidebar-resize-handle")
            .absolute()
            .right(px(-SIDEBAR_RESIZE_HITBOX_WIDTH * 0.5))
            .top(px(0.0))
            .h_full()
            .w(px(SIDEBAR_RESIZE_HITBOX_WIDTH))
            .cursor_col_resize()
            .block_mouse_except_scroll()
            .on_drag(DraggedNotionSidebarResize, |dragged, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| *dragged)
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .occlude()
            .into_any_element()
    }
}
