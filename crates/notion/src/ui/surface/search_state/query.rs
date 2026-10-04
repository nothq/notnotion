use std::collections::{HashMap, HashSet};

use crate::model::{notion_page_identity_key, quick_find_local_query_key, PageShellSearchResult};
use crate::ui::Arc;

use super::{
    local::{contextualize_search_result, normalized_notion_search_query},
    NotionSearchLocalRequest, NotionSearchQueryCacheEntry, NotionSearchRecentsCache,
    NotionSearchRequestToken, NotionSearchResultsState, NotionSearchState,
    NOTION_SEARCH_LOCAL_RESULT_LIMIT, NOTION_SEARCH_QUERY_CACHE_LIMIT,
};
impl NotionSearchState {
    pub(crate) fn commit_query_results(
        &mut self,
        token: NotionSearchRequestToken,
        total: u32,
        consumed_result_count: u32,
        server_results: Vec<PageShellSearchResult>,
    ) -> bool {
        let Some(mut prepared) =
            self.prepare_query_results(token, total, consumed_result_count, server_results)
        else {
            return false;
        };
        let local_results = self.local_results_for_query_response(&mut prepared);
        let display = self.combine_query_response(prepared, local_results);
        self.show_query_response(display);
        true
    }

    fn prepare_query_results(
        &mut self,
        token: NotionSearchRequestToken,
        total: u32,
        consumed_result_count: u32,
        mut server_results: Vec<PageShellSearchResult>,
    ) -> Option<PreparedQueryResults> {
        if !self.request_is_current(token) {
            return None;
        }
        server_results.retain(|result| !self.page_mutation_is_dirty(&result.block_id));
        self.query_request_dispatched = true;
        let preserve_scroll = self
            .pagination_request
            .is_some_and(|(pending_token, _)| pending_token == token);
        if preserve_scroll {
            self.pagination_request = None;
        }
        let query = normalized_notion_search_query(&self.query);
        let query_key = quick_find_local_query_key(self.scope, &query);
        let query_result_details = self.query_result_details(&query, &server_results);
        let local_request = self
            .local_request
            .take()
            .filter(|request| request.token == token);
        let query_was_indexed = local_request
            .as_ref()
            .is_some_and(|request| request.query_was_indexed);
        let preexisting_local_ids = self.preexisting_local_result_ids();
        let request_had_local_exclusions = query_was_indexed
            && local_request
                .as_ref()
                .is_some_and(|request| !request.results.is_empty());
        let response_is_complete = !request_had_local_exclusions && consumed_result_count >= total;
        server_results.truncate(self.result_limit as usize);
        let server_returned_ids = server_results
            .iter()
            .map(|result| notion_page_identity_key(&result.block_id))
            .collect::<HashSet<_>>();
        self.record_query_result_authority(&query_key, response_is_complete, &server_results);
        Some(PreparedQueryResults {
            query,
            total,
            consumed_result_count,
            preserve_scroll,
            query_result_details,
            preexisting_local_ids,
            request_had_local_exclusions,
            query_was_indexed,
            local_request,
            server_results,
            server_returned_ids,
        })
    }

    fn query_result_details(
        &self,
        query: &str,
        server_results: &[PageShellSearchResult],
    ) -> HashMap<String, PageShellSearchResult> {
        let mut details = self
            .query_cache
            .iter()
            .find(|entry| entry.query == query && entry.scope == self.scope)
            .into_iter()
            .flat_map(|entry| entry.results.iter())
            .map(|result| (notion_page_identity_key(&result.block_id), result.clone()))
            .collect::<HashMap<_, _>>();
        details.extend(
            server_results
                .iter()
                .map(|result| (notion_page_identity_key(&result.block_id), result.clone())),
        );
        details
    }

    fn preexisting_local_result_ids(&self) -> HashSet<String> {
        self.local_search_cache
            .pages
            .iter()
            .map(|result| notion_page_identity_key(&result.block_id))
            .chain(match &self.recents_cache {
                NotionSearchRecentsCache::Loaded { results, .. } => results
                    .iter()
                    .map(|result| notion_page_identity_key(&result.page.block_id))
                    .collect::<Vec<_>>(),
                NotionSearchRecentsCache::Empty => Vec::new(),
            })
            .collect()
    }

    fn record_query_result_authority(
        &mut self,
        query_key: &str,
        response_is_complete: bool,
        server_results: &[PageShellSearchResult],
    ) {
        if response_is_complete {
            self.local_search_cache
                .record_complete_query_results(query_key, server_results.iter().cloned());
        } else {
            self.local_search_cache
                .record_partial_query_results(query_key, server_results.iter().cloned());
        }
    }

