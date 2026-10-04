use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard},
};

use crate::{
    model::{
        BoardSnapshot, CardPage, LoadRecentPagesRequest, LoadRecentPagesResult, MoveCardRequest,
        NotionWorkspaceApi, NotionWorkspaceLoad, NotionWorkspaceResult, PageShellSearchResult,
        QuickFindLocalSearchCache, RecentPageResult, SearchWorkspaceRequest, SearchWorkspaceResult,
        SearchWorkspaceScope,
    },
    ui::tests::cases::quick_find::support::{card_page_for_result, recent_page_results},
};

#[derive(Clone)]
pub(super) struct RecordingQuickFindApi {
    state: Arc<Mutex<RecordingQuickFindApiState>>,
}

struct RecordingQuickFindApiState {
    recent_responses: VecDeque<LoadRecentPagesResult>,
    search_responses: VecDeque<SearchWorkspaceResult>,
    pages: HashMap<String, CardPage>,
    workspaces: HashMap<String, BoardSnapshot>,
    recent_requests: Vec<LoadRecentPagesRequest>,
    search_requests: Vec<SearchWorkspaceRequest>,
    page_requests: Vec<String>,
    workspace_requests: Vec<String>,
    cached_recent_pages: Option<LoadRecentPagesResult>,
    cached_local_search: QuickFindLocalSearchCache,
    quick_find_cache_scope: Option<String>,
    recorded_recent_visits: Vec<RecentPageResult>,
    in_memory_page_mutation_invalidations: Vec<String>,
    persisted_page_mutation_invalidations: Vec<String>,
}

impl RecordingQuickFindApi {
    pub(super) fn new(
        recent_responses: Vec<Vec<PageShellSearchResult>>,
        search_responses: Vec<SearchWorkspaceResult>,
        additional_preview_results: &[PageShellSearchResult],
        workspaces: HashMap<String, BoardSnapshot>,
    ) -> Arc<Self> {
        let mut pages = HashMap::new();
        for result in recent_responses
            .iter()
            .flatten()
            .chain(
                search_responses
                    .iter()
                    .flat_map(|response| response.results.iter()),
            )
            .chain(additional_preview_results)
        {
            pages
                .entry(result.block_id.clone())
                .or_insert_with(|| card_page_for_result(result));
        }
        Arc::new(Self {
            state: Arc::new(Mutex::new(RecordingQuickFindApiState {
                recent_responses: recent_responses
                    .into_iter()
                    .map(|results| LoadRecentPagesResult {
                        results: recent_page_results(results),
                    })
                    .collect(),
                search_responses: search_responses.into(),
                pages,
                workspaces,
                recent_requests: Vec::new(),
                search_requests: Vec::new(),
                page_requests: Vec::new(),
                workspace_requests: Vec::new(),
                cached_recent_pages: None,
                cached_local_search: QuickFindLocalSearchCache::default(),
                quick_find_cache_scope: Some("quick-find-test-workspace".to_string()),
                recorded_recent_visits: Vec::new(),
                in_memory_page_mutation_invalidations: Vec::new(),
                persisted_page_mutation_invalidations: Vec::new(),
            })),
        })
    }

    fn lock(&self) -> MutexGuard<'_, RecordingQuickFindApiState> {
        self.state
            .lock()
            .expect("Quick Find test API mutex poisoned")
    }

    pub(super) fn recent_requests(&self) -> Vec<LoadRecentPagesRequest> {
        self.lock().recent_requests.clone()
    }

    pub(super) fn search_requests(&self) -> Vec<SearchWorkspaceRequest> {
        self.lock().search_requests.clone()
    }

    pub(super) fn workspace_requests(&self) -> Vec<String> {
        self.lock().workspace_requests.clone()
    }

    pub(super) fn page_requests(&self) -> Vec<String> {
        self.lock().page_requests.clone()
    }

    pub(super) fn recorded_recent_visits(&self) -> Vec<RecentPageResult> {
        self.lock().recorded_recent_visits.clone()
    }

    pub(super) fn page_mutation_invalidations(&self) -> (Vec<String>, Vec<String>) {
        let state = self.lock();
        (
            state.in_memory_page_mutation_invalidations.clone(),
            state.persisted_page_mutation_invalidations.clone(),
        )
    }

    pub(super) fn set_persisted_recents(
        &self,
        cache_scope: Option<&str>,
        results: Vec<RecentPageResult>,
    ) {
        let mut state = self.lock();
        for result in &results {
            state
                .pages
                .entry(result.page.block_id.clone())
                .or_insert_with(|| card_page_for_result(&result.page));
        }
        state.quick_find_cache_scope = cache_scope.map(str::to_string);
        state.cached_recent_pages = Some(LoadRecentPagesResult { results });
    }

    pub(super) fn set_recent_responses(&self, responses: Vec<Vec<RecentPageResult>>) {
        self.lock().recent_responses = responses
            .into_iter()
            .map(|results| LoadRecentPagesResult { results })
            .collect();
    }

    pub(super) fn set_local_search_cache(&self, cache: QuickFindLocalSearchCache) {
        self.lock().cached_local_search = cache;
    }

    pub(super) fn set_page(&self, page: CardPage) {
        self.lock().pages.insert(page.block_id.clone(), page);
    }
}

