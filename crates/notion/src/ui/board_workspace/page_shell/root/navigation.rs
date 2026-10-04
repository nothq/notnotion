use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::PageMutationAction;
use std::mem;

use super::handle_sidebar_resource_event;
use crate::model::{NotionBoardUrl, NotionLaunchRoute};
use crate::ui::surface::{
    NotionNavigationDirection, NotionNavigationHistoryUpdate, SidebarResourceContext,
};
use crate::ui::{
    Context, NotionPendingNavigation, NotionWorkspaceLoad, PageShellSidebarItem, PageShellSnapshot,
    SurfaceState,
};

mod prefetch;
mod presence;
mod startup;
mod state;

use state::{NotionNavigationCompletion, NotionNavigationStart, NotionWorkspaceReplacement};

impl SurfaceState {
    pub(crate) fn open_notion_workspace(
        &mut self,
        board_url: String,
        _label: String,
        cx: &mut Context<Self>,
    ) {
        let result = board_url.parse::<NotionBoardUrl>().and_then(|board_url| {
            let history_update = self.notion_startup.push_history_update();
            self.begin_notion_navigation(NotionLaunchRoute::board(board_url), history_update, cx)
        });
        if let Err(error) = result {
            self.print_notion_error(error);
            cx.notify();
        }
    }

    pub(crate) fn navigate_notion_workspace_history(
        &mut self,
        direction: NotionNavigationDirection,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.notion_chrome.notion_pending_navigation.is_some() {
            return false;
        }
        let Some(source) = self.notion_startup.ready_route() else {
            return false;
        };
        let Some(target) = self
            .notion_chrome
            .notion_navigation_history
            .target(direction)
        else {
            return false;
        };
        matches!(
            self.begin_notion_navigation(
                target.clone(),
                NotionNavigationHistoryUpdate::Traverse {
                    direction,
                    source,
                    target,
                },
                cx,
            ),
            Ok(NotionNavigationStart::Started)
        )
    }

    /// Starts showing `route`, or reports why the surface cannot navigate.
    pub(crate) fn begin_notion_navigation(
        &mut self,
        route: NotionLaunchRoute,
        history_update: NotionNavigationHistoryUpdate,
        cx: &mut Context<Self>,
    ) -> Result<NotionNavigationStart, String> {
        let board_url = route.board_url().as_str().to_string();
        if self.notion_startup.board_url().as_deref() == Some(board_url.as_str()) {
            return Ok(NotionNavigationStart::AlreadyShown);
        }
        let workspace_api = self.notion_startup.navigation_api()?;
        if self
            .notion_chrome
            .notion_pending_navigation
            .as_ref()
            .is_some_and(|pending| pending.board_url == board_url)
        {
            return Ok(NotionNavigationStart::AlreadyPending);
        }
        self.notion_chrome.notion_page_menu_open = false;
        self.comments.dismiss_notion_comments_panel_without_notify();
        self.notion_chrome.inline_database_view_menu = None;
        self.notion_sidebar.prefetch_target = None;
        self.notion_chrome.notion_navigation_request_id = self
            .notion_chrome
            .notion_navigation_request_id
            .checked_add(1)
            .expect("Notion navigation request id exhausted");
        let request_id = self.notion_chrome.notion_navigation_request_id;
        self.notion_chrome.notion_pending_navigation = Some(NotionPendingNavigation {
            board_url: board_url.clone(),
            request_id,
        });
        let load_api = workspace_api;
        let completion = NotionNavigationCompletion {
            board_url: board_url.clone(),
            route,
            request_id,
            page_authorities: self.page_mutations.authority_snapshot(),
            history_update,
        };
        self.spawn_background_task(
            board_url,
            cx,
            move |board_url| load_api.load_notion_workspace(&board_url),
            move |this, result, cx| this.complete_notion_navigation(completion, result, cx),
        );
        Ok(NotionNavigationStart::Started)
    }

    fn complete_notion_navigation(
        &mut self,
        completion: NotionNavigationCompletion,
        result: crate::model::NotionWorkspaceResult<NotionWorkspaceLoad>,
        cx: &mut Context<Self>,
    ) {
        if self
            .notion_chrome
            .notion_pending_navigation
            .as_ref()
            .is_none_or(|pending| pending.request_id != completion.request_id)
        {
            return;
        }
        match result {
            Ok(loaded) => self.install_completed_notion_navigation(completion, loaded, cx),
            Err(error) => {
                self.notion_chrome.notion_pending_navigation = None;
                if self.handle_notion_workspace_failure("workspace navigation failed", error, cx) {
                    return;
                }
                self.notion_sidebar
                    .rebuild_rows(self.board.page_shell.as_ref());
                cx.notify();
            }
        }
    }

