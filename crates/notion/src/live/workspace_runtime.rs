use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{mpsc, Arc, Mutex},
};

mod api;
mod operations;
mod quick_find;
mod quick_find_state;
mod recovery;
mod runtime;
#[cfg(test)]
mod tests;

use crate::live::LiveBoardMutator;
use crate::model::{
    CardPageCodeSettingsCapability, LoadRecentPagesResult, NotionBootstrapApi,
    PageShellSearchResult, QuickFindLocalSearchCache, RecentPageResult,
};

use super::board::{LiveCollectionQueryState, LiveDatabaseQueryState, LiveWorkspaceContext};
use super::snapshot_cache::{
    NotionWorkspaceSnapshotCacheStore, QuickFindQueryCacheUpdate, QuickFindRecentsRefreshUpdate,
};

pub struct NotionWorkspaceRuntime {
    active_user_id: String,
    rebootstrap_api: Arc<dyn NotionBootstrapApi>,
    mutator: Box<Mutex<LiveBoardMutator>>,
    workspace_context: LiveWorkspaceContext,
    collection_query_state: Option<Mutex<LiveCollectionQueryState>>,
    database_query_state: Option<LiveDatabaseQueryState>,
    snapshot_cache: Option<NotionWorkspaceSnapshotCacheStore>,
    quick_find_runtime_generation: Arc<Mutex<u64>>,
    runtime_generation: u64,
    quick_find_query_cache_writer: Option<QuickFindQueryCacheWriter>,
    quick_find_recents_cache_writer: Option<QuickFindRecentsCacheWriter>,
    quick_find_recents_commit: Mutex<()>,
    quick_find_recents: Mutex<QuickFindRecentsState>,
    quick_find_local_search: Mutex<QuickFindLocalSearchState>,
    code_settings: CardPageCodeSettingsCapability,
}

const QUICK_FIND_PENDING_QUERY_LIMIT: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct QuickFindQueryIdentity {
    session_id: String,
    flow_number: u32,
}

#[derive(Clone, Copy)]
struct QuickFindQueryAuthority {
    version: u64,
    started_at_unix_millis: u64,
}

struct QuickFindLocalSearchState {
    cache: QuickFindLocalSearchCache,
    authority_version: u64,
    pending_queries: HashMap<QuickFindQueryIdentity, QuickFindQueryAuthority>,
    pending_query_order: VecDeque<QuickFindQueryIdentity>,
}

#[derive(Clone)]
pub(super) struct QuickFindQueryCacheWrite {
    pub(super) query_key: String,
    pub(super) results: Vec<PageShellSearchResult>,
    pub(super) response_is_complete: bool,
    pub(super) authority_version: u64,
    pub(super) accepted_at_unix_millis: u64,
}

#[derive(Clone)]
pub(super) struct QuickFindPageMutationCacheWrite {
    pub(super) page_ids: Vec<String>,
    pub(super) authority_version: u64,
    pub(super) invalidated_at_unix_millis: u64,
}

enum QuickFindQueryCacheWriterEvent {
    Query(QuickFindQueryCacheWrite),
    PageMutation(QuickFindPageMutationCacheWrite),
}

pub(super) struct QuickFindQueryCacheWriter {
    pending: Arc<Mutex<VecDeque<QuickFindQueryCacheWriterEvent>>>,
    sender: mpsc::SyncSender<()>,
}

#[derive(Clone)]
pub(super) struct QuickFindRecentsCacheWrite {
    pub(super) results: Vec<RecentPageResult>,
    pub(super) concurrent_visits: Vec<RecentPageResult>,
    pub(super) authority_version: u64,
    pub(super) refresh_started_at_unix_millis: u64,
    pub(super) refresh_committed_at_unix_millis: u64,
}

pub(super) struct QuickFindRecentsCacheWriter {
    pending: Arc<Mutex<VecDeque<QuickFindRecentsCacheWrite>>>,
    sender: mpsc::SyncSender<()>,
}

struct QuickFindRecentsState {
    results: Option<LoadRecentPagesResult>,
    next_refresh_token: u64,
    last_committed_refresh_token: u64,
    dirty_page_ids_by_refresh: HashMap<u64, HashSet<String>>,
}

struct QuickFindRecentsRefresh {
    token: u64,
    started_at_unix_millis: u64,
    runtime_generation: u64,
}

pub(crate) struct NotionWorkspaceRuntimeInput {
    pub(crate) active_user_id: String,
    pub(crate) rebootstrap_api: Arc<dyn NotionBootstrapApi>,
    pub(crate) mutator: LiveBoardMutator,
    pub(crate) workspace_context: LiveWorkspaceContext,
    pub(crate) collection_query_state: Option<LiveCollectionQueryState>,
    pub(crate) database_query_state: Option<LiveDatabaseQueryState>,
    pub(crate) snapshot_cache: Option<NotionWorkspaceSnapshotCacheStore>,
    pub(crate) quick_find_runtime_generation: Option<Arc<Mutex<u64>>>,
    pub(crate) code_settings: CardPageCodeSettingsCapability,
}

