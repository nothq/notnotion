use std::collections::HashMap;
#[cfg(test)]
use std::time::{Duration, Instant};

use chrono::Local;

use crate::model::{
    notion_page_identity_key, quick_find_local_query_key, PageShellSearchBadge,
    PageShellSearchResult,
};
use crate::ui::Arc;

use super::{
    list::recent_list_rows, NotionSearchRecentsCache, NotionSearchResultsState, NotionSearchState,
    NOTION_SEARCH_LOCAL_RESULT_LIMIT, NOTION_SEARCH_RECENCY_RANK_WEIGHT,
    NOTION_SEARCH_RECENTS_FRESH_FOR, NOTION_SEARCH_RECENT_PAGE_WEIGHT,
    NOTION_SEARCH_TITLE_MATCH_DENSITY_WEIGHT,
};
impl NotionSearchState {
    pub(super) fn show_cached_recents(&mut self) -> bool {
        let NotionSearchRecentsCache::Loaded { results, .. } = &self.recents_cache else {
            return false;
        };
        let list_rows = recent_list_rows(results, &Local::now());
        let results: Arc<[PageShellSearchResult]> = results
            .iter()
            .map(|result| {
                contextualize_search_result(
                    &result.page,
                    self.current_page_id.as_deref(),
                    self.current_page_url.as_deref(),
                )
            })
            .collect();
        self.reconcile_selected_result(&results);
        self.replace_list_rows(list_rows);
        self.results = NotionSearchResultsState::Loaded {
            total: results.len() as u32,
            has_more: false,
            results,
        };
        true
    }

    pub(super) fn recents_are_fresh(&self) -> bool {
        matches!(
            &self.recents_cache,
            NotionSearchRecentsCache::Loaded { refreshed_at, .. }
                if refreshed_at.is_some_and(|refreshed_at| {
                    refreshed_at.elapsed() < NOTION_SEARCH_RECENTS_FRESH_FOR
                })
        )
    }

    #[cfg(test)]
    pub(crate) fn expire_recents_freshness_for_test(&mut self) {
        if let NotionSearchRecentsCache::Loaded { refreshed_at, .. } = &mut self.recents_cache {
            *refreshed_at = Some(
                Instant::now()
                    .checked_sub(NOTION_SEARCH_RECENTS_FRESH_FOR + Duration::from_secs(1))
                    .expect("Quick Find test freshness interval must fit in Instant"),
            );
        }
    }

    pub(super) fn matching_local_pages(&self, query: &str) -> Vec<PageShellSearchResult> {
        self.matching_local_pages_up_to(query, NOTION_SEARCH_LOCAL_RESULT_LIMIT)
    }

    pub(super) fn matching_local_pages_up_to(
        &self,
        query: &str,
        limit: usize,
    ) -> Vec<PageShellSearchResult> {
        let query_key = quick_find_local_query_key(self.scope, query);
        let indexed_result_rank_by_id = self.indexed_result_ranks(&query_key);
        let recency_rank_by_id = self.recent_result_ranks();
        let candidates = self.local_search_candidates();
        let mut matches = scored_local_results(
            candidates,
            query,
            indexed_result_rank_by_id.as_ref(),
            &recency_rank_by_id,
        );
        matches.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.source_order.cmp(&right.source_order))
                .then_with(|| left.result.block_id.cmp(&right.result.block_id))
        });
        matches
            .into_iter()
            .take(limit)
            .map(|entry| {
                contextualize_search_result(
                    &entry.result,
                    self.current_page_id.as_deref(),
                    self.current_page_url.as_deref(),
                )
            })
            .collect()
    }

    fn indexed_result_ranks(&self, query_key: &str) -> Option<HashMap<String, usize>> {
        self.local_search_cache
            .indexed_result_ids(query_key)
            .map(|result_ids| {
                let mut rank_by_id = HashMap::with_capacity(result_ids.len());
                for (rank, result_id) in result_ids.iter().enumerate() {
                    rank_by_id
                        .entry(notion_page_identity_key(result_id))
                        .or_insert(rank);
                }
                rank_by_id
            })
    }

    fn recent_result_ranks(&self) -> HashMap<String, usize> {
        let mut recency_rank_by_id = HashMap::new();
        if let NotionSearchRecentsCache::Loaded { results, .. } = &self.recents_cache {
            for (rank, result) in results.iter().enumerate() {
                recency_rank_by_id
                    .entry(notion_page_identity_key(&result.page.block_id))
                    .or_insert(rank);
            }
        }
        recency_rank_by_id
    }

    fn local_search_candidates(&self) -> Vec<PageShellSearchResult> {
        let mut candidates = Vec::new();
        let mut candidate_index_by_id = HashMap::new();
        for page in &self.local_search_cache.pages {
            let identity = notion_page_identity_key(&page.block_id);
            if self.page_mutation_is_dirty(&identity) {
                continue;
            }
            candidate_index_by_id.insert(identity, candidates.len());
            candidates.push(page.clone());
        }
        if let NotionSearchRecentsCache::Loaded { results, .. } = &self.recents_cache {
            for result in results.iter() {
                let identity = notion_page_identity_key(&result.page.block_id);
                if self.page_mutation_is_dirty(&identity)
                    || candidate_index_by_id.contains_key(&identity)
                {
                    continue;
                }
                candidate_index_by_id.insert(identity, candidates.len());
                candidates.push(result.page.clone());
            }
        }
        candidates
    }
}

