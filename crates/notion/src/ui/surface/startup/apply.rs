use crate::ui::board_workspace::PageEditSession;
use crate::ui::board_workspace::PageFocusSession;
use crate::ui::board_workspace::PageMutationAction;

use std::mem;

use gpui::Context;

use super::super::{NotionStartup, SurfaceState};
use crate::model::{
    NotionCachedWorkspaceBootstrap, NotionPreviousStateDisposition, NotionWorkspaceBootstrap,
};

impl SurfaceState {
    pub(super) fn apply_notion_cached_bootstrap(
        &mut self,
        request_generation: u64,
        cached: NotionCachedWorkspaceBootstrap,
        cx: &mut Context<Self>,
    ) {
        if self.notion_startup_generation != request_generation {
            return;
        }
        let route = cached.route.clone();
        let startup = match &self.notion_startup {
            NotionStartup::Loading {
                bootstrap_api,
                request_started,
                cache_request_started,
                retry_attempt,
                ..
            } => NotionStartup::Loading {
                bootstrap_api: bootstrap_api.clone(),
                request_started: *request_started,
                cache_request_started: *cache_request_started,
                cached: Some(cached),
                retry_attempt: *retry_attempt,
            },
            NotionStartup::Error {
                bootstrap_api,
                retry_attempt,
                cache_retry,
                ..
            } => NotionStartup::Error {
                bootstrap_api: bootstrap_api.clone(),
                cached: Some(cached),
                retry_attempt: *retry_attempt,
                cache_retry: *cache_retry,
            },
            NotionStartup::Ready(_) => return,
            #[cfg(any(test, feature = "test-support"))]
            NotionStartup::Fixture(_) => return,
        };
        self.dispatch_page_mutation_action(PageMutationAction::CaptureVisibleProjection, cx);
        let mut replacement = Self::from_startup(
            startup,
            self.appearance_mode,
            self.viewport,
            self.surface_active,
            self.notion_resources.clone(),
        );
        let _ = replacement.restore_notion_route(&route);
        replacement.restore_notion_startup_ui_state(self, false, true);
        replacement.notion_chrome.notion_sidebar_width = replacement.page_layout().sidebar_width();
        replacement.page_mutations = mem::take(&mut self.page_mutations);
        replacement.dispatch_page_mutation_action(PageMutationAction::RestoreVisibleProjection, cx);
        replacement.notion_startup_generation = self.notion_startup_generation;
        *self = replacement;
        cx.notify();
    }

    pub(super) fn apply_notion_bootstrap(
        &mut self,
        request_generation: u64,
        bootstrap: NotionWorkspaceBootstrap,
        cx: &mut Context<Self>,
    ) {
        if self.notion_startup_generation != request_generation {
            return;
        }
        let route = bootstrap.route.clone();
        let discard_previous_state =
            bootstrap.previous_state_disposition == NotionPreviousStateDisposition::Discard;
        let preserve_visible_state = !discard_previous_state
            && self
                .notion_startup
                .route()
                .is_some_and(|current_route| current_route == &route);
        let cached_page_edits = preserve_visible_state
            .then(|| {
                self.notion_startup.cached_page_edit_handoff(
                    self.page_documents
                        .standalone
                        .as_ref()
                        .map(|page| &page.data.page),
                )
            })
            .flatten();
        if discard_previous_state {
            self.page_mutations.invalidate_session();
        }
        let mut replacement = self.live_bootstrap_replacement(bootstrap, !discard_previous_state);
        let _ = replacement.restore_notion_route(&route);
        replacement.restore_notion_startup_ui_state(
            self,
            preserve_visible_state,
            !discard_previous_state,
        );
        replacement.notion_chrome.notion_sidebar_width = replacement.page_layout().sidebar_width();
        self.reconcile_optimistic_cached_page(&mut replacement, cached_page_edits, cx);
        if preserve_visible_state {
            replacement
                .page_editor
                .take_cached_state_from(&mut self.page_editor);
            let transition = {
                let mut edit =
                    PageEditSession::new(&mut replacement.page_editor, &replacement.page_documents);
                edit.retain_compatible_page_edit_histories();
                edit.finish(())
            };
            replacement.apply_page_edit_transition(transition, cx);
        }
        replacement.page_mutations = mem::take(&mut self.page_mutations);
        replacement.notion_startup_generation = self.notion_startup_generation;
        *self = replacement;
        PageFocusSession::new(&mut self.page_editor, &mut self.page_documents)
            .reconcile_active_projection(cx);
        self.dispatch_page_mutation_action(PageMutationAction::StartPending, cx);
        self.load_notion_search_recents(cx);
        cx.notify();
    }

    fn live_bootstrap_replacement(
        &self,
        bootstrap: NotionWorkspaceBootstrap,
        preserve_cached_regions: bool,
    ) -> Self {
        let mut replacement = Self::from_startup(
            NotionStartup::Ready(bootstrap),
            self.appearance_mode,
            self.viewport,
            self.surface_active,
            self.notion_resources.clone(),
        );
        replacement.preview_width = self.preview_width;
        if preserve_cached_regions {
            replacement
                .presentation
                .inherit_cached_regions(&self.presentation);
        }
        replacement
    }

    fn restore_notion_startup_ui_state(
        &mut self,
        previous: &Self,
        preserve_visible_state: bool,
        preserve_session_state: bool,
    ) {
        self.preview_width = previous.preview_width;
        self.presentation
            .inherit_cached_regions(&previous.presentation);
        self.notion_chrome.notion_sidebar_width = previous.notion_chrome.notion_sidebar_width;
        if preserve_session_state {
            self.page_editor
                .inherit_session_state(&previous.page_editor);
            self.notion_search
                .inherit_completed_recents(&previous.notion_search);
        }
        if !preserve_visible_state {
            return;
        }

        self.board_view.scroll_handle = previous.board_view.scroll_handle.clone();
        self.board_view.timeline_scroll_handle = previous.board_view.timeline_scroll_handle.clone();
        self.date_view.visible_month = previous.date_view.visible_month;
        self.notion_sidebar
            .inherit_startup_state(&previous.notion_sidebar);
        self.notion_chrome.notion_sidebar_visible = previous.notion_chrome.notion_sidebar_visible
            && self.board.page_shell.as_ref().is_some_and(|page_shell| {
                !(page_shell.builtin_links.is_empty() && page_shell.sidebar_sections.is_empty())
            });
        if let (Some(current_page), Some(previous_page)) = (
            self.page_documents.standalone.as_mut(),
            previous.page_documents.standalone.as_ref(),
        ) {
            if current_page.data.page.block_id == previous_page.data.page.block_id {
                current_page.list_state = previous_page.list_state.clone();
                current_page.list_allocation = previous_page.list_allocation.clone();
            }
        }
        self.page_documents
            .rebuild_loaded_page_disclosure_projections(&mut self.page_editor);
        self.notion_sidebar
            .rebuild_rows(self.board.page_shell.as_ref());
    }
}
