mod calendar;
mod comments;
mod database;
mod icons;
mod page;
mod sharing;
mod sidebar;
mod workspace;

use crate::model::{
    CardPage, CreateCalendarPageRequest, CreateCustomEmojiPageIconRequest,
    DatabaseViewControlMutationRequest, DatabaseViewFilterMutationRequest,
    DatabaseViewFilterQueryRequest, DatabaseViewFilterSaveRequest, EditPageBlockTextRequest,
    LoadCalendarItemsRequest, LoadCalendarItemsResult, LoadDatabaseFilterUsersResult,
    LoadNotionPageSharingRequest, LoadRecentPagesRequest, LoadRecentPagesResult,
    LoadSidebarCalendarRequest, LoadSidebarCalendarResult, LoadSidebarChatsRequest,
    LoadSidebarChatsResult, LoadSidebarChildrenRequest, LoadSidebarChildrenResult,
    LoadSidebarInboxRequest, LoadSidebarInboxResult, MoveCardRequest,
    MutateNotionPageSharingRequest, MutateSidebarInboxRequest, NotionCollectionViewId,
    NotionCommentMutationRequest, NotionCommentWorkspaceUsers, NotionCustomEmojiLibrary,
    NotionPageSharingSnapshot, NotionWorkspaceBootstrap, NotionWorkspaceLoad,
    NotionWorkspaceResult, PageMutationRequest, PageMutationResult, PagePresenceSnapshot,
    QuickFindLocalSearchCache, RecentPageResult, SearchDatabaseFilterRelationPagesRequest,
    SearchDatabaseFilterRelationPagesResult, SearchWorkspaceRequest, SearchWorkspaceResult,
    SetCalendarPageDateRangeRequest, SetCalendarPageDateRequest, SetDatabaseStatusPropertyRequest,
    UploadPageIconRequest,
};

use super::NotionWorkspaceRuntime;

impl crate::model::NotionWorkspaceApi for NotionWorkspaceRuntime {
    fn load_card_page(&self, block_id: &str) -> NotionWorkspaceResult<CardPage> {
        self.with_session_recovery(|session| self.load_card_page_once(session, block_id))
    }

    fn load_card_page_preview(&self, block_id: &str) -> NotionWorkspaceResult<CardPage> {
        self.with_session_recovery(|session| self.load_card_page_preview_once(session, block_id))
    }

    fn load_page_sharing(
        &self,
        request: LoadNotionPageSharingRequest,
    ) -> NotionWorkspaceResult<NotionPageSharingSnapshot> {
        self.with_session_recovery(|session| self.load_page_sharing_once(session, request.clone()))
    }

    fn load_comment_users(&self) -> NotionWorkspaceResult<NotionCommentWorkspaceUsers> {
        self.with_session_recovery(|session| self.load_comment_users_once(session))
    }

    fn mutate_page_comment(
        &self,
        request: NotionCommentMutationRequest,
    ) -> NotionWorkspaceResult<CardPage> {
        self.with_session_recovery(|session| {
            self.mutate_page_comment_once(session, request.clone())
        })
    }

    fn mutate_page_sharing(
        &self,
        request: MutateNotionPageSharingRequest,
    ) -> NotionWorkspaceResult<NotionPageSharingSnapshot> {
        self.with_session_recovery(|session| {
            self.mutate_page_sharing_once(session, request.clone())
        })
    }

    fn load_inline_database(
        &self,
        collection_view_block_id: &str,
    ) -> NotionWorkspaceResult<NotionWorkspaceBootstrap> {
        self.with_session_recovery(|session| {
            self.load_inline_database_from_cached_bootstrap(session, collection_view_block_id, None)
        })
    }

    fn load_inline_database_view(
        &self,
        collection_view_block_id: &str,
        provider_view_id: &NotionCollectionViewId,
    ) -> NotionWorkspaceResult<NotionWorkspaceBootstrap> {
        self.with_session_recovery(|session| {
            self.load_inline_database_from_cached_bootstrap(
                session,
                collection_view_block_id,
                Some(provider_view_id),
            )
        })
    }