struct ScoredLocalResult {
    result: PageShellSearchResult,
    score: f64,
    source_order: usize,
}

fn scored_local_results(
    candidates: Vec<PageShellSearchResult>,
    query: &str,
    indexed_result_ranks: Option<&HashMap<String, usize>>,
    recent_result_ranks: &HashMap<String, usize>,
) -> Vec<ScoredLocalResult> {
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(source_order, result)| {
            let identity = notion_page_identity_key(&result.block_id);
            let source_order = match indexed_result_ranks {
                Some(rank_by_id) => rank_by_id.get(&identity).copied()?,
                None => source_order,
            };
            let recency_rank = recent_result_ranks.get(&identity).copied();
            local_search_score(&result, query, recency_rank).map(|score| ScoredLocalResult {
                result,
                score,
                source_order,
            })
        })
        .collect()
}

pub(super) fn normalized_notion_search_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn local_search_score(
    result: &PageShellSearchResult,
    query: &str,
    recency_rank: Option<usize>,
) -> Option<f64> {
    let title = result.title.to_lowercase();
    let title_length = title.chars().count().max(1) as f64;
    let query_length = query.chars().count() as f64;
    let match_density = query_length / title_length;
    if title == query {
        return Some(10_000.0 + match_density * NOTION_SEARCH_TITLE_MATCH_DENSITY_WEIGHT);
    }
    let lexical_score = if title.starts_with(query) {
        1_000.0 + match_density * NOTION_SEARCH_TITLE_MATCH_DENSITY_WEIGHT
    } else if title
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word.starts_with(query))
    {
        800.0 + match_density * NOTION_SEARCH_TITLE_MATCH_DENSITY_WEIGHT
    } else if title.contains(query) {
        600.0 + match_density * NOTION_SEARCH_TITLE_MATCH_DENSITY_WEIGHT
    } else {
        return None;
    };
    let recency_boost = recency_rank.map_or(0.0, |rank| {
        NOTION_SEARCH_RECENT_PAGE_WEIGHT + NOTION_SEARCH_RECENCY_RANK_WEIGHT / (rank + 1) as f64
    });
    Some(lexical_score + recency_boost)
}

pub(super) fn contextualize_search_result(
    result: &PageShellSearchResult,
    current_page_id: Option<&str>,
    current_page_url: Option<&str>,
) -> PageShellSearchResult {
    let mut result = result.clone();
    result
        .badges
        .retain(|badge| *badge != PageShellSearchBadge::CurrentPage);
    if current_page_id
        .is_some_and(|current_page_id| notion_page_ids_equal(current_page_id, &result.block_id))
    {
        result.badges.insert(0, PageShellSearchBadge::CurrentPage);
        if let Some(current_page_url) = current_page_url {
            result.target_board_url = current_page_url.to_string();
        }
    }
    result
}

fn notion_page_ids_equal(left: &str, right: &str) -> bool {
    notion_page_identity_key(left) == notion_page_identity_key(right)
}
