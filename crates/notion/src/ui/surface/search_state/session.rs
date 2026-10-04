use std::time::Instant;

use gpui::ScrollHandle;

use crate::model::{
    notion_page_identity_key, LoadRecentPagesResult, PageShellSearchResult,
    QuickFindLocalSearchCache, RecentPageResult,
};

use super::{
    next_notion_search_request_token, NotionSearchPreviewState, NotionSearchRecentsCache,
    NotionSearchRecentsCommit, NotionSearchRequestToken, NotionSearchResultsState,
    NotionSearchState,
};
impl NotionSearchState {
    pub(crate) fn visible_results(&self) -> &[PageShellSearchResult] {
        match &self.results {
            NotionSearchResultsState::Loaded { results, .. } => results,
            NotionSearchResultsState::Idle
            | NotionSearchResultsState::Loading
            | NotionSearchResultsState::Failed => &[],
        }
    }

    pub(crate) fn begin_session(&mut self, session_id: String) {
        let recents_cache = self.recents_cache.clone();
        let recents_refresh_token = self.recents_refresh_token;
        let recents_dirty_during_refresh = self.recents_dirty_during_refresh.clone();
        let cache_scope = self.cache_scope.clone();
        let recents_snapshot_prepared = self.recents_snapshot_prepared;
        let local_search_snapshot_prepared = self.local_search_snapshot_prepared;
        let current_page_id = self.current_page_id.clone();
        let current_page_url = self.current_page_url.clone();
        let local_search_cache = self.local_search_cache.clone();
        let query_cache = self.query_cache.clone();
        let preview_cache = self.preview_cache.clone();
        let preview_requests = self.preview_requests.clone();
        let dirty_page_identities = self.dirty_page_identities.clone();
        *self = Self {
            session_id,
            recents_cache,
            recents_refresh_token,
            recents_dirty_during_refresh,
            cache_scope,
            recents_snapshot_prepared,
            local_search_snapshot_prepared,
            current_page_id,
            current_page_url,
            local_search_cache,
            query_cache,
            preview_cache,
            preview_requests,
            dirty_page_identities,
            ..Self::default()
        };
    }

    pub(crate) fn prepare_recents_cache(
        &mut self,
        cache_scope: Option<String>,
        current_page_id: Option<String>,
        current_page_url: Option<String>,
        persisted_results: Option<Vec<RecentPageResult>>,
    ) {
        self.current_page_id = current_page_id;
        self.current_page_url = current_page_url;
        if self.cache_scope != cache_scope {
            self.cache_scope = cache_scope;
            self.recents_cache = NotionSearchRecentsCache::Empty;
            self.recents_refresh_token = None;
            self.recents_dirty_during_refresh.clear();
            self.recents_snapshot_prepared = false;
            self.local_search_snapshot_prepared = false;
            self.local_search_cache = QuickFindLocalSearchCache::default();
            self.local_request = None;
            self.query_cache.clear();
            self.preview_cache.clear();
            self.preview_requests.clear();
            self.preview_scroll_handle = ScrollHandle::new();
            self.request_token = next_notion_search_request_token();
        }
        if self.recents_snapshot_prepared {
            return;
        }
        self.recents_snapshot_prepared = true;
        let Some(mut persisted_results) = persisted_results else {
            return;
        };
        persisted_results.retain(|result| !self.page_mutation_is_dirty(&result.page.block_id));
        let (refreshed_at, existing) = match &self.recents_cache {
            NotionSearchRecentsCache::Loaded {
                refreshed_at,
                results,
            } => (*refreshed_at, results.to_vec()),
            NotionSearchRecentsCache::Empty => (None, Vec::new()),
        };
        self.recents_cache = NotionSearchRecentsCache::Loaded {
            refreshed_at,
            results: LoadRecentPagesResult::merge_results(existing, persisted_results).into(),
        };
    }

    pub(crate) fn prepare_local_search_cache(&mut self, mut cache: QuickFindLocalSearchCache) {
        if self.local_search_snapshot_prepared {
            return;
        }
        self.local_search_snapshot_prepared = true;
        for identity in self
            .dirty_page_identities
            .iter()
            .map(|(_, identity)| identity)
        {
            cache.invalidate_page_mutation(identity);
        }
        self.local_search_cache.merge_from(cache);
    }

    #[cfg(test)]
    pub(crate) fn replace_local_search_cache_for_test(&mut self, cache: QuickFindLocalSearchCache) {
        self.local_search_cache = cache;
        self.local_search_snapshot_prepared = true;
    }

    pub(crate) fn begin_request(&mut self) -> NotionSearchRequestToken {
        self.pagination_request = None;
        self.local_request = None;
        self.request_token = next_notion_search_request_token();
        self.query_request_dispatched = false;
        self.request_token
    }

    pub(crate) fn begin_pagination_request(
        &mut self,
        page_size: u32,
    ) -> Option<NotionSearchRequestToken> {
        if self.pagination_request.is_some() {
            return None;
        }
        self.request_token = next_notion_search_request_token();
        self.local_request = None;
        self.query_request_dispatched = false;
        let previous_limit = self.result_limit;
        self.result_limit = self.result_limit.saturating_add(page_size);
        self.pagination_request = Some((self.request_token, previous_limit));
        Some(self.request_token)
    }

    pub(crate) fn pagination_is_loading(&self) -> bool {
        self.pagination_request.is_some()
    }

