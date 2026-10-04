use crate::model::{
    BoardSnapshot, CardPage, CreateCalendarPageRequest, CreateCustomEmojiPageIconRequest,
    DatabaseViewControlMutationRequest, DatabaseViewFilterMutationRequest,
    DatabaseViewFilterQueryRequest, DatabaseViewFilterSaveRequest, EditPageBlockTextRequest,
    LoadCalendarItemsRequest, LoadCalendarItemsResult, LoadDatabaseFilterUsersResult,
    LoadNotionPageSharingRequest, LoadRecentPagesRequest, LoadRecentPagesResult,
    LoadSidebarCalendarRequest, LoadSidebarCalendarResult, LoadSidebarChatsRequest,
    LoadSidebarChatsResult, LoadSidebarChildrenRequest, LoadSidebarChildrenResult,
    LoadSidebarInboxRequest, LoadSidebarInboxResult, MoveCardRequest,
    MutateNotionPageSharingRequest, MutateSidebarInboxRequest, NotionCollectionViewId,
    NotionCommentMutationRequest, NotionCommentWorkspaceUsers, NotionCustomEmojiLibrary,
    NotionPageSharingSnapshot, NotionWorkspaceBootstrap, NotionWorkspaceLoad, PageMutationRequest,
    PageMutationResult, PagePresenceSnapshot, PageShellIcon, QuickFindLocalSearchCache,
    RecentPageResult, SearchDatabaseFilterRelationPagesRequest,
    SearchDatabaseFilterRelationPagesResult, SearchWorkspaceRequest, SearchWorkspaceResult,
    SetCalendarPageDateRangeRequest, SetCalendarPageDateRequest, SetDatabaseStatusPropertyRequest,
    UploadPageIconRequest,
};

mod failure;

pub use failure::NotionWorkspaceOperationFailure;

pub type NotionWorkspaceResult<T> = Result<T, NotionWorkspaceOperationFailure>;