    fn install_completed_notion_navigation(
        &mut self,
        completion: NotionNavigationCompletion,
        mut loaded: NotionWorkspaceLoad,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = loaded
            .workspace_api
            .synchronize_workspace_snapshot(&mut loaded.workspace)
        {
            self.notion_chrome.notion_pending_navigation = None;
            if self.handle_notion_workspace_failure("workspace synchronization failed", error, cx) {
                return;
            }
            self.notion_sidebar
                .rebuild_rows(self.board.page_shell.as_ref());
            cx.notify();
            return;
        }
        let presence_api = loaded.workspace_api.clone();
        let board_url = completion.board_url.clone();
        merge_notion_sidebar_hydration(
            self.board.page_shell.as_ref(),
            loaded.workspace.page_shell.as_mut(),
            &completion.board_url,
        );
        let page_authority = completion.reconcile_page_authority(
            loaded.workspace.page_content.as_mut(),
            &mut self.page_mutations,
        );
        self.notion_chrome
            .notion_navigation_history
            .record_success(completion.history_update, &completion.route);
        self.replace_notion_workspace(
            NotionWorkspaceReplacement {
                board: loaded.workspace,
                workspace_api: loaded.workspace_api,
                code_settings: loaded.code_settings,
                route: completion.route,
                appearance_mode: self.appearance_mode,
                viewport: self.viewport,
                page_authority,
            },
            cx,
        );
        let resource_context = SidebarResourceContext::new(
            self.notion_startup.board_url(),
            self.notion_startup.workspace_api(),
            self.notion_chrome.notion_navigation_request_id,
        );
        let jobs = self
            .notion_sidebar
            .prepare_expanded_children(self.board.page_shell.as_ref(), &resource_context);
        for job in jobs {
            job.spawn(cx, handle_sidebar_resource_event);
        }
        self.load_notion_page_presence(board_url, completion.request_id, presence_api, cx);
        cx.notify();
    }

    fn replace_notion_workspace(
        &mut self,
        replacement: NotionWorkspaceReplacement,
        cx: &mut Context<Self>,
    ) {
        self.dispatch_page_mutation_action(PageMutationAction::CaptureVisibleProjection, cx);
        let NotionWorkspaceReplacement {
            board,
            workspace_api,
            code_settings,
            route,
            appearance_mode,
            viewport,
            page_authority,
        } = replacement;
        let notion_sidebar_visible = self.notion_chrome.notion_sidebar_visible
            && board.page_shell.as_ref().is_some_and(|page_shell| {
                !(page_shell.builtin_links.is_empty() && page_shell.sidebar_sections.is_empty())
            });
        let page_mutations = mem::take(&mut self.page_mutations);
        let startup =
            self.notion_startup
                .for_navigation(board, workspace_api, code_settings, route);
        let mut replacement = Self::from_startup(
            startup,
            appearance_mode,
            viewport,
            self.surface_active,
            self.notion_resources.clone(),
        );
        replacement.page_mutations = page_mutations;
        if let Some((page_id, authority)) = page_authority {
            replacement.dispatch_page_document_action(
                PageDocumentAction::AdvanceAuthority { page_id, authority },
                cx,
            );
        }
        replacement.restore_notion_navigation_ui_state(self, notion_sidebar_visible);
        replacement
            .notion_sidebar
            .reconcile_node_state(replacement.board.page_shell.as_ref());
        replacement
            .notion_sidebar
            .rebuild_rows(replacement.board.page_shell.as_ref());
        replacement.notion_chrome.notion_sidebar_width = replacement.page_layout().sidebar_width();
        *self = replacement;
    }

    fn restore_notion_navigation_ui_state(
        &mut self,
        previous: &Self,
        notion_sidebar_visible: bool,
    ) {
        self.preview_width = previous.preview_width;
        self.presentation.page_host = previous.presentation.page_host.clone();
        self.date_view.visible_month = previous.date_view.visible_month;
        self.presentation
            .inherit_cached_regions(&previous.presentation);
        self.page_editor
            .inherit_session_state(&previous.page_editor);
        self.notion_search
            .inherit_completed_recents(&previous.notion_search);
        self.notion_chrome.notion_sidebar_visible = notion_sidebar_visible;
        self.notion_sidebar
            .inherit_navigation_state(&previous.notion_sidebar);
        self.notion_chrome.notion_sidebar_width = previous.notion_chrome.notion_sidebar_width;
        self.notion_chrome.notion_navigation_request_id =
            previous.notion_chrome.notion_navigation_request_id;
        self.notion_chrome.notion_navigation_history =
            previous.notion_chrome.notion_navigation_history.clone();
    }
}

fn merge_notion_sidebar_hydration(
    previous: Option<&PageShellSnapshot>,
    next: Option<&mut PageShellSnapshot>,
    current_board_url: &str,
) {
    let (Some(previous), Some(next)) = (previous, next) else {
        return;
    };
    next.merge_sidebar_hydration_from(&previous.sidebar_sections);
    for section in &mut next.sidebar_sections {
        for item in &mut section.items {
            refresh_notion_sidebar_active_state(item, current_board_url);
        }
    }
}

fn refresh_notion_sidebar_active_state(item: &mut PageShellSidebarItem, current_board_url: &str) {
    item.active = item.target_board_url.as_deref() == Some(current_board_url);
    for child in &mut item.children {
        refresh_notion_sidebar_active_state(child, current_board_url);
    }
}
