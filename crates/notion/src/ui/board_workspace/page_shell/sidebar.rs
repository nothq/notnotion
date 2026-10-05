use super::{
    alpha, div, img, notion_sidebar_active_bg, notion_sidebar_active_text, notion_sidebar_bg,
    notion_sidebar_muted, notion_sidebar_text, point, px, relative, rgb, AnyElement,
    AppearanceMode, BoxShadow, Div, DraggedNotionSidebarResize, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled,
};
use crate::model::{
    MutateSidebarInboxAction, PageShellCalendarEvent, PageShellChatThread, PageShellInboxItem,
    PageShellNodeIdentity, PageShellSidebarSectionIdentity,
};
use crate::ui::board_workspace::PageShellIconRenderer;
use crate::ui::surface::{
    NotionSidebarCalendarState, NotionSidebarChatsState, NotionSidebarInboxState, NotionSidebarRow,
    NotionSidebarRowAction, NotionSidebarRowLayout, NotionSidebarSectionRow, NotionSidebarState,
    NotionSidebarUpdate, NotionSidebarVisibleRow,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{
    Arc, NotionSidebarNodeKey, NotionSidebarSectionKey, NotionSidebarTab, PageShellIcon,
    PageShellLink, PageShellSidebarItem, PageShellSidebarSection, PageShellSnapshot,
};
use gpui::{App, SharedString};

mod chat;
mod chat_render;
mod footer;
mod header;
mod host;
mod inbox;
mod panel;
mod panels;
mod row;
mod section;
mod tree;

pub(crate) use host::activate_sidebar_tab;

const SIDEBAR_HEADER_HEIGHT: f32 = 48.0;
/// Room above the header for the window controls notnotion draws over the
/// sidebar's top-left corner on macOS.
const SIDEBAR_WINDOW_CONTROLS_CLEARANCE: f32 = if cfg!(target_os = "macos") { 28.0 } else { 0.0 };
const SIDEBAR_FOOTER_HEIGHT: f32 = 64.0;
const SIDEBAR_RESIZE_HITBOX_WIDTH: f32 = 12.0;
const SIDEBAR_ROW_HEIGHT: f32 = 30.0;
const SIDEBAR_ROW_ADVANCE: f32 = 31.0;
const SIDEBAR_SECTION_GAP: f32 = 12.0;

pub(super) const fn notion_sidebar_panel_muted(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0xa19e99,
        AppearanceMode::Dark => notion_sidebar_muted(appearance_mode),
    }
}

#[derive(Clone)]
pub(super) enum SidebarAction {
    Update(NotionSidebarUpdate),
    Inbox(SidebarInboxAction),
    Navigate(SidebarNavigationAction),
    OpenChatThread(SharedString),
    TogglePageMenu,
    ToggleSearch,
}

#[derive(Clone)]
pub(super) enum SidebarInboxAction {
    LoadMore,
    Mutate(MutateSidebarInboxAction),
    OpenInboxTarget {
        board_url: String,
        label: String,
        unread_notification_id: Option<String>,
    },
}

#[derive(Clone)]
pub(super) enum SidebarNavigationAction {
    OpenUrl(SharedString),
    OpenWorkspace {
        board_url: String,
        label: SharedString,
    },
    SetWorkspacePrefetch {
        board_url: String,
        hovered: bool,
    },
}

/// A borrowed view of sidebar-owned state. It contains only data and narrow
/// capabilities needed to render the sidebar; host coordination stays in
/// `host.rs` behind typed `SidebarAction`s.
pub(crate) struct SidebarView<'a> {
    state: &'a NotionSidebarState,
    rows: Arc<[NotionSidebarVisibleRow]>,
    page_shell: &'a PageShellSnapshot,
    appearance_mode: AppearanceMode,
    page_icons: PageShellIconRenderer,
    width: f32,
    search_open: bool,
    actions: ViewActionSink<SidebarAction>,
}

impl SidebarView<'_> {
    pub(super) fn render(self, cx: &mut App) -> AnyElement {
        let width = px(self.width);
        let renderer = SidebarRenderer {
            appearance_mode: self.appearance_mode,
            page_icons: self.page_icons,
            width: self.width,
            actions: self.actions,
        };
        div()
            .id("notion-page-sidebar")
            .relative()
            .w(width)
            .min_w(width)
            .max_w(width)
            .h_full()
            .min_h(px(0.0))
            .flex_none()
            .border_r_1()
            .border_color(renderer.notion_sidebar_separator_color())
            .bg(rgb(renderer.notion_sidebar_surface_bg_color()))
            .pt(px(SIDEBAR_WINDOW_CONTROLS_CLEARANCE))
            .flex()
            .flex_col()
            .child(renderer.render_notion_sidebar_header(
                self.state,
                self.page_shell,
                self.search_open,
                cx,
            ))
            .child(renderer.render_notion_sidebar_body(self.state, self.rows, cx))
            .child(renderer.render_notion_sidebar_footer(cx))
            .child(renderer.render_notion_sidebar_resize_handle(cx))
            .into_any_element()
    }
}