pub trait NotionWorkspaceApi: Send + Sync + 'static {
    fn load_card_page(&self, block_id: &str) -> NotionWorkspaceResult<CardPage>;
    fn load_card_page_preview(&self, block_id: &str) -> NotionWorkspaceResult<CardPage> {
        self.load_card_page(block_id)
    }
    fn load_comment_users(&self) -> NotionWorkspaceResult<NotionCommentWorkspaceUsers> {
        Err(
            "Notion comment-user loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn mutate_page_comment(
        &self,
        request: NotionCommentMutationRequest,
    ) -> NotionWorkspaceResult<CardPage> {
        let _ = request;
        Err(
            "Notion comment mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_page_sharing(
        &self,
        request: LoadNotionPageSharingRequest,
    ) -> NotionWorkspaceResult<NotionPageSharingSnapshot> {
        let _ = request;
        Err("Notion sharing is unavailable for this workspace API"
            .to_string()
            .into())
    }
    fn mutate_page_sharing(
        &self,
        request: MutateNotionPageSharingRequest,
    ) -> NotionWorkspaceResult<NotionPageSharingSnapshot> {
        let _ = request;
        Err(
            "Notion sharing mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_inline_database(
        &self,
        collection_view_block_id: &str,
    ) -> NotionWorkspaceResult<NotionWorkspaceBootstrap> {
        let _ = collection_view_block_id;
        Err(
            "Notion inline database loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_inline_database_view(
        &self,
        collection_view_block_id: &str,
        provider_view_id: &NotionCollectionViewId,
    ) -> NotionWorkspaceResult<NotionWorkspaceBootstrap> {
        let _ = (collection_view_block_id, provider_view_id);
        Err(
            "Notion inline database view loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn mutate_database_view_filter(
        &self,
        request: DatabaseViewFilterMutationRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion database filter mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn mutate_database_view_control(
        &self,
        request: DatabaseViewControlMutationRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion database view control mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn save_database_view_filter_state(
        &self,
        request: DatabaseViewFilterSaveRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion database filter save is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn query_database_view_filter(
        &self,
        request: DatabaseViewFilterQueryRequest,
    ) -> NotionWorkspaceResult<NotionWorkspaceLoad> {
        let _ = request;
        Err(
            "Notion database filter querying is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn set_database_status_property(
        &self,
        request: SetDatabaseStatusPropertyRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion database status property editing is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_database_filter_users(&self) -> NotionWorkspaceResult<LoadDatabaseFilterUsersResult> {
        Err(
            "Notion database filter user loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn search_database_filter_relation_pages(
        &self,
        request: SearchDatabaseFilterRelationPagesRequest,
    ) -> NotionWorkspaceResult<SearchDatabaseFilterRelationPagesResult> {
        let _ = request;
        Err(
            "Notion database filter relation search is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_date_undated_count(&self) -> NotionWorkspaceResult<usize> {
        Err(
            "Notion no-date count loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_calendar_items(
        &self,
        request: LoadCalendarItemsRequest,
    ) -> NotionWorkspaceResult<LoadCalendarItemsResult> {
        let _ = request;
        Err(
            "Notion Calendar item loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn create_calendar_page(
        &self,
        request: CreateCalendarPageRequest,
    ) -> NotionWorkspaceResult<String> {
        let _ = request;
        Err(
            "Notion Calendar page creation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn set_calendar_page_date(
        &self,
        request: SetCalendarPageDateRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion Calendar date mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn set_calendar_page_date_range(
        &self,
        request: SetCalendarPageDateRangeRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion Calendar date range mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_page_presence(&self) -> NotionWorkspaceResult<Option<PagePresenceSnapshot>> {
        Ok(None)
    }
    fn hydrate_sidebar(&self) -> NotionWorkspaceResult<()> {
        Err(
            "Notion sidebar hydration is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_sidebar_children(
        &self,
        request: LoadSidebarChildrenRequest,
    ) -> NotionWorkspaceResult<LoadSidebarChildrenResult> {
        let _ = request;
        Err(
            "Notion sidebar child loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_sidebar_calendar(
        &self,
        request: LoadSidebarCalendarRequest,
    ) -> NotionWorkspaceResult<LoadSidebarCalendarResult> {
        let _ = request;
        Err(
            "Notion sidebar calendar loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_sidebar_chats(
        &self,
        request: LoadSidebarChatsRequest,
    ) -> NotionWorkspaceResult<LoadSidebarChatsResult> {
        let _ = request;
        Err(
            "Notion sidebar chat loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_sidebar_inbox(
        &self,
        request: LoadSidebarInboxRequest,
    ) -> NotionWorkspaceResult<LoadSidebarInboxResult> {
        let _ = request;
        Err(
            "Notion sidebar inbox loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn mutate_sidebar_inbox(
        &self,
        request: MutateSidebarInboxRequest,
    ) -> NotionWorkspaceResult<()> {
        let _ = request;
        Err(
            "Notion sidebar inbox mutation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_recent_pages(
        &self,
        request: LoadRecentPagesRequest,
    ) -> NotionWorkspaceResult<LoadRecentPagesResult> {
        let _ = request;
        Err(
            "Notion recent-page loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn cached_recent_pages(&self) -> Option<LoadRecentPagesResult> {
        None
    }
    fn cached_quick_find_local_search(&self) -> QuickFindLocalSearchCache {
        QuickFindLocalSearchCache::default()
    }
    fn quick_find_cache_scope(&self) -> Option<String> {
        None
    }
    fn record_recent_page_visit(&self, recent_page: RecentPageResult) -> NotionWorkspaceResult<()> {
        let _ = recent_page;
        Ok(())
    }
    fn search_workspace(
        &self,
        request: SearchWorkspaceRequest,
    ) -> NotionWorkspaceResult<SearchWorkspaceResult> {
        let _ = request;
        Err(
            "Notion workspace search is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn record_accepted_quick_find_query_response(
        &self,
        request: &SearchWorkspaceRequest,
        result: &SearchWorkspaceResult,
    ) {
        let _ = (request, result);
    }
    fn invalidate_quick_find_page_mutation_in_memory(&self, page_block_id: &str) {
        let _ = page_block_id;
    }
    fn invalidate_quick_find_page_mutation(&self, page_block_id: &str) {
        let _ = page_block_id;
    }
    fn apply_page_mutation(
        &self,
        request: PageMutationRequest,
    ) -> NotionWorkspaceResult<PageMutationResult> {
        let _ = request;
        Err("Notion page mutation is unavailable for this workspace API"
            .to_string()
            .into())
    }
    fn edit_page_block_text(
        &self,
        request: EditPageBlockTextRequest,
    ) -> NotionWorkspaceResult<PageMutationResult> {
        let _ = request;
        Err(
            "Notion page text editing is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn upload_page_icon(
        &self,
        request: UploadPageIconRequest,
    ) -> NotionWorkspaceResult<PageShellIcon> {
        let _ = request;
        Err(
            "Notion page-icon uploading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn create_custom_emoji_page_icon(
        &self,
        request: CreateCustomEmojiPageIconRequest,
    ) -> NotionWorkspaceResult<PageShellIcon> {
        let _ = request;
        Err(
            "Notion custom-emoji creation is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn load_custom_emoji_library(&self) -> NotionWorkspaceResult<NotionCustomEmojiLibrary> {
        Err(
            "Notion custom-emoji loading is unavailable for this workspace API"
                .to_string()
                .into(),
        )
    }
    fn cache_favorited(&self, _is_favorited: bool) -> NotionWorkspaceResult<Option<bool>> {
        Ok(None)
    }
    fn flush_favorite_mutations(&self) -> NotionWorkspaceResult<Vec<String>> {
        Ok(Vec::new())
    }
    fn synchronize_workspace_snapshot(
        &self,
        _snapshot: &mut BoardSnapshot,
    ) -> NotionWorkspaceResult<()> {
        Ok(())
    }
    fn prefetch_notion_workspace(&self, _board_url: &str) -> NotionWorkspaceResult<()> {
        Ok(())
    }
    fn set_favorited(&self, is_favorited: bool) -> NotionWorkspaceResult<()>;
    fn create_page_in_column(&self, target_column_title: &str) -> NotionWorkspaceResult<String>;
    fn move_card(&self, request: MoveCardRequest) -> NotionWorkspaceResult<()>;
    fn load_notion_workspace(&self, board_url: &str) -> NotionWorkspaceResult<NotionWorkspaceLoad>;
}