    pub(crate) fn mark_query_request_dispatched(&mut self, token: NotionSearchRequestToken) {
        if self.request_is_current(token) {
            self.query_request_dispatched = true;
        }
    }

    pub(crate) fn request_is_current(&self, token: NotionSearchRequestToken) -> bool {
        self.request_token == token
    }

    pub(crate) fn begin_preview_request(&mut self, block_id: String) -> NotionSearchRequestToken {
        let token = next_notion_search_request_token();
        self.preview_requests.insert(block_id, token);
        token
    }

    pub(crate) fn pending_preview_request(
        &self,
        block_id: &str,
    ) -> Option<NotionSearchRequestToken> {
        self.preview_requests.get(block_id).copied()
    }

    pub(crate) fn finish_preview_request(
        &mut self,
        block_id: &str,
        token: NotionSearchRequestToken,
    ) -> bool {
        if self.preview_requests.get(block_id) != Some(&token) {
            return false;
        }
        self.preview_requests.remove(block_id);
        true
    }

    pub(crate) fn begin_recents_refresh(
        &mut self,
        search_open: bool,
    ) -> (Option<NotionSearchRequestToken>, bool) {
        let showing_cached_recents = search_open && self.show_cached_recents();
        if self.recents_refresh_token.is_some() {
            if search_open && !showing_cached_recents {
                self.results = NotionSearchResultsState::Loading;
                self.reset_list(6);
            }
            return (None, showing_cached_recents);
        }
        if self.recents_are_fresh() {
            return (None, showing_cached_recents);
        }
        let token = next_notion_search_request_token();
        self.recents_refresh_token = Some(token);
        self.recents_dirty_during_refresh.clear();
        if search_open && !showing_cached_recents {
            self.results = NotionSearchResultsState::Loading;
            self.reset_list(6);
        }
        (Some(token), showing_cached_recents)
    }

    pub(crate) fn commit_recents_refresh(
        &mut self,
        token: NotionSearchRequestToken,
        mut results: Vec<RecentPageResult>,
        search_open: bool,
    ) -> NotionSearchRecentsCommit {
        if self.recents_refresh_token != Some(token) {
            return NotionSearchRecentsCommit::Ignored;
        }
        results.retain(|result| !self.page_mutation_is_dirty(&result.page.block_id));
        self.recents_refresh_token = None;
        let concurrent_visits = match &self.recents_cache {
            NotionSearchRecentsCache::Loaded { results, .. } => results.to_vec(),
            NotionSearchRecentsCache::Empty => Vec::new(),
        }
        .into_iter()
        .filter(|result| {
            !self.page_mutation_is_dirty(&result.page.block_id)
                && self
                    .recents_dirty_during_refresh
                    .contains(&notion_page_identity_key(&result.page.block_id))
        })
        .collect();
        self.recents_dirty_during_refresh.clear();
        let refreshed_results = LoadRecentPagesResult::merge_results(results, concurrent_visits);
        self.local_search_cache
            .invalidate_query_authority(refreshed_results.iter().map(|result| result.page.clone()));
        self.recents_snapshot_prepared = true;
        self.local_search_snapshot_prepared = true;
        self.recents_cache = NotionSearchRecentsCache::Loaded {
            refreshed_at: Some(Instant::now()),
            results: refreshed_results.into(),
        };
        if !search_open {
            return NotionSearchRecentsCommit::Ignored;
        }
        if !self.query.trim().is_empty() {
            let query_request_dispatched = self.query_request_dispatched;
            self.reset_selected_index();
            if !self.show_query_seed() {
                self.results = NotionSearchResultsState::Loading;
                self.preview = NotionSearchPreviewState::Idle;
                self.reset_list(6);
            }
            return if query_request_dispatched {
                NotionSearchRecentsCommit::RequeryImmediately
            } else {
                NotionSearchRecentsCommit::AwaitingDebouncedQuery
            };
        }
        if self.show_cached_recents() {
            NotionSearchRecentsCommit::ShowingRecents
        } else {
            NotionSearchRecentsCommit::Ignored
        }
    }

    pub(crate) fn finish_recents_failure(
        &mut self,
        token: NotionSearchRequestToken,
        search_open: bool,
    ) -> bool {
        if self.recents_refresh_token != Some(token) {
            return false;
        }
        self.recents_refresh_token = None;
        self.recents_dirty_during_refresh.clear();
        if !search_open || !self.query.trim().is_empty() {
            return false;
        }
        if self.show_cached_recents() {
            return true;
        }
        self.results = NotionSearchResultsState::Failed;
        self.reset_list(0);
        true
    }

    pub(crate) fn recents_refresh_is_current(&self, token: NotionSearchRequestToken) -> bool {
        self.recents_refresh_token == Some(token)
    }

    pub(crate) fn inherit_completed_recents(&mut self, previous: &Self) {
        self.recents_cache.clone_from(&previous.recents_cache);
        self.recents_refresh_token = None;
        self.recents_dirty_during_refresh.clear();
        self.cache_scope.clone_from(&previous.cache_scope);
        self.recents_snapshot_prepared = previous.recents_snapshot_prepared;
        self.local_search_snapshot_prepared = previous.local_search_snapshot_prepared;
        self.local_search_cache
            .clone_from(&previous.local_search_cache);
        self.dirty_page_identities
            .clone_from(&previous.dirty_page_identities);
        self.current_page_id.clone_from(&previous.current_page_id);
        self.current_page_url.clone_from(&previous.current_page_url);
    }
}
