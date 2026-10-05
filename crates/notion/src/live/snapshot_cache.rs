use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use serde::{Deserialize, Serialize};

use crate::model::{
    BoardSnapshot, NotionCacheFailure, NotionLaunchRoute, NotionRouteSource,
    QuickFindLocalSearchCache, RecentPageResult, RecentPageVisit,
};

use super::credentials::{stored_notion_desktop_session, NotionDesktopSession};

mod quick_find_load;
mod quick_find_mutation;
mod quick_find_query;
mod quick_find_refresh;
mod quick_find_store;
#[cfg(test)]
mod tests;
mod workspace;

pub(crate) use quick_find_query::QuickFindQueryCacheUpdate;
pub(crate) use quick_find_refresh::QuickFindRecentsRefreshUpdate;

const NOTION_CACHE_DIR_ENV: &str = "NOTNOTION_CACHE_DIR";
const NOTION_WORKSPACE_CACHE_KEY_NAMESPACE: &str = "notion|workspace-snapshot|v6";
const NOTION_WORKSPACE_CACHE_SCHEMA_VERSION: u32 = 6;
const NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION: u32 = 6;
const NOTION_QUICK_FIND_CACHED_SPACE_LIMIT: usize = 4;
const NOTION_QUICK_FIND_CACHE_MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct NotionWorkspaceSnapshotCache {
    schema_version: u32,
    fetched_at_unix_secs: u64,
    active_user_id: String,
    canonical_board_url: String,
    route: NotionLaunchRoute,
    workspace: BoardSnapshot,
}

struct ValidatedNotionWorkspaceSnapshot {
    route: NotionLaunchRoute,
    workspace: BoardSnapshot,
}

#[derive(Serialize, Deserialize)]
struct NotionQuickFindCache {
    schema_version: u32,
    active_user_id: String,
    spaces: Vec<NotionQuickFindSpaceCache>,
}

#[derive(Serialize, Deserialize)]
struct NotionQuickFindSpaceCache {
    space_id: String,
    refreshed_at_unix_millis: u64,
    #[serde(default)]
    local_search_updated_at_unix_millis: u64,
    #[serde(default)]
    authority_version: u64,
    #[serde(default)]
    query_authority_version_by_key: HashMap<String, u64>,
    local_visits: Vec<RecentPageVisit>,
    results: Vec<RecentPageResult>,
    #[serde(default)]
    local_search: QuickFindLocalSearchCache,
}

#[derive(Clone)]
pub(crate) struct NotionWorkspaceSnapshotCacheStore {
    path: PathBuf,
    key: [u8; 32],
    active_user_id: String,
}

pub(crate) struct NotionQuickFindLocalSearchSnapshot {
    pub(crate) local_search: QuickFindLocalSearchCache,
    pub(crate) authority_version: u64,
}

pub(crate) fn load_cached_notion_route(
    session: &NotionDesktopSession,
    route_source: &NotionRouteSource,
) -> Result<Option<NotionLaunchRoute>, NotionCacheFailure> {
    match NotionWorkspaceSnapshotCacheStore::from_session(session) {
        Ok(store) => store.load_route(route_source),
        Err(error) => Err(unreadable_cache_failure(error)),
    }
}

fn invalid_cache_failure(diagnostic: impl Into<String>) -> NotionCacheFailure {
    NotionCacheFailure::Invalid {
        diagnostic: diagnostic.into(),
    }
}

fn unreadable_cache_failure(diagnostic: impl Into<String>) -> NotionCacheFailure {
    NotionCacheFailure::Unreadable {
        diagnostic: diagnostic.into(),
    }
}

pub(crate) fn stored_notion_workspace_cache_store(
) -> Result<Option<NotionWorkspaceSnapshotCacheStore>, String> {
    stored_notion_desktop_session()
        .map_err(|error| error.to_string())?
        .map(|session| NotionWorkspaceSnapshotCacheStore::from_session(&session))
        .transpose()
}

fn workspace_cache_path(cache_root: &Path, cache_key: &str) -> PathBuf {
    local_cache::account_cache_dir(cache_root, cache_key).join("workspace.bin")
}

fn cache_key_lock() -> &'static Mutex<()> {
    static CACHE_KEY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    CACHE_KEY_LOCK.get_or_init(|| Mutex::new(()))
}

fn quick_find_cache_lock() -> &'static Mutex<()> {
    static QUICK_FIND_CACHE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    QUICK_FIND_CACHE_LOCK.get_or_init(|| Mutex::new(()))
}