pub(super) fn indexed_local_search_cache(
    query: &str,
    pages: Vec<PageShellSearchResult>,
) -> QuickFindLocalSearchCache {
    let mut cache = QuickFindLocalSearchCache::default();
    cache.record_complete_query_results(
        &crate::model::quick_find_local_query_key(SearchWorkspaceScope::AllContent, query),
        pages,
    );
    cache
}

impl NotionWorkspaceApi for RecordingQuickFindApi {
    fn load_card_page(&self, block_id: &str) -> NotionWorkspaceResult<CardPage> {
        let mut state = self.lock();
        state.page_requests.push(block_id.to_string());
        let page = state
            .pages
            .get(block_id)
            .cloned()
            .unwrap_or_else(|| panic!("unexpected Quick Find preview request `{block_id}`"));
        Ok(page)
    }

    fn load_recent_pages(
        &self,
        request: LoadRecentPagesRequest,
    ) -> NotionWorkspaceResult<LoadRecentPagesResult> {
        let mut state = self.lock();
        state.recent_requests.push(request);
        Ok(state
            .recent_responses
            .pop_front()
            .unwrap_or_else(|| LoadRecentPagesResult {
                results: Vec::new(),
            }))
    }

    fn cached_recent_pages(&self) -> Option<LoadRecentPagesResult> {
        self.lock().cached_recent_pages.clone()
    }

    fn cached_quick_find_local_search(&self) -> QuickFindLocalSearchCache {
        self.lock().cached_local_search.clone()
    }

    fn quick_find_cache_scope(&self) -> Option<String> {
        self.lock().quick_find_cache_scope.clone()
    }

    fn record_recent_page_visit(&self, recent_page: RecentPageResult) -> NotionWorkspaceResult<()> {
        let mut state = self.lock();
        state.recorded_recent_visits.push(recent_page.clone());
        let cached = state
            .cached_recent_pages
            .take()
            .map_or_else(Vec::new, |cached| cached.results);
        state.cached_recent_pages = Some(LoadRecentPagesResult {
            results: LoadRecentPagesResult::merge_results(cached, vec![recent_page]),
        });
        Ok(())
    }

    fn search_workspace(
        &self,
        request: SearchWorkspaceRequest,
    ) -> NotionWorkspaceResult<SearchWorkspaceResult> {
        let mut state = self.lock();
        state.search_requests.push(request);
        Ok(state
            .search_responses
            .pop_front()
            .expect("unexpected Quick Find search request"))
    }

    fn invalidate_quick_find_page_mutation_in_memory(&self, page_block_id: &str) {
        self.lock()
            .in_memory_page_mutation_invalidations
            .push(page_block_id.to_string());
    }

    fn invalidate_quick_find_page_mutation(&self, page_block_id: &str) {
        self.lock()
            .persisted_page_mutation_invalidations
            .push(page_block_id.to_string());
    }

    fn set_favorited(&self, _is_favorited: bool) -> NotionWorkspaceResult<()> {
        panic!("unexpected Quick Find favorite mutation")
    }

    fn create_page_in_column(&self, _target_column_title: &str) -> NotionWorkspaceResult<String> {
        panic!("unexpected Quick Find page creation")
    }

    fn move_card(&self, _request: MoveCardRequest) -> NotionWorkspaceResult<()> {
        panic!("unexpected Quick Find card move")
    }

    fn load_notion_workspace(&self, board_url: &str) -> NotionWorkspaceResult<NotionWorkspaceLoad> {
        let workspace = {
            let mut state = self.lock();
            state.workspace_requests.push(board_url.to_string());
            state
                .workspaces
                .get(board_url)
                .cloned()
                .unwrap_or_else(|| panic!("unexpected Quick Find navigation `{board_url}`"))
        };
        Ok(NotionWorkspaceLoad {
            workspace,
            workspace_api: Arc::new(self.clone()),
            code_settings: crate::model::CardPageCodeSettingsCapability::memory(),
        })
    }
}