    fn mutate_database_view_filter(
        &self,
        request: DatabaseViewFilterMutationRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.mutate_database_view_filter_once(session, request.clone())
        })
    }

    fn mutate_database_view_control(
        &self,
        request: DatabaseViewControlMutationRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.mutate_database_view_control_once(session, request.clone())
        })
    }

    fn save_database_view_filter_state(
        &self,
        request: DatabaseViewFilterSaveRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.save_database_view_filter_state_once(session, request.clone())
        })
    }

    fn query_database_view_filter(
        &self,
        request: DatabaseViewFilterQueryRequest,
    ) -> NotionWorkspaceResult<NotionWorkspaceLoad> {
        self.with_session_recovery(|session| {
            self.query_database_view_filter_once(session, request.clone())
        })
    }

    fn set_database_status_property(
        &self,
        request: SetDatabaseStatusPropertyRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.set_database_status_property_once(session, request.clone())
        })
    }

    fn load_database_filter_users(&self) -> NotionWorkspaceResult<LoadDatabaseFilterUsersResult> {
        self.with_session_recovery(|session| self.load_database_filter_users_once(session))
    }

    fn search_database_filter_relation_pages(
        &self,
        request: SearchDatabaseFilterRelationPagesRequest,
    ) -> NotionWorkspaceResult<SearchDatabaseFilterRelationPagesResult> {
        self.with_session_recovery(|session| {
            self.search_database_filter_relation_pages_once(session, request.clone())
        })
    }

    fn load_date_undated_count(&self) -> NotionWorkspaceResult<usize> {
        self.with_session_recovery(|session| self.load_date_undated_count_once(session))
    }

    fn load_calendar_items(
        &self,
        request: LoadCalendarItemsRequest,
    ) -> NotionWorkspaceResult<LoadCalendarItemsResult> {
        self.with_session_recovery(|session| {
            self.load_calendar_items_once(session, request.clone())
        })
    }

    fn create_calendar_page(
        &self,
        request: CreateCalendarPageRequest,
    ) -> NotionWorkspaceResult<String> {
        self.with_session_recovery(|session| self.create_calendar_page_atomic(session, &request))
    }

    fn set_calendar_page_date(
        &self,
        request: SetCalendarPageDateRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.mutate_calendar_date(session, |state| {
                state.prepare_calendar_date_mutation(&request)
            })
        })
    }

    fn set_calendar_page_date_range(
        &self,
        request: SetCalendarPageDateRangeRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.mutate_calendar_date(session, |state| {
                state.prepare_calendar_date_range_mutation(&request)
            })
        })
    }

    fn load_page_presence(&self) -> NotionWorkspaceResult<Option<PagePresenceSnapshot>> {
        self.with_session_recovery(|session| self.load_page_presence_once(session))
    }

    fn hydrate_sidebar(&self) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| self.hydrate_sidebar_once(session))
    }

    fn load_sidebar_children(
        &self,
        request: LoadSidebarChildrenRequest,
    ) -> NotionWorkspaceResult<LoadSidebarChildrenResult> {
        self.with_session_recovery(|session| {
            self.load_sidebar_children_once(session, request.clone())
        })
    }

    fn load_sidebar_calendar(
        &self,
        request: LoadSidebarCalendarRequest,
    ) -> NotionWorkspaceResult<LoadSidebarCalendarResult> {
        self.with_session_recovery(|session| {
            self.load_sidebar_calendar_once(session, request.clone())
        })
    }

    fn load_sidebar_chats(
        &self,
        request: LoadSidebarChatsRequest,
    ) -> NotionWorkspaceResult<LoadSidebarChatsResult> {
        self.with_session_recovery(|session| self.load_sidebar_chats_once(session, request.clone()))
    }

    fn load_sidebar_inbox(
        &self,
        request: LoadSidebarInboxRequest,
    ) -> NotionWorkspaceResult<LoadSidebarInboxResult> {
        self.with_session_recovery(|session| self.load_sidebar_inbox_once(session, request.clone()))
    }

    fn mutate_sidebar_inbox(
        &self,
        request: MutateSidebarInboxRequest,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.mutate_sidebar_inbox_once(session, request.clone())
        })
    }

    fn load_recent_pages(
        &self,
        request: LoadRecentPagesRequest,
    ) -> NotionWorkspaceResult<LoadRecentPagesResult> {
        let cache_quick_find_results =
            usize::from(request.limit) >= SearchWorkspaceRequest::MAX_RECENT_PAGES_FOR_BOOSTING;
        let refresh = cache_quick_find_results.then(|| self.begin_quick_find_recents_refresh());
        let loaded = self
            .with_session_recovery(|session| self.load_recent_pages_once(session, request.clone()));
        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                if let Some(refresh) = refresh {
                    self.cancel_quick_find_recents_refresh(refresh.token);
                }
                return Err(error);
            }
        };
        if let Some(refresh) = refresh {
            self.commit_quick_find_recents_refresh(loaded.results.clone(), refresh);
        }
        Ok(loaded)
    }

    fn cached_recent_pages(&self) -> Option<LoadRecentPagesResult> {
        self.cached_quick_find_recents()
    }

    fn cached_quick_find_local_search(&self) -> QuickFindLocalSearchCache {
        self.quick_find_local_search_snapshot()
    }

    fn quick_find_cache_scope(&self) -> Option<String> {
        self.workspace_context.cached_space_id().ok()
    }

    fn record_recent_page_visit(&self, recent_page: RecentPageResult) -> NotionWorkspaceResult<()> {
        self.record_quick_find_recents(vec![recent_page]);
        Ok(())
    }

    fn search_workspace(
        &self,
        request: SearchWorkspaceRequest,
    ) -> NotionWorkspaceResult<SearchWorkspaceResult> {
        self.begin_quick_find_query(&request);
        let result = self
            .with_session_recovery(|session| self.search_workspace_once(session, request.clone()));
        if result.is_err() {
            self.cancel_quick_find_query(&request);
        }
        result
    }

    fn record_accepted_quick_find_query_response(
        &self,
        request: &SearchWorkspaceRequest,
        result: &SearchWorkspaceResult,
    ) {
        self.accept_quick_find_query_response(request, result);
    }

    fn invalidate_quick_find_page_mutation_in_memory(&self, page_block_id: &str) {
        self.invalidate_quick_find_page_mutation_cache(page_block_id, false);
    }

    fn invalidate_quick_find_page_mutation(&self, page_block_id: &str) {
        self.invalidate_quick_find_page_mutation_cache(page_block_id, true);
    }

    fn apply_page_mutation(
        &self,
        request: PageMutationRequest,
    ) -> NotionWorkspaceResult<PageMutationResult> {
        self.with_session_recovery(|session| {
            self.apply_page_mutation_once(session, request.clone())
        })
    }

    fn edit_page_block_text(
        &self,
        request: EditPageBlockTextRequest,
    ) -> NotionWorkspaceResult<PageMutationResult> {
        self.with_session_recovery(|session| {
            self.edit_page_block_text_once(session, request.clone())
        })
    }

    fn upload_page_icon(
        &self,
        request: UploadPageIconRequest,
    ) -> NotionWorkspaceResult<crate::model::PageShellIcon> {
        self.with_session_recovery(|session| self.upload_page_icon_once(session, request.clone()))
    }

    fn create_custom_emoji_page_icon(
        &self,
        request: CreateCustomEmojiPageIconRequest,
    ) -> NotionWorkspaceResult<crate::model::PageShellIcon> {
        self.with_session_recovery(|session| {
            self.create_custom_emoji_page_icon_once(session, request.clone())
        })
    }

    fn load_custom_emoji_library(&self) -> NotionWorkspaceResult<NotionCustomEmojiLibrary> {
        self.with_session_recovery(|session| self.load_custom_emoji_library_once(session))
    }

    fn cache_favorited(&self, is_favorited: bool) -> NotionWorkspaceResult<Option<bool>> {
        self.with_session_recovery(|session| self.cache_favorited_once(session, is_favorited))
    }

    fn flush_favorite_mutations(&self) -> NotionWorkspaceResult<Vec<String>> {
        self.with_session_recovery(|session| self.flush_favorite_mutations_once(session))
    }

    fn synchronize_workspace_snapshot(
        &self,
        snapshot: &mut crate::model::BoardSnapshot,
    ) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.synchronize_workspace_snapshot_once(session, snapshot)
        })
    }

    fn prefetch_notion_workspace(&self, board_url: &str) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| {
            self.prefetch_notion_workspace_once(session, board_url)
        })
    }

    fn set_favorited(&self, is_favorited: bool) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| self.set_favorited_once(session, is_favorited))
    }

    fn create_page_in_column(&self, target_column_title: &str) -> NotionWorkspaceResult<String> {
        self.with_session_recovery(|session| {
            self.create_page_in_column_once(session, target_column_title)
        })
    }

    fn move_card(&self, request: MoveCardRequest) -> NotionWorkspaceResult<()> {
        self.with_session_recovery(|session| self.move_card_once(session, request.clone()))
    }

    fn load_notion_workspace(&self, board_url: &str) -> NotionWorkspaceResult<NotionWorkspaceLoad> {
        self.with_session_recovery(|session| self.load_notion_workspace_once(session, board_url))
    }
}
