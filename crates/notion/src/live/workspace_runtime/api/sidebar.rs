use crate::model::{
    LoadRecentPagesRequest, LoadRecentPagesResult, LoadSidebarCalendarRequest,
    LoadSidebarCalendarResult, LoadSidebarChatsRequest, LoadSidebarChatsResult,
    LoadSidebarChildrenRequest, LoadSidebarChildrenResult, LoadSidebarInboxRequest,
    LoadSidebarInboxResult, MutateSidebarInboxRequest, PagePresenceSnapshot,
    SearchWorkspaceRequest, SearchWorkspaceResult,
};

use super::super::NotionWorkspaceRuntime;
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

impl NotionWorkspaceRuntime {
    pub(super) fn load_page_presence_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<Option<PagePresenceSnapshot>, NotionLiveError> {
        self.workspace_context.load_page_presence(session).map(Some)
    }

    pub(super) fn hydrate_sidebar_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context.hydrate_sidebar(session)
    }

    pub(super) fn load_sidebar_children_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadSidebarChildrenRequest,
    ) -> Result<LoadSidebarChildrenResult, NotionLiveError> {
        crate::live::load_sidebar_children(
            session,
            &request.current_board_url,
            &request.parent_block_id,
        )
    }

    pub(super) fn load_sidebar_calendar_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadSidebarCalendarRequest,
    ) -> Result<LoadSidebarCalendarResult, NotionLiveError> {
        let _ = request;
        self.workspace_context.load_sidebar_calendar(session)
    }

    pub(super) fn load_sidebar_chats_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadSidebarChatsRequest,
    ) -> Result<LoadSidebarChatsResult, NotionLiveError> {
        crate::live::load_sidebar_chats(session, &request.current_board_url, request.cursor)
    }

    pub(super) fn load_sidebar_inbox_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadSidebarInboxRequest,
    ) -> Result<LoadSidebarInboxResult, NotionLiveError> {
        crate::live::load_sidebar_inbox(
            session,
            &request.current_board_url,
            request.filter,
            request.size,
        )
    }

    pub(super) fn mutate_sidebar_inbox_once(
        &self,
        session: &NotionDesktopSession,
        request: MutateSidebarInboxRequest,
    ) -> Result<(), NotionLiveError> {
        crate::live::mutate_sidebar_inbox(session, &request.current_board_url, request.action)
    }

    pub(super) fn load_recent_pages_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadRecentPagesRequest,
    ) -> Result<LoadRecentPagesResult, NotionLiveError> {
        let search_context = self.workspace_context.cached_search_context()?;
        crate::live::load_recent_pages(session, &search_context, request)
    }

    pub(super) fn search_workspace_once(
        &self,
        session: &NotionDesktopSession,
        request: SearchWorkspaceRequest,
    ) -> Result<SearchWorkspaceResult, NotionLiveError> {
        let search_context = self.workspace_context.cached_search_context()?;
        crate::live::search_workspace(session, &search_context, request)
    }
}
