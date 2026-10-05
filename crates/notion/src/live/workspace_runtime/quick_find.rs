use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    NotionWorkspaceSnapshotCacheStore, QuickFindLocalSearchState, QuickFindRecentsCacheWrite,
    QuickFindRecentsRefresh, QuickFindRecentsState,
};
use crate::live::board::LiveWorkspaceContext;
use crate::model::{
    notion_page_identity_key, LoadRecentPagesResult, QuickFindLocalSearchCache, RecentPageResult,
    SearchWorkspaceRequest, SearchWorkspaceResult,
};

/// Refreshed recents and the concurrent local visits they preserved.
type RefreshedRecents = (Vec<RecentPageResult>, Vec<RecentPageResult>);

pub(super) fn merge_authoritative_quick_find_refresh(
    cached: Vec<RecentPageResult>,
    incoming: Vec<RecentPageResult>,
    dirty_page_ids: &HashSet<String>,
) -> RefreshedRecents {
    let concurrent_visits = cached
        .into_iter()
        .filter(|result| dirty_page_ids.contains(&notion_page_identity_key(&result.page.block_id)))
        .collect::<Vec<_>>();
    let refreshed = LoadRecentPagesResult::merge_results(incoming, concurrent_visits.clone());
    (refreshed, concurrent_visits)
}

pub(super) struct QuickFindRecentsCommitLocks<'a> {
    pub(super) commit_lock: &'a Mutex<()>,
    pub(super) quick_find_runtime_generation: &'a Mutex<u64>,
    pub(super) quick_find_recents: &'a Mutex<QuickFindRecentsState>,
    pub(super) quick_find_local_search: &'a Mutex<QuickFindLocalSearchState>,
}

pub(super) fn commit_quick_find_recents_refresh_state(
    locks: QuickFindRecentsCommitLocks<'_>,
    incoming: Vec<RecentPageResult>,
    refresh: QuickFindRecentsRefresh,
) -> Option<QuickFindRecentsCacheWrite> {
    let QuickFindRecentsCommitLocks {
        commit_lock,
        quick_find_runtime_generation,
        quick_find_recents,
        quick_find_local_search,
    } = locks;
    let runtime_generation = lock_runtime_generation(quick_find_runtime_generation)?;
    if *runtime_generation != refresh.runtime_generation {
        if let Ok(mut cached) = quick_find_recents.lock() {
            cached.dirty_page_ids_by_refresh.remove(&refresh.token);
        }
        return None;
    }
    let _commit_guard = lock_recents_commit(commit_lock)?;
    let (refreshed, concurrent_visits) =
        commit_recents_results(quick_find_recents, incoming, refresh.token)?;
    let authority_version = reset_local_search_authority(quick_find_local_search, &refreshed)?;
    Some(QuickFindRecentsCacheWrite {
        results: refreshed,
        concurrent_visits,
        authority_version,
        refresh_started_at_unix_millis: refresh.started_at_unix_millis,
        refresh_committed_at_unix_millis: quick_find_unix_timestamp_millis()
            .max(refresh.started_at_unix_millis),
    })
}

fn lock_runtime_generation(generation: &Mutex<u64>) -> Option<std::sync::MutexGuard<'_, u64>> {
    match generation.lock() {
        Ok(generation) => Some(generation),
        Err(_) => {
            println!("notnotion Quick Find runtime generation lock is poisoned");
            None
        }
    }
}

fn lock_recents_commit(commit_lock: &Mutex<()>) -> Option<std::sync::MutexGuard<'_, ()>> {
    match commit_lock.lock() {
        Ok(guard) => Some(guard),
        Err(_) => {
            println!("notnotion Quick Find recents commit lock is poisoned");
            None
        }
    }
}

