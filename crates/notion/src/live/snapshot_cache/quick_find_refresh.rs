use std::collections::{HashMap, HashSet};

use super::quick_find_store::{
    empty_quick_find_space, merge_quick_find_local_visits, take_quick_find_space,
};
use super::{quick_find_cache_lock, NotionWorkspaceSnapshotCacheStore};
use crate::model::{
    notion_page_identity_key, QuickFindLocalSearchCache, RecentPageResult, RecentPageVisit,
};

pub(crate) struct QuickFindRecentsRefreshUpdate<'a> {
    pub(crate) results: &'a [RecentPageResult],
    pub(crate) in_memory_concurrent_visits: &'a [RecentPageResult],
    pub(crate) refresh_authority_version: u64,
    pub(crate) refresh_started_at_unix_millis: u64,
    pub(crate) refresh_committed_at_unix_millis: u64,
}

impl NotionWorkspaceSnapshotCacheStore {
    pub(crate) fn persist_quick_find_recents_refresh(
        &self,
        space_id: &str,
        update: QuickFindRecentsRefreshUpdate<'_>,
    ) {
        let QuickFindRecentsRefreshUpdate {
            results,
            in_memory_concurrent_visits,
            refresh_authority_version,
            refresh_started_at_unix_millis,
            refresh_committed_at_unix_millis,
        } = update;
        let Ok(_guard) = quick_find_cache_lock().lock() else {
            println!("notnotion Quick Find cache lock is poisoned");
            return;
        };
        let path = self.quick_find_path();
        let mut cache = self.read_quick_find_cache_for_write(&path);
        let existing = take_quick_find_space(&mut cache, space_id);
        if existing
            .as_ref()
            .is_some_and(|space| space.authority_version > refresh_authority_version)
        {
            cache.spaces.insert(
                0,
                existing.expect("newer Quick Find cache space must exist"),
            );
            return;
        }
        let mut space = existing.unwrap_or_else(|| empty_quick_find_space(space_id));
        let (results, local_visits) = merge_refresh_visits(
            space.results,
            space.local_visits,
            results,
            in_memory_concurrent_visits,
            refresh_started_at_unix_millis,
        );
        let has_retained_query_authority = retain_query_authority(
            &mut space.local_search,
            &mut space.query_authority_version_by_key,
            &results,
            refresh_authority_version,
        );
        space.refreshed_at_unix_millis = refresh_started_at_unix_millis;
        space.local_search_updated_at_unix_millis = if has_retained_query_authority {
            space
                .local_search_updated_at_unix_millis
                .max(refresh_committed_at_unix_millis)
        } else {
            refresh_committed_at_unix_millis
        };
        space.authority_version = space.authority_version.max(refresh_authority_version);
        space.local_visits = local_visits;
        space.results = results;
        cache.spaces.insert(0, space);
        self.write_quick_find_cache(&path, &mut cache);
    }
}

fn merge_refresh_visits(
    existing_results: Vec<RecentPageResult>,
    existing_local_visits: Vec<RecentPageVisit>,
    refreshed: &[RecentPageResult],
    in_memory_concurrent_visits: &[RecentPageResult],
    refresh_started_at_unix_millis: u64,
) -> (Vec<RecentPageResult>, Vec<RecentPageVisit>) {
    let concurrent_local_visits = existing_local_visits
        .into_iter()
        .filter(|visit| visit.visited_at_unix_millis >= refresh_started_at_unix_millis)
        .collect::<Vec<_>>();
    let concurrent_page_ids = concurrent_local_visits
        .iter()
        .map(|visit| notion_page_identity_key(&visit.page_id))
        .collect::<HashSet<_>>();
    let on_disk_concurrent_visits = existing_results.into_iter().filter(|result| {
        concurrent_page_ids.contains(&notion_page_identity_key(&result.page.block_id))
    });
    let concurrent_visits = crate::model::LoadRecentPagesResult::merge_results(
        on_disk_concurrent_visits.collect(),
        in_memory_concurrent_visits.to_vec(),
    );
    let local_visits =
        merge_quick_find_local_visits(concurrent_local_visits, in_memory_concurrent_visits);
    let results =
        crate::model::LoadRecentPagesResult::merge_results(refreshed.to_vec(), concurrent_visits);
    (results, local_visits)
}

fn retain_query_authority(
    local_search: &mut QuickFindLocalSearchCache,
    query_authority_version_by_key: &mut HashMap<String, u64>,
    results: &[RecentPageResult],
    refresh_authority_version: u64,
) -> bool {
    let mut retained_indexed_queries = std::mem::take(&mut local_search.indexed_queries);
    retained_indexed_queries.retain(|indexed| {
        query_authority_version_by_key
            .get(&indexed.query)
            .is_some_and(|authority_version| *authority_version >= refresh_authority_version)
    });
    let retained_result_ids = retained_indexed_queries
        .iter()
        .flat_map(|indexed| indexed.result_ids.iter().cloned())
        .collect::<HashSet<_>>();
    let retained_query_pages = local_search
        .pages
        .iter()
        .filter(|page| retained_result_ids.contains(&notion_page_identity_key(&page.block_id)))
        .cloned()
        .collect::<Vec<_>>();
    local_search.invalidate_query_authority(results.iter().map(|result| result.page.clone()));
    local_search.merge_query_results(retained_query_pages);
    local_search.indexed_queries = retained_indexed_queries;
    query_authority_version_by_key.retain(|query_key, authority_version| {
        *authority_version >= refresh_authority_version
            && local_search
                .indexed_queries
                .iter()
                .any(|indexed| indexed.query == *query_key)
    });
    !query_authority_version_by_key.is_empty()
}
