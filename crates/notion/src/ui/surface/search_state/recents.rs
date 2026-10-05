use crate::model::{
    notion_page_identity_key, quick_find_local_query_key, LoadRecentPagesResult,
    PageShellSearchResult, RecentPageResult, RecentPageVisit,
};
use crate::ui::Arc;

use super::{
    local::{contextualize_search_result, normalized_notion_search_query},
    NotionSearchLocalRequest, NotionSearchRecentsCache, NotionSearchRequestToken,
    NotionSearchResultsState, NotionSearchState,
};
impl NotionSearchState {
    pub(crate) fn record_recent_page(&mut self, recent_page: RecentPageResult) {
        if self.page_mutation_is_dirty(&recent_page.page.block_id) {
            return;
        }
        if self.recents_refresh_token.is_some() {
            self.recents_dirty_during_refresh
                .insert(notion_page_identity_key(&recent_page.page.block_id));
        }
        let (refreshed_at, existing) = match &self.recents_cache {
            NotionSearchRecentsCache::Loaded {
                refreshed_at,
                results,
            } => (*refreshed_at, results.to_vec()),
            NotionSearchRecentsCache::Empty => (None, Vec::new()),
        };
        self.recents_cache = NotionSearchRecentsCache::Loaded {
            refreshed_at,
            results: LoadRecentPagesResult::merge_results(existing, vec![recent_page]).into(),
        };
    }

    pub(crate) fn recent_pages_for_boosting(&self) -> Vec<RecentPageVisit> {
        let NotionSearchRecentsCache::Loaded { results, .. } = &self.recents_cache else {
            return Vec::new();
        };
        results
            .iter()
            .take(50)
            .map(RecentPageResult::recent_page_visit)
            .collect()
    }

    pub(crate) fn local_result_ids_for_query(
        &mut self,
        token: NotionSearchRequestToken,
    ) -> Vec<String> {
        if !self.request_is_current(token) {
            return Vec::new();
        }
        let query = normalized_notion_search_query(&self.query);
        let query_key = quick_find_local_query_key(self.scope, &query);
        let query_was_indexed = self.local_search_cache.query_is_indexed(&query_key);
        let results: Arc<[PageShellSearchResult]> = self.matching_local_pages(&query).into();
        let excluded_block_ids = if query_was_indexed {
            results
                .iter()
                .map(|result| result.block_id.clone())
                .collect()
        } else {
            Vec::new()
        };
        self.local_request = Some(NotionSearchLocalRequest {
            token,
            query_was_indexed,
            results,
        });
        excluded_block_ids
    }

    pub(crate) fn show_query_seed(&mut self) -> bool {
        let query = normalized_notion_search_query(&self.query);
        if let Some(entry) = self
            .query_cache
            .iter()
            .find(|entry| entry.query == query && entry.scope == self.scope)
        {
            let total = entry.total;
            let visible_limit = self.result_limit as usize + entry.local_result_count;
            let has_more = entry.has_more || entry.results.len() > visible_limit;
            let results: Arc<[PageShellSearchResult]> = entry
                .results
                .iter()
                .take(visible_limit)
                .map(|result| {
                    contextualize_search_result(
                        result,
                        self.current_page_id.as_deref(),
                        self.current_page_url.as_deref(),
                    )
                })
                .collect();
            self.show_query_rows(results.len(), false);
            self.results = NotionSearchResultsState::Loaded {
                total,
                has_more,
                results,
            };
            return true;
        }
        let results = self.matching_local_pages(&query);
        if results.is_empty() {
            return false;
        }
        let total = results.len() as u32;
        self.show_query_rows(results.len(), false);
        self.results = NotionSearchResultsState::Loaded {
            total,
            has_more: false,
            results: results.into(),
        };
        true
    }
}