impl QuickFindQueryCacheWriter {
    pub(super) fn start(
        store: NotionWorkspaceSnapshotCacheStore,
        space_id: String,
    ) -> Result<Self, String> {
        let pending = Arc::new(Mutex::new(VecDeque::new()));
        let writer_pending = pending.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("notion-quick-find-query-cache".to_string())
            .spawn(move || {
                persist_pending_quick_find_query_cache(receiver, writer_pending, store, space_id);
            })
            .map_err(|error| format!("failed to start its background query writer: {error}"))?;
        Ok(Self { pending, sender })
    }

    pub(super) fn stage(&self, write: QuickFindQueryCacheWrite) {
        self.stage_event(QuickFindQueryCacheWriterEvent::Query(write));
    }

    pub(super) fn stage_page_mutation(&self, write: QuickFindPageMutationCacheWrite) {
        self.stage_event(QuickFindQueryCacheWriterEvent::PageMutation(write));
    }

    fn stage_event(&self, event: QuickFindQueryCacheWriterEvent) {
        self.pending
            .lock()
            .expect("Notion Quick Find query-cache writer state must not be poisoned")
            .push_back(event);
        match self.sender.try_send(()) {
            Ok(()) | Err(mpsc::TrySendError::Full(())) => {}
            Err(mpsc::TrySendError::Disconnected(())) => {
                println!("notnotion Quick Find query-cache writer stopped unexpectedly");
            }
        }
    }
}

fn persist_pending_quick_find_query_cache(
    receiver: mpsc::Receiver<()>,
    pending: Arc<Mutex<VecDeque<QuickFindQueryCacheWriterEvent>>>,
    store: NotionWorkspaceSnapshotCacheStore,
    space_id: String,
) {
    while receiver.recv().is_ok() {
        while receiver.try_recv().is_ok() {}
        let writes = pending
            .lock()
            .expect("Notion Quick Find query-cache writer state must not be poisoned")
            .drain(..)
            .collect::<Vec<_>>();
        for event in writes {
            match event {
                QuickFindQueryCacheWriterEvent::Query(write) => {
                    store.persist_quick_find_query_cache(
                        &space_id,
                        QuickFindQueryCacheUpdate {
                            query_key: &write.query_key,
                            results: &write.results,
                            response_is_complete: write.response_is_complete,
                            query_authority_version: write.authority_version,
                            accepted_at_unix_millis: write.accepted_at_unix_millis,
                        },
                    );
                }
                QuickFindQueryCacheWriterEvent::PageMutation(write) => {
                    store.persist_quick_find_page_mutation(
                        &space_id,
                        &write.page_ids,
                        write.authority_version,
                        write.invalidated_at_unix_millis,
                    );
                }
            }
        }
    }
}

impl QuickFindRecentsCacheWriter {
    pub(super) fn start(
        store: NotionWorkspaceSnapshotCacheStore,
        space_id: String,
    ) -> Result<Self, String> {
        let pending = Arc::new(Mutex::new(VecDeque::new()));
        let writer_pending = pending.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("notion-quick-find-recents-cache".to_string())
            .spawn(move || {
                persist_pending_quick_find_recents_cache(receiver, writer_pending, store, space_id);
            })
            .map_err(|error| format!("failed to start its background recents writer: {error}"))?;
        Ok(Self { pending, sender })
    }

    pub(super) fn stage(&self, write: QuickFindRecentsCacheWrite) {
        self.pending
            .lock()
            .expect("Notion Quick Find recents-cache writer state must not be poisoned")
            .push_back(write);
        match self.sender.try_send(()) {
            Ok(()) | Err(mpsc::TrySendError::Full(())) => {}
            Err(mpsc::TrySendError::Disconnected(())) => {
                println!("notnotion Quick Find recents-cache writer stopped unexpectedly");
            }
        }
    }
}

fn persist_pending_quick_find_recents_cache(
    receiver: mpsc::Receiver<()>,
    pending: Arc<Mutex<VecDeque<QuickFindRecentsCacheWrite>>>,
    store: NotionWorkspaceSnapshotCacheStore,
    space_id: String,
) {
    while receiver.recv().is_ok() {
        while receiver.try_recv().is_ok() {}
        let writes = pending
            .lock()
            .expect("Notion Quick Find recents-cache writer state must not be poisoned")
            .drain(..)
            .collect::<Vec<_>>();
        for write in writes {
            store.persist_quick_find_recents_refresh(
                &space_id,
                QuickFindRecentsRefreshUpdate {
                    results: &write.results,
                    in_memory_concurrent_visits: &write.concurrent_visits,
                    refresh_authority_version: write.authority_version,
                    refresh_started_at_unix_millis: write.refresh_started_at_unix_millis,
                    refresh_committed_at_unix_millis: write.refresh_committed_at_unix_millis,
                },
            );
        }
    }
}