    fn local_results_for_query_response(
        &self,
        prepared: &mut PreparedQueryResults,
    ) -> Vec<PageShellSearchResult> {
        let local_results = if prepared.query_was_indexed {
            prepared
                .local_request
                .take()
                .map(|request| request.results.to_vec())
                .unwrap_or_default()
        } else {
            self.matching_local_pages_up_to(&prepared.query, usize::MAX)
                .into_iter()
                .filter(|result| {
                    let identity = notion_page_identity_key(&result.block_id);
                    prepared.server_returned_ids.contains(&identity)
                        && (prepared.preexisting_local_ids.contains(&identity)
                            || normalized_notion_search_query(&result.title) == prepared.query)
                })
                .take(NOTION_SEARCH_LOCAL_RESULT_LIMIT)
                .collect()
        };
        local_results
            .into_iter()
            .map(|mut result| {
                let identity = notion_page_identity_key(&result.block_id);
                if !prepared.preexisting_local_ids.contains(&identity) {
                    if let Some(details) = prepared.query_result_details.get(&identity) {
                        result.match_snippet = details.match_snippet.clone();
                    }
                }
                result
            })
            .collect()
    }

    fn combine_query_response(
        &self,
        mut prepared: PreparedQueryResults,
        local_results: Vec<PageShellSearchResult>,
    ) -> QueryDisplayResults {
        let local_result_ids = local_results
            .iter()
            .map(|result| notion_page_identity_key(&result.block_id))
            .collect::<HashSet<_>>();
        let local_server_overlap_count = local_result_ids
            .intersection(&prepared.server_returned_ids)
            .count();
        prepared.server_results.retain(|result| {
            !local_result_ids.contains(&notion_page_identity_key(&result.block_id))
        });
        let mut seen = HashSet::new();
        let combined_results = local_results
            .into_iter()
            .chain(prepared.server_results)
            .map(|result| {
                contextualize_search_result(
                    &result,
                    self.current_page_id.as_deref(),
                    self.current_page_url.as_deref(),
                )
            })
            .filter(|result| seen.insert(notion_page_identity_key(&result.block_id)))
            .collect::<Vec<_>>();
        let local_result_count = combined_results
            .iter()
            .take_while(|result| {
                local_result_ids.contains(&notion_page_identity_key(&result.block_id))
            })
            .count();
        let display_total = if prepared.request_had_local_exclusions {
            prepared.total
        } else {
            prepared
                .total
                .saturating_sub(local_server_overlap_count as u32)
        };
        QueryDisplayResults {
            query: prepared.query,
            preserve_scroll: prepared.preserve_scroll,
            display_total,
            has_more: prepared.consumed_result_count < prepared.total,
            local_result_count,
            results: combined_results.into(),
        }
    }

    fn show_query_response(&mut self, display: QueryDisplayResults) {
        self.reconcile_selected_result(&display.results);
        self.query_cache
            .retain(|entry| entry.query != display.query || entry.scope != self.scope);
        self.query_cache.push_front(NotionSearchQueryCacheEntry {
            query: display.query,
            scope: self.scope,
            total: display.display_total,
            has_more: display.has_more,
            local_result_count: display.local_result_count,
            results: display.results.clone(),
        });
        self.query_cache.truncate(NOTION_SEARCH_QUERY_CACHE_LIMIT);
        self.show_query_rows(display.results.len(), display.preserve_scroll);
        self.results = NotionSearchResultsState::Loaded {
            total: display.display_total,
            has_more: display.has_more,
            results: display.results,
        };
    }

    pub(crate) fn finish_query_failure(&mut self, token: NotionSearchRequestToken) {
        if !self.request_is_current(token) {
            return;
        }
        if let Some((pending_token, previous_limit)) = self.pagination_request {
            if pending_token == token {
                self.pagination_request = None;
                self.result_limit = previous_limit;
            }
        }
        if matches!(self.results, NotionSearchResultsState::Loaded { .. }) {
            return;
        }
        self.results = NotionSearchResultsState::Failed;
        self.clear_list();
    }
}

struct PreparedQueryResults {
    query: String,
    total: u32,
    consumed_result_count: u32,
    preserve_scroll: bool,
    query_result_details: HashMap<String, PageShellSearchResult>,
    preexisting_local_ids: HashSet<String>,
    request_had_local_exclusions: bool,
    query_was_indexed: bool,
    local_request: Option<NotionSearchLocalRequest>,
    server_results: Vec<PageShellSearchResult>,
    server_returned_ids: HashSet<String>,
}

struct QueryDisplayResults {
    query: String,
    preserve_scroll: bool,
    display_total: u32,
    has_more: bool,
    local_result_count: usize,
    results: Arc<[PageShellSearchResult]>,
}
