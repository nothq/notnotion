use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use super::quick_find::{
    commit_quick_find_recents_refresh_state, load_cached_quick_find_local_search,
    load_cached_quick_find_recents, quick_find_response_is_complete,
    quick_find_unix_timestamp_millis, QuickFindRecentsCommitLocks,
};
use super::{
    NotionWorkspaceRuntime, NotionWorkspaceRuntimeInput, QuickFindLocalSearchState,
    QuickFindPageMutationCacheWrite, QuickFindQueryCacheWriter, QuickFindRecentsCacheWriter,
    QuickFindRecentsRefresh, QuickFindRecentsState,
};
use crate::live::snapshot_cache::NotionWorkspaceSnapshotCacheStore;
use crate::model::{
    notion_page_identity_key, LoadRecentPagesResult, QuickFindLocalSearchCache, RecentPageResult,
    SearchWorkspaceRequest, SearchWorkspaceResult,
};

fn next_runtime_generation(generation: &Arc<Mutex<u64>>) -> u64 {
    let mut generation = generation
        .lock()
        .expect("Notion Quick Find runtime generation lock must not be poisoned");
    *generation = generation
        .checked_add(1)
        .expect("Notion Quick Find runtime generation overflowed");
    *generation
}

fn start_query_cache_writer(
    snapshot_cache: Option<&NotionWorkspaceSnapshotCacheStore>,
    space_id: &str,
) -> Option<QuickFindQueryCacheWriter> {
    let snapshot_cache = snapshot_cache?;
    match QuickFindQueryCacheWriter::start(snapshot_cache.clone(), space_id.to_string()) {
        Ok(writer) => Some(writer),
        Err(error) => {
            println!("notnotion Quick Find query cache is memory-only: {error}");
            None
        }
    }
}

fn start_recents_cache_writer(
    snapshot_cache: Option<&NotionWorkspaceSnapshotCacheStore>,
    space_id: &str,
) -> Option<QuickFindRecentsCacheWriter> {
    let snapshot_cache = snapshot_cache?;
    match QuickFindRecentsCacheWriter::start(snapshot_cache.clone(), space_id.to_string()) {
        Ok(writer) => Some(writer),
        Err(error) => {
            println!("notnotion Quick Find recents cache is memory-only: {error}");
            None
        }
    }
}

impl NotionWorkspaceRuntime {
    pub(crate) fn new(input: NotionWorkspaceRuntimeInput) -> Self {
        let NotionWorkspaceRuntimeInput {
            active_user_id,
            rebootstrap_api,
            mutator,
            workspace_context,
            collection_query_state,
            database_query_state,
            snapshot_cache,
            quick_find_runtime_generation,
            code_settings,
        } = input;
        let quick_find_space_id = workspace_context
            .cached_space_id()
            .expect("live Notion Quick Find requires a stable workspace space ID");
        let quick_find_runtime_generation =
            quick_find_runtime_generation.unwrap_or_else(|| Arc::new(Mutex::new(0)));
        let runtime_generation = next_runtime_generation(&quick_find_runtime_generation);
        let quick_find_recents = QuickFindRecentsState {
            results: load_cached_quick_find_recents(snapshot_cache.as_ref(), &workspace_context),
            next_refresh_token: 1,
            last_committed_refresh_token: 0,
            dirty_page_ids_by_refresh: HashMap::new(),
        };
        let (quick_find_local_search, persisted_quick_find_authority_version) =
            load_cached_quick_find_local_search(snapshot_cache.as_ref(), &workspace_context);
        let quick_find_query_cache_writer =
            start_query_cache_writer(snapshot_cache.as_ref(), &quick_find_space_id);
        let quick_find_recents_cache_writer =
            start_recents_cache_writer(snapshot_cache.as_ref(), &quick_find_space_id);
        Self {
            active_user_id,
            rebootstrap_api,
            mutator: Box::new(Mutex::new(mutator)),
            workspace_context,
            collection_query_state: collection_query_state.map(Mutex::new),
            database_query_state,
            snapshot_cache,
            quick_find_runtime_generation,
            runtime_generation,
            quick_find_query_cache_writer,
            quick_find_recents_cache_writer,
            quick_find_recents_commit: Mutex::new(()),
            quick_find_recents: Mutex::new(quick_find_recents),
            quick_find_local_search: Mutex::new(QuickFindLocalSearchState::new_after(
                quick_find_local_search,
                persisted_quick_find_authority_version,
            )),
            code_settings,
        }
    }

    pub(super) fn cached_quick_find_recents(&self) -> Option<LoadRecentPagesResult> {
        match self.quick_find_recents.lock() {
            Ok(recents) => recents.results.clone(),
            Err(_) => {
                println!("notnotion Quick Find recents lock is poisoned");
                None
            }
        }
    }

    pub(super) fn record_quick_find_recents(&self, incoming: Vec<RecentPageResult>) {
        if incoming.is_empty() {
            return;
        }
        match self.quick_find_recents.lock() {
            Ok(mut cached) => {
                let added_page_ids = incoming
                    .iter()
                    .map(|result| notion_page_identity_key(&result.page.block_id))
                    .collect::<Vec<_>>();
                for dirty_page_ids in cached.dirty_page_ids_by_refresh.values_mut() {
                    dirty_page_ids.extend(added_page_ids.iter().cloned());
                }
                let merged = LoadRecentPagesResult::merge_results(
                    cached
                        .results
                        .as_ref()
                        .map_or_else(Vec::new, |cached| cached.results.clone()),
                    incoming.clone(),
                );
                cached.results = Some(LoadRecentPagesResult { results: merged });
            }
            Err(_) => {
                println!("notnotion Quick Find recents lock is poisoned");
                return;
            }
        }
        let Some(snapshot_cache) = self.snapshot_cache.as_ref() else {
            return;
        };
        let space_id = match self.workspace_context.cached_space_id() {
            Ok(space_id) => space_id,
            Err(error) => {
                println!("notnotion Quick Find could not persist its recent page: {error}");
                return;
            }
        };
        snapshot_cache.persist_quick_find_recents(&space_id, &incoming);
    }

