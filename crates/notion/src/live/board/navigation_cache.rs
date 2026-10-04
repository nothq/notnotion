use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use serde_json::Value;

use super::CompletePageResponse;
use crate::live::NotionLiveError;

const NAVIGATION_BOOTSTRAP_CACHE_CAPACITY: usize = 16;
const NAVIGATION_BOOTSTRAP_CACHE_TTL: Duration = Duration::from_secs(60);
const MAX_SPECULATIVE_REQUESTS: usize = 1;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct NavigationBootstrapKey {
    pub(super) active_user_id: String,
    pub(super) space_id: String,
    pub(super) page_block_id: String,
    pub(super) collection_view_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct NavigationBootstrap {
    pub(super) active_user_id: String,
    pub(super) space_id: String,
    pub(super) root_type: String,
    pub(super) response: NavigationBootstrapResponse,
}

impl NavigationBootstrap {
    pub(super) fn is_cacheable_for(&self, key: &NavigationBootstrapKey) -> bool {
        self.active_user_id == key.active_user_id
            && self.space_id == key.space_id
            && matches!(self.root_type.as_str(), "page" | "transcription")
            && self.response.is_complete()
    }
}

#[derive(Clone, Debug)]
pub(super) struct NavigationBootstrapResponse(NavigationBootstrapResponseKind);

#[derive(Clone, Debug)]
enum NavigationBootstrapResponseKind {
    Complete(Arc<CompletePageResponse>),
    InlineDatabase(Arc<Value>),
}

impl NavigationBootstrapResponse {
    pub(super) fn complete(response: CompletePageResponse) -> Self {
        Self(NavigationBootstrapResponseKind::Complete(Arc::new(
            response,
        )))
    }

    pub(super) fn inline_database(response: Value) -> Self {
        Self(NavigationBootstrapResponseKind::InlineDatabase(Arc::new(
            response,
        )))
    }

    pub(super) fn completed_page(&self) -> Result<&CompletePageResponse, String> {
        match &self.0 {
            NavigationBootstrapResponseKind::Complete(response) => Ok(response.as_ref()),
            NavigationBootstrapResponseKind::InlineDatabase(_) => {
                Err("inline Notion database bootstrap cannot load a page".to_string())
            }
        }
    }

    pub(super) fn database_value(&self) -> Value {
        match &self.0 {
            NavigationBootstrapResponseKind::Complete(response) => response.as_value().clone(),
            NavigationBootstrapResponseKind::InlineDatabase(response) => response.as_ref().clone(),
        }
    }

    pub(super) fn record_map(&self) -> Result<&serde_json::Map<String, Value>, String> {
        let response = match &self.0 {
            NavigationBootstrapResponseKind::Complete(response) => response.as_value(),
            NavigationBootstrapResponseKind::InlineDatabase(response) => response.as_ref(),
        };
        response
            .get("recordMap")
            .and_then(Value::as_object)
            .ok_or_else(|| "Notion navigation bootstrap is missing recordMap".to_string())
    }

    fn is_complete(&self) -> bool {
        matches!(&self.0, NavigationBootstrapResponseKind::Complete(_))
    }
}

pub(super) type NavigationBootstrapCell = OnceLock<Result<NavigationBootstrap, NotionLiveError>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NavigationBootstrapPriority {
    Demand,
    Prefetch,
}

#[derive(Debug)]
struct NavigationBootstrapEntry {
    cell: Arc<NavigationBootstrapCell>,
    last_used_at: Instant,
}

#[derive(Debug, Default)]
pub(super) struct NavigationBootstrapCache {
    entries: HashMap<NavigationBootstrapKey, NavigationBootstrapEntry>,
    recency: VecDeque<NavigationBootstrapKey>,
}

impl NavigationBootstrapCache {
    pub(super) fn acquire(
        &mut self,
        key: NavigationBootstrapKey,
        priority: NavigationBootstrapPriority,
    ) -> Option<Arc<NavigationBootstrapCell>> {
        self.remove_expired_entries();
        if let Some(cell) = self.entries.get_mut(&key).map(|entry| {
            entry.last_used_at = Instant::now();
            entry.cell.clone()
        }) {
            self.touch(&key);
            return Some(cell);
        }
        if priority == NavigationBootstrapPriority::Prefetch
            && self.in_flight_count() >= MAX_SPECULATIVE_REQUESTS
        {
            return None;
        }
        let cell = Arc::new(OnceLock::new());
        self.entries.insert(
            key.clone(),
            NavigationBootstrapEntry {
                cell: cell.clone(),
                last_used_at: Instant::now(),
            },
        );
        self.touch(&key);
        self.evict_ready_entries();
        Some(cell)
    }

    pub(super) fn seed(&mut self, key: NavigationBootstrapKey, bootstrap: NavigationBootstrap) {
        if !bootstrap.is_cacheable_for(&key) {
            return;
        }
        let cell = Arc::new(OnceLock::new());
        cell.set(Ok(bootstrap))
            .expect("new Notion navigation cache cell must be empty");
        self.entries.insert(
            key.clone(),
            NavigationBootstrapEntry {
                cell,
                last_used_at: Instant::now(),
            },
        );
        self.touch(&key);
        self.evict_ready_entries();
    }

    pub(super) fn discard_if_same(
        &mut self,
        key: &NavigationBootstrapKey,
        cell: &Arc<NavigationBootstrapCell>,
    ) {
        if self
            .entries
            .get(key)
            .is_some_and(|entry| Arc::ptr_eq(&entry.cell, cell))
        {
            self.entries.remove(key);
            self.recency.retain(|candidate| candidate != key);
        }
    }

    pub(super) fn invalidate_page(&mut self, page_block_id: &str) {
        self.entries
            .retain(|key, _| key.page_block_id != page_block_id);
        self.recency
            .retain(|key| key.page_block_id != page_block_id);
    }

    fn remove_expired_entries(&mut self) {
        let expired = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                entry.cell.get().is_some()
                    && entry.last_used_at.elapsed() >= NAVIGATION_BOOTSTRAP_CACHE_TTL
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in expired {
            self.entries.remove(&key);
            self.recency.retain(|candidate| candidate != &key);
        }
    }

    fn in_flight_count(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.cell.get().is_none())
            .count()
    }

    fn evict_ready_entries(&mut self) {
        let mut attempts = self.recency.len();
        while self.entries.len() > NAVIGATION_BOOTSTRAP_CACHE_CAPACITY && attempts > 0 {
            attempts -= 1;
            let Some(key) = self.recency.pop_front() else {
                break;
            };
            let ready = self
                .entries
                .get(&key)
                .is_some_and(|entry| entry.cell.get().is_some());
            if ready {
                self.entries.remove(&key);
            } else {
                self.recency.push_back(key);
            }
        }
    }

    fn touch(&mut self, key: &NavigationBootstrapKey) {
        self.recency.retain(|candidate| candidate != key);
        self.recency.push_back(key.clone());
    }
}
