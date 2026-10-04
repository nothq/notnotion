use super::{
    div, px, rgb, AnyElement, Context, DraggedNotionSidebarResize, FluentBuilder,
    InteractiveElement, IntoElement, MouseButton, NotionCachedSurfaceRegion,
    NotionCachedSurfaceView, ParentElement, Render, StatefulInteractiveElement, Styled,
    SurfaceState, Window,
};
use gpui::{AnyView, StyleRefinement};

mod content;
mod layout;

use crate::ui::board_workspace::SidebarView;

impl Render for NotionCachedSurfaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let surface = self
            .surface
            .upgrade()
            .expect("cached Notion region lost its surface");
        surface.update(cx, |surface, cx| match self.region {
            NotionCachedSurfaceRegion::Sidebar => surface.render_notion_sidebar_region(cx),
            NotionCachedSurfaceRegion::Main => {
                let main_width = px(surface.page_layout().main_pane_width());
                div()
                    .w(main_width)
                    .min_w(main_width)
                    .max_w(main_width)
                    .h_full()
                    .min_h(px(0.0))
                    .flex_none()
                    .overflow_hidden()
                    .flex()
                    .child(surface.render_notion_main_region(cx))
                    .into_any_element()
            }
        })
    }
}

impl SurfaceState {
    fn render_board_workspace_root(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let workspace = if self.board.page_shell.is_some()
            && !content::has_main_content(&self.board, &self.page_documents)
        {
            self.render_notion_page_root(cx)
        } else {
            self.board_root(cx).into_any_element()
        };
        if !self.notion_startup.cached_workspace_visible() {
            return workspace;
        }
        div()
            .size_full()
            .relative()
            .capture_key_down(cx.listener(|_: &mut SurfaceState, _, _, cx| cx.stop_propagation()))
            .child(workspace)
            .child(
                div()
                    .id("notion-cached-workspace-read-only")
                    .absolute()
                    .inset_0()
                    .block_mouse_except_scroll()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_: &mut SurfaceState, _, _, cx| cx.stop_propagation()),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(|_: &mut SurfaceState, _, _, cx| cx.stop_propagation()),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|_: &mut SurfaceState, _, _, cx| cx.stop_propagation()),
                    ),
            )
            .into_any_element()
    }

    fn board_root(&self, cx: &mut Context<Self>) -> gpui::Stateful<crate::ui::Div> {
        let chrome = &self.notion_chrome;
        self.board_root_frame(cx)
            .child(self.board_root_content(cx))
            .when(self.notion_sidebar.inbox_menu_is_open(), |this| {
                this.child(SidebarView::render_inbox_menu_backdrop(self.viewport, cx))
            })
            .child(if chrome.notion_ai_open {
                self.render_notion_ai_panel(cx)
            } else {
                self.render_notion_ai_button(cx)
            })
            .when(
                self.board.page_shell.is_some() && chrome.notion_page_menu_open,
                |this| {
                    this.child(
                        self.page_shell_body_renderer(cx)
                            .render_notion_page_create_menu(cx),
                    )
                },
            )
            .when_some(
                self.board_view.drag.clone().filter(|drag| drag.moved),
                |this, drag| {
                    this.child(self.board_columns_renderer(cx).render_drag_ghost(drag, cx))
                },
            )
            .when(self.page_documents.selected_page.is_some(), |this| {
                this.child(self.render_selected_page_overlay(cx))
            })
            .children(self.render_page_mention_picker(cx))
            .children(self.render_page_rich_text_toolbar(cx))
            .when_some(chrome.toolbar_dialog, |this, dialog| {
                this.child(self.render_toolbar_dialog(dialog, cx))
            })
            .when(chrome.inline_toolbar_dialog.is_some(), |this| {
                this.child(self.render_inline_toolbar_dialog_overlay(cx))
            })
            .when(chrome.date_undated_dialog.is_some(), |this| {
                this.child(self.render_date_undated_dialog_overlay(cx))
            })
            .when(chrome.inline_database_view_menu.is_some(), |this| {
                this.child(self.render_inline_database_view_menu_overlay(cx))
            })
            .when(chrome.notion_search_open, |this| {
                this.child(self.render_notion_search_overlay(cx))
            })
            .when(chrome.ai_autofill_dialog.is_some(), |this| {
                this.child(self.render_ai_autofill_overlay(cx))
            })
            .when(chrome.status_property_picker.is_some(), |this| {
                this.child(self.render_status_property_picker_overlay(cx))
            })
            .when(chrome.top_bar_popover_open(), |this| {
                this.child(self.render_top_bar_popover(cx))
            })
            .when(self.comments.panel.is_some(), |this| {
                this.child(self.render_notion_comments_overlay(cx))
            })
    }

    fn board_root_frame(&self, cx: &mut Context<Self>) -> gpui::Stateful<crate::ui::Div> {
        div()
            .id("notion-root")
            .size_full()
            .relative()
            .flex()
            .focusable()
            .text_color(rgb(self.theme.text_primary))
            .bg(rgb(self.theme.app_bg))
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

    fn board_root_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let board_surface = self.render_cached_notion_main_region(cx);
        if self.board.page_shell.is_none() {
            return board_surface;
        }

        div()
            .flex_grow(1.0)
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .when(self.notion_chrome.notion_sidebar_visible, |this| {
                this.child(self.render_cached_notion_sidebar_region(cx))
            })
            .child(board_surface)
            .into_any_element()
    }

    pub(crate) fn render_cached_notion_sidebar_region(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.presentation.cached_sidebar.as_ref() {
            Some(sidebar) => AnyView::from(sidebar.clone())
                .cached(
                    StyleRefinement::default()
                        .w(px(self.page_layout().sidebar_width()))
                        .h_full(),
                )
                .into_any_element(),
            None => self.render_notion_sidebar_region(cx),
        }
    }

    pub(crate) fn render_cached_notion_main_region(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.presentation.cached_main.as_ref() {
            Some(main) => {
                let main_width = px(self.page_layout().main_pane_width());
                div()
                    .w(main_width)
                    .min_w(main_width)
                    .max_w(main_width)
                    .min_h(px(0.0))
                    .h_full()
                    .flex_none()
                    .overflow_hidden()
                    .flex()
                    .child(
                        AnyView::from(main.clone()).cached(
                            StyleRefinement::default()
                                .w(main_width)
                                .h_full()
                                .min_w(main_width)
                                .max_w(main_width)
                                .min_h(px(0.0))
                                .flex_none(),
                        ),
                    )
                    .into_any_element()
            }
            None => self.render_notion_main_region(cx),
        }
    }

    fn render_notion_sidebar_region(&self, cx: &mut Context<Self>) -> AnyElement {
        self.board
            .page_shell
            .as_ref()
            .map(|page_shell| self.render_notion_page_sidebar(page_shell, cx))
            .unwrap_or_else(|| div().into_any_element())
    }

    fn render_notion_main_region(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(page_shell) = self
            .board
            .page_shell
            .as_ref()
            .filter(|_| !content::has_main_content(&self.board, &self.page_documents))
        {
            return self
                .page_shell_body_renderer(cx)
                .render_notion_page_main_pane(page_shell, cx)
                .into_any_element();
        }
        let columns = (0..self.columns.len())
            .map(|column_index| {
                self.board_columns_renderer(cx)
                    .render_column(column_index, cx)
            })
            .collect::<Vec<_>>();
        self.board_main_pane(columns, cx)
    }

    fn board_main_pane(&self, columns: Vec<AnyElement>, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .h_full()
            .flex()
            .flex_col()
            .child(self.top_bar_renderer(cx).render_top_bar(cx))
            .child(self.board_container(columns, cx))
            .into_any_element()
    }

    fn board_container(&self, columns: Vec<AnyElement>, cx: &mut Context<Self>) -> AnyElement {
        if let Some(page) = self
            .page_documents
            .standalone
            .as_ref()
            .filter(|_| !self.board.has_collection_content())
        {
            self.render_notion_standalone_page_content(page, cx)
        } else {
            div()
                .flex_grow(1.0)
                .min_w(px(0.0))
                .w_full()
                .flex()
                .flex_col()
                .pb(px(24.0))
                .child(layout::board_title(self.board.database_title.clone()))
                .child(layout::board_controls(
                    self.render_view_tabs(cx),
                    self.render_board_toolbar(cx),
                ))
                .when(
                    self.database_filter.bar_is_visible(&self.board),
                    |container| container.child(self.render_database_filter_bar(None, cx)),
                )
                .child(self.render_active_view(columns, cx))
                .into_any_element()
        }
    }
}

impl Render for SurfaceState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_notion_startup(cx);
        self.ensure_notion_page_shell_resources(cx);
        self.ensure_date_view_queries(cx);
        if let Some(startup) = self.notion_startup.render_startup(self.theme) {
            return startup;
        }
        self.render_board_workspace_root(cx)
    }
}