#[derive(Clone)]
pub(super) struct SidebarRenderer {
    appearance_mode: AppearanceMode,
    page_icons: PageShellIconRenderer,
    width: f32,
    actions: ViewActionSink<SidebarAction>,
}

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_hover_actions(
        &self,
        group: SharedString,
        add: SidebarAction,
        cx: &mut App,
    ) -> Div {
        div()
            .absolute()
            .top(px(5.0))
            .right(px(0.0))
            .h(px(20.0))
            .pl(px(4.0))
            .bg(self.notion_sidebar_selected_bg())
            .flex()
            .items_center()
            .opacity(0.0)
            .group_hover(group, |mut style| {
                style.opacity = Some(1.0);
                style
            })
            .child(self.render_notion_sidebar_more_action(cx))
            .child(self.render_notion_sidebar_add_action(add, cx))
    }

    fn render_notion_sidebar_more_action(&self, cx: &mut App) -> Div {
        self.notion_sidebar_hover_action()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(self.page_icons.builtin("more", 16.0, cx))
    }

    fn render_notion_sidebar_add_action(&self, add: SidebarAction, cx: &mut App) -> Div {
        self.notion_sidebar_hover_action()
            .text_size(px(16.0))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    add.clone()
                }),
            )
            .child(self.page_icons.builtin("add", 16.0, cx))
    }

    fn notion_sidebar_hover_action(&self) -> Div {
        div()
            .size(px(20.0))
            .rounded(px(5.0))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(self.notion_sidebar_row_text_color()))
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
    }

    pub(super) fn render_notion_sidebar_chevron(&self, expanded: bool, cx: &mut App) -> AnyElement {
        self.page_icons.sidebar_chevron(expanded, 12.0, cx)
    }

    pub(super) fn render_notion_page_shell_icon(
        &self,
        icon: &PageShellIcon,
        size: f32,
        cx: &mut App,
    ) -> AnyElement {
        self.page_icons.render(icon, size, cx)
    }

    pub(super) fn render_notion_sidebar_builtin_icon_at_size(
        &self,
        icon_name: &str,
        size: f32,
        cx: &mut App,
    ) -> AnyElement {
        self.page_icons.builtin(icon_name, size, cx)
    }

    pub(super) fn notion_sidebar_hover_bg(&self) -> gpui::Hsla {
        match self.appearance_mode {
            AppearanceMode::Light => rgb(0xeeeceb).into(),
            AppearanceMode::Dark => alpha(notion_sidebar_active_bg(self.appearance_mode), 0.18),
        }
    }

    pub(super) fn notion_sidebar_selected_bg(&self) -> gpui::Hsla {
        match self.appearance_mode {
            AppearanceMode::Light => self.notion_sidebar_hover_bg(),
            AppearanceMode::Dark => rgb(0x2c2c2c).into(),
        }
    }

    pub(super) fn notion_sidebar_selected_text_color(&self) -> u32 {
        notion_sidebar_active_text(self.appearance_mode)
    }

    pub(super) fn notion_sidebar_row_text_color(&self) -> u32 {
        match self.appearance_mode {
            AppearanceMode::Light => 0x5f5e59,
            AppearanceMode::Dark => notion_sidebar_text(self.appearance_mode),
        }
    }

    pub(super) fn notion_sidebar_top_text_color(&self) -> u32 {
        match self.appearance_mode {
            AppearanceMode::Light => 0x37352f,
            AppearanceMode::Dark => notion_sidebar_active_text(self.appearance_mode),
        }
    }

    fn notion_sidebar_separator_color(&self) -> gpui::Hsla {
        rgb(match self.appearance_mode {
            AppearanceMode::Light => 0xf0efed,
            AppearanceMode::Dark => 0x2c2c2c,
        })
        .into()
    }

    fn notion_sidebar_surface_bg_color(&self) -> u32 {
        notion_sidebar_bg(self.appearance_mode)
    }
}
