use super::{
    div, point, px, rgb, AnyElement, Context, Div, DragMoveEvent, DraggedNotionSidebarResize,
    FluentBuilder, InteractiveElement, IntoElement, MouseButton, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState, Window,
};
use crate::ui::surface::NotionChromeState;

mod layout;
use layout::NotionPageLayout;

mod navigation;
mod page_hydration;
mod sidebar_children;

use super::SidebarView;
pub(super) use sidebar_children::handle_sidebar_resource_event;

impl SurfaceState {
    pub(crate) fn render_notion_page_root(&self, cx: &mut Context<Self>) -> AnyElement {
        let page_main_pane = self.render_cached_notion_main_region(cx);
        let page_content = self.notion_page_root_content(page_main_pane, cx);
        (self
            .notion_page_root_frame(cx)
            .child(page_content)
            .when(self.notion_sidebar.inbox_menu_is_open(), |this| {
                this.child(SidebarView::render_inbox_menu_backdrop(self.viewport, cx))
            })
            .when(self.notion_chrome.notion_ai_open, |this| {
                this.child(self.render_notion_ai_panel(cx))
            })
            .when(!self.notion_chrome.notion_ai_open, |this| {
                this.child(self.render_notion_ai_button(cx))
            })
            .when(self.notion_chrome.notion_page_menu_open, |this| {
                this.child(
                    self.page_shell_body_renderer(cx)
                        .render_notion_page_create_menu(cx),
                )
            })
            .when(self.notion_chrome.notion_search_open, |this| {
                this.child(self.render_notion_search_overlay(cx))
            })
            .when(self.page_documents.selected_page.is_some(), |this| {
                this.child(self.render_selected_page_overlay(cx))
            })
            .when_some(self.render_page_mention_picker(cx), |this, picker| {
                this.child(picker)
            })
            .when_some(self.render_page_rich_text_toolbar(cx), |this, toolbar| {
                this.child(toolbar)
            })
            .when(self.notion_chrome.inline_toolbar_dialog.is_some(), |this| {
                this.child(self.render_inline_toolbar_dialog_overlay(cx))
            })
            .when(self.notion_chrome.date_undated_dialog.is_some(), |this| {
                this.child(self.render_date_undated_dialog_overlay(cx))
            })
            .when(
                self.notion_chrome.inline_database_view_menu.is_some(),
                |this| this.child(self.render_inline_database_view_menu_overlay(cx)),
            )
            .when(self.notion_chrome.ai_autofill_dialog.is_some(), |this| {
                this.child(self.render_ai_autofill_overlay(cx))
            })
            .when(self.notion_chrome.top_bar_popover_open(), |this| {
                this.child(self.render_top_bar_popover(cx))
            })
            .when(self.comments.panel.is_some(), |this| {
                this.child(self.render_notion_comments_overlay(cx))
            }))
        .into_any_element()
    }

    fn notion_page_root_content(&self, page_main_pane: AnyElement, cx: &mut Context<Self>) -> Div {
        div()
            .flex_grow(1.0)
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .when(self.notion_chrome.notion_sidebar_visible, |this| {
                this.child(self.render_cached_notion_sidebar_region(cx))
            })
            .child(page_main_pane)
    }

    fn notion_page_root_frame(&self, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        div()
            .id("notnotion-root")
            .size_full()
            .relative()
            .flex()
            .focusable()
            .bg(rgb(self.theme.app_bg))
            .text_color(rgb(self.theme.text_primary))
            .can_drop(|dragged, _, _| {
                dragged
                    .downcast_ref::<DraggedNotionSidebarResize>()
                    .is_some()
            })
            .on_drag_move::<DraggedNotionSidebarResize>(
                cx.listener(Self::handle_notion_sidebar_resize_drag_move),
            )
            .on_drop(cx.listener(|_: &mut SurfaceState, _: &DraggedNotionSidebarResize, _, _| {}))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.page_editor
                        .dismiss_page_block_interaction_from_root(cx);
                }),
            )
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_key_down(cx.listener(Self::handle_key_down))
    }
}

impl NotionChromeState {
    pub(crate) fn toggle_notion_sidebar(&mut self, cx: &mut Context<SurfaceState>) {
        self.notion_sidebar_visible = !self.notion_sidebar_visible;
        self.notion_page_menu_open = false;
        cx.notify();
    }

    pub(crate) fn toggle_notion_page_menu(&mut self, cx: &mut Context<SurfaceState>) {
        self.notion_page_menu_open = !self.notion_page_menu_open;
        cx.notify();
    }

    pub(crate) fn set_notion_page_top_controls_visible(
        &mut self,
        visible: bool,
        cx: &mut Context<SurfaceState>,
    ) {
        if self.notion_page_top_controls_visible == visible {
            return;
        }
        self.notion_page_top_controls_visible = visible;
        cx.notify();
    }
}

impl SurfaceState {
    pub(crate) fn page_layout(&self) -> NotionPageLayout {
        NotionPageLayout::new(
            &self.notion_chrome,
            self.preview_width,
            self.viewport.app_width(),
        )
    }

    pub(crate) fn handle_notion_sidebar_resize_drag_move(
        &mut self,
        event: &DragMoveEvent<DraggedNotionSidebarResize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let position = point(
            event.event.position.x - event.bounds.left(),
            event.event.position.y - event.bounds.top(),
        );
        let layout = self.page_layout();
        if self
            .notion_chrome
            .set_sidebar_width(position.x.as_f32().max(0.0), layout)
        {
            cx.notify();
        }
    }
}
