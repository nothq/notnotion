use crate::ui::surface::{NotionSidebarEffect, NotionSidebarUpdate, SidebarResourceContext};
use crate::ui::view_actions::{ViewActionSink, ViewNotifier};
use crate::ui::{Context, NotionAiMode, NotionSidebarTab, PageShellSnapshot, SurfaceState, Window};

use super::super::root::handle_sidebar_resource_event;
use super::{
    PageShellIconRenderer, SidebarAction, SidebarInboxAction, SidebarNavigationAction, SidebarView,
};

impl SurfaceState {
    pub(crate) fn render_notion_page_sidebar(
        &self,
        page_shell: &PageShellSnapshot,
        cx: &mut Context<Self>,
    ) -> crate::ui::AnyElement {
        SidebarView {
            state: &self.notion_sidebar,
            page_shell,
            appearance_mode: self.appearance_mode,
            page_icons: PageShellIconRenderer::new(
                self.appearance_mode,
                self.icons.clone(),
                self.notion_resources.clone(),
                ViewNotifier::new(cx),
            ),
            width: self.page_layout().sidebar_width(),
            search_open: self.notion_chrome.notion_search_open,
            actions: ViewActionSink::new(cx, handle_sidebar_action),
            rows: self.notion_sidebar.rows.clone(),
        }
        .render(cx)
    }
}

fn handle_sidebar_action(
    surface: &mut SurfaceState,
    action: SidebarAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        SidebarAction::Update(update) => apply_sidebar_update(surface, update, cx),
        SidebarAction::Inbox(action) => handle_sidebar_inbox_action(surface, action, cx),
        SidebarAction::Navigate(action) => handle_sidebar_navigation(surface, action, cx),
        SidebarAction::OpenChatThread(thread_id) => {
            surface.notion_sidebar.select_chat(thread_id);
            surface.notion_chrome.notion_ai_mode = NotionAiMode::Sidebar;
            surface.notion_chrome.notion_ai_open = true;
            surface
                .notion_sidebar
                .rebuild_rows(surface.board.page_shell.as_ref());
            cx.notify();
        }
        SidebarAction::TogglePageMenu => surface.notion_chrome.toggle_notion_page_menu(cx),
        SidebarAction::ToggleSearch => surface.toggle_notion_search(cx),
    }
}

fn handle_sidebar_inbox_action(
    surface: &mut SurfaceState,
    action: SidebarInboxAction,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        SidebarInboxAction::LoadMore => {
            let context = SidebarResourceContext::new(
                surface.notion_startup.board_url(),
                surface.notion_startup.workspace_api(),
                surface.notion_chrome.notion_navigation_request_id,
            );
            let start = surface.notion_sidebar.prepare_inbox_pagination(&context);
            if let Some(start) = start {
                start.spawn(cx, handle_sidebar_resource_event);
            }
        }
        SidebarInboxAction::Mutate(action) => {
            let context = SidebarResourceContext::new(
                surface.notion_startup.board_url(),
                surface.notion_startup.workspace_api(),
                surface.notion_chrome.notion_navigation_request_id,
            );
            let start = surface
                .notion_sidebar
                .prepare_inbox_mutation(action, &context);
            if let Some(start) = start {
                start.spawn(cx, handle_sidebar_resource_event);
            }
        }
        SidebarInboxAction::OpenInboxTarget {
            board_url,
            label,
            unread_notification_id,
        } => {
            if let Some(notification_id) = unread_notification_id {
                let context = SidebarResourceContext::new(
                    surface.notion_startup.board_url(),
                    surface.notion_startup.workspace_api(),
                    surface.notion_chrome.notion_navigation_request_id,
                );
                let start = surface.notion_sidebar.prepare_inbox_mutation(
                    crate::model::MutateSidebarInboxAction::SetRead {
                        notification_ids: vec![notification_id],
                        read: true,
                    },
                    &context,
                );
                if let Some(start) = start {
                    start.spawn(cx, handle_sidebar_resource_event);
                }
            }
            surface.open_notion_workspace(board_url, label, cx);
        }
    }
}

fn handle_sidebar_navigation(
    surface: &mut SurfaceState,
    action: SidebarNavigationAction,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        SidebarNavigationAction::OpenUrl(url) => cx.open_url(url.as_ref()),
        SidebarNavigationAction::OpenWorkspace { board_url, label } => {
            apply_sidebar_update(
                surface,
                NotionSidebarUpdate::ActivateTab(NotionSidebarTab::Home),
                cx,
            );
            surface.open_notion_workspace(board_url, label.to_string(), cx);
        }
        SidebarNavigationAction::SetWorkspacePrefetch { board_url, hovered } => {
            surface.update_notion_workspace_prefetch(board_url, hovered, cx);
        }
    }
}

pub(in crate::ui::board_workspace::page_shell) fn apply_sidebar_update(
    surface: &mut SurfaceState,
    update: NotionSidebarUpdate,
    cx: &mut Context<SurfaceState>,
) {
    let effect = surface.notion_sidebar.reduce(update);
    match effect {
        NotionSidebarEffect::None => {}
        NotionSidebarEffect::Notify => cx.notify(),
        NotionSidebarEffect::RebuildRows => {
            surface
                .notion_sidebar
                .rebuild_rows(surface.board.page_shell.as_ref());
            cx.notify();
        }
        NotionSidebarEffect::RebuildRowsAndClosePageMenu => {
            surface.notion_chrome.notion_page_menu_open = false;
            surface
                .notion_sidebar
                .rebuild_rows(surface.board.page_shell.as_ref());
            cx.notify();
        }
        NotionSidebarEffect::RebuildRowsAndLoadNode(key) => {
            surface
                .notion_sidebar
                .rebuild_rows(surface.board.page_shell.as_ref());
            cx.notify();
            let context = SidebarResourceContext::new(
                surface.notion_startup.board_url(),
                surface.notion_startup.workspace_api(),
                surface.notion_chrome.notion_navigation_request_id,
            );
            if let Some(job) = surface.notion_sidebar.prepare_child_resource(
                key,
                surface.board.page_shell.as_ref(),
                &context,
            ) {
                job.spawn(cx, handle_sidebar_resource_event);
            }
        }
        NotionSidebarEffect::LoadInbox => {
            let context = SidebarResourceContext::new(
                surface.notion_startup.board_url(),
                surface.notion_startup.workspace_api(),
                surface.notion_chrome.notion_navigation_request_id,
            );
            let start = surface
                .notion_sidebar
                .prepare_inbox_resource(surface.board.page_shell.as_ref(), &context);
            if let Some(start) = start {
                start.spawn(cx, handle_sidebar_resource_event);
            }
        }
    }
}

pub(crate) fn activate_sidebar_tab(
    surface: &mut SurfaceState,
    tab: NotionSidebarTab,
    cx: &mut Context<SurfaceState>,
) {
    apply_sidebar_update(surface, NotionSidebarUpdate::ActivateTab(tab), cx);
}