fn commit_recents_results(
    quick_find_recents: &Mutex<QuickFindRecentsState>,
    incoming: Vec<RecentPageResult>,
    refresh_token: u64,
) -> Option<RefreshedRecents> {
    match quick_find_recents.lock() {
        Ok(mut cached) => {
            let dirty_page_ids = cached.dirty_page_ids_by_refresh.remove(&refresh_token)?;
            if cached.last_committed_refresh_token > refresh_token {
                return None;
            }
            let (refreshed, concurrent_visits) = merge_authoritative_quick_find_refresh(
                cached
                    .results
                    .take()
                    .map_or_else(Vec::new, |cached| cached.results),
                incoming,
                &dirty_page_ids,
            );
            cached.results = Some(LoadRecentPagesResult {
                results: refreshed.clone(),
            });
            cached.last_committed_refresh_token = refresh_token;
            Some((refreshed, concurrent_visits))
        }
        Err(_) => {
            println!("notnotion Quick Find recents lock is poisoned");
            None
        }
    }
}

fn reset_local_search_authority(
    quick_find_local_search: &Mutex<QuickFindLocalSearchState>,
    refreshed: &[RecentPageResult],
) -> Option<u64> {
    let mut local_search = match quick_find_local_search.lock() {
        Ok(local_search) => local_search,
        Err(_) => {
            println!("notnotion Quick Find local-search cache lock is poisoned");
            return None;
        }
    };
    local_search.reset_authority(refreshed.iter().map(|result| result.page.clone()));
    Some(local_search.authority_version)
}

pub(super) fn quick_find_unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch for Notion Quick Find")
        .as_millis()
        .try_into()
        .expect("Notion Quick Find timestamp must fit in a u64")
}

pub(super) fn next_quick_find_authority_version_after(minimum: u64) -> u64 {
    static LAST_AUTHORITY_VERSION: AtomicU64 = AtomicU64::new(0);
    let wall_clock_candidate: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must follow the Unix epoch for Notion Quick Find")
        .as_nanos()
        .try_into()
        .expect("Notion Quick Find nanosecond timestamp must fit in a u64");
    let mut observed = LAST_AUTHORITY_VERSION.load(Ordering::Relaxed);
    loop {
        let next = wall_clock_candidate.max(
            observed
                .max(minimum)
                .checked_add(1)
                .expect("Notion Quick Find authority version overflowed"),
        );
        match LAST_AUTHORITY_VERSION.compare_exchange_weak(
            observed,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return next,
            Err(current) => observed = current,
        }
    }
}

pub(super) fn quick_find_response_is_complete(
    request: &SearchWorkspaceRequest,
    result: &SearchWorkspaceResult,
) -> bool {
    request.excluded_block_ids.is_empty() && result.consumed_result_count >= result.total
}

pub(super) fn load_cached_quick_find_recents(
    snapshot_cache: Option<&NotionWorkspaceSnapshotCacheStore>,
    workspace_context: &LiveWorkspaceContext,
) -> Option<LoadRecentPagesResult> {
    let snapshot_cache = snapshot_cache?;
    let space_id = match workspace_context.cached_space_id() {
        Ok(space_id) => space_id,
        Err(error) => {
            println!("notnotion Quick Find could not identify its cached space: {error}");
            return None;
        }
    };
    match snapshot_cache.load_quick_find_recents(&space_id) {
        Ok(Some(results)) => Some(LoadRecentPagesResult { results }),
        Ok(None) => None,
        Err(error) => {
            println!("notnotion Quick Find cache is unavailable: {error}");
            None
        }
    }
}

pub(super) fn load_cached_quick_find_local_search(
    snapshot_cache: Option<&NotionWorkspaceSnapshotCacheStore>,
    workspace_context: &LiveWorkspaceContext,
) -> (QuickFindLocalSearchCache, u64) {
    let (mut local_search, authority_version) = snapshot_cache
        .and_then(|snapshot_cache| {
            let space_id = workspace_context.cached_space_id().ok()?;
            match snapshot_cache.load_quick_find_local_search_with_authority(&space_id) {
                Ok(local_search) => local_search,
                Err(error) => {
                    println!("notnotion Quick Find local index is unavailable: {error}");
                    None
                }
            }
        })
        .map_or_else(
            || (QuickFindLocalSearchCache::default(), 0),
            |loaded| (loaded.local_search, loaded.authority_version),
        );
    match workspace_context.cached_local_search_pages() {
        Ok(pages) => local_search.add_provisional_pages(pages),
        Err(error) => println!("notnotion Quick Find sidebar index is unavailable: {error}"),
    }
    (local_search, authority_version)
}