    pub(super) fn quick_find_local_search_snapshot(&self) -> QuickFindLocalSearchCache {
        match self.quick_find_local_search.lock() {
            Ok(local_search) => local_search.cache.clone(),
            Err(_) => {
                println!("notnotion Quick Find local-search cache lock is poisoned");
                QuickFindLocalSearchCache::default()
            }
        }
    }

    pub(super) fn begin_quick_find_query(&self, request: &SearchWorkspaceRequest) {
        match self.quick_find_local_search.lock() {
            Ok(mut local_search) => local_search.begin_query(request),
            Err(_) => println!("notnotion Quick Find local-search cache lock is poisoned"),
        }
    }

    pub(super) fn cancel_quick_find_query(&self, request: &SearchWorkspaceRequest) {
        match self.quick_find_local_search.lock() {
            Ok(mut local_search) => local_search.cancel_query(request),
            Err(_) => println!("notnotion Quick Find local-search cache lock is poisoned"),
        }
    }

    pub(super) fn accept_quick_find_query_response(
        &self,
        request: &SearchWorkspaceRequest,
        result: &SearchWorkspaceResult,
    ) {
        let cache_write = match self.quick_find_local_search.lock() {
            Ok(mut local_search) => local_search.accept_query_response(
                request,
                result.results.clone(),
                quick_find_response_is_complete(request, result),
                quick_find_unix_timestamp_millis(),
            ),
            Err(_) => {
                println!("notnotion Quick Find local-search cache lock is poisoned");
                None
            }
        };
        if let (Some(writer), Some(cache_write)) =
            (self.quick_find_query_cache_writer.as_ref(), cache_write)
        {
            writer.stage(cache_write);
        }
    }

    pub(super) fn invalidate_quick_find_page_mutation_cache(&self, page_id: &str, persist: bool) {
        if let Err(error) = self
            .workspace_context
            .invalidate_quick_find_record_root(page_id)
        {
            println!("notnotion Quick Find record cache invalidation failed: {error}");
        }
        let identity = notion_page_identity_key(page_id);
        let _commit_guard = match self.quick_find_recents_commit.lock() {
            Ok(guard) => guard,
            Err(_) => {
                println!("notnotion Quick Find recents commit lock is poisoned");
                return;
            }
        };
        match self.quick_find_recents.lock() {
            Ok(mut recents) => {
                recents.dirty_page_ids_by_refresh.clear();
                if let Some(cached) = &mut recents.results {
                    cached.results.retain(|result| {
                        notion_page_identity_key(&result.page.block_id) != identity
                    });
                }
            }
            Err(_) => {
                println!("notnotion Quick Find recents lock is poisoned");
                return;
            }
        }
        let authority_version = match self.quick_find_local_search.lock() {
            Ok(mut local_search) => local_search.invalidate_page_mutation(&identity),
            Err(_) => {
                println!("notnotion Quick Find local-search cache lock is poisoned");
                return;
            }
        };
        if !persist {
            return;
        }
        if let Some(writer) = self.quick_find_query_cache_writer.as_ref() {
            writer.stage_page_mutation(QuickFindPageMutationCacheWrite {
                page_ids: vec![identity],
                authority_version,
                invalidated_at_unix_millis: quick_find_unix_timestamp_millis(),
            });
        }
    }

    pub(super) fn begin_quick_find_recents_refresh(&self) -> QuickFindRecentsRefresh {
        let started_at_unix_millis = quick_find_unix_timestamp_millis();
        let mut cached = self
            .quick_find_recents
            .lock()
            .expect("Notion Quick Find recents lock must not be poisoned");
        let token = cached.next_refresh_token;
        cached.next_refresh_token = token
            .checked_add(1)
            .expect("Notion Quick Find refresh token overflowed");
        cached
            .dirty_page_ids_by_refresh
            .insert(token, HashSet::new());
        QuickFindRecentsRefresh {
            token,
            started_at_unix_millis,
            runtime_generation: self.runtime_generation,
        }
    }

    pub(super) fn cancel_quick_find_recents_refresh(&self, token: u64) {
        match self.quick_find_recents.lock() {
            Ok(mut cached) => {
                cached.dirty_page_ids_by_refresh.remove(&token);
            }
            Err(_) => println!("notnotion Quick Find recents lock is poisoned"),
        }
    }

    pub(super) fn commit_quick_find_recents_refresh(
        &self,
        incoming: Vec<RecentPageResult>,
        refresh: QuickFindRecentsRefresh,
    ) {
        let cache_write = commit_quick_find_recents_refresh_state(
            QuickFindRecentsCommitLocks {
                commit_lock: &self.quick_find_recents_commit,
                quick_find_runtime_generation: &self.quick_find_runtime_generation,
                quick_find_recents: &self.quick_find_recents,
                quick_find_local_search: &self.quick_find_local_search,
            },
            incoming,
            refresh,
        );
        if let (Some(writer), Some(cache_write)) =
            (self.quick_find_recents_cache_writer.as_ref(), cache_write)
        {
            writer.stage(cache_write);
        }
    }
}
