use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::model::{
    PageShellSearchResult, QuickFindLocalSearchCache, RecentPageResult, SearchWorkspaceScope,
};
use crate::ui::{Arc, LoadedCardPage};
use gpui::{px, Entity, ListAlignment, ListState, ScrollHandle, SharedString};
use gpui_components::text_input::TextInput;

mod controller;
mod database;
mod list;
mod local;
mod mutation;
mod preview;
mod preview_cache;
mod query;
mod recents;
mod recents_controller;
mod session;

#[cfg(test)]
mod tests;

pub(crate) use database::DatabaseSearchState;
#[derive(Clone, Default)]
pub(crate) enum NotionSearchResultsState {
    #[default]
    Idle,
    Loading,
    Loaded {
        total: u32,
        has_more: bool,
        results: Arc<[PageShellSearchResult]>,
    },
    Failed,
}

#[derive(Clone, Default)]
pub(crate) enum NotionSearchPreviewState {
    #[default]
    Idle,
    Loading {
        block_id: SharedString,
        request_token: NotionSearchRequestToken,
    },
    Loaded {
        block_id: SharedString,
        page: LoadedCardPage,
    },
    Failed {
        block_id: SharedString,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NotionSearchRequestToken(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionSearchRecentsCommit {
    Ignored,
    ShowingRecents,
    AwaitingDebouncedQuery,
    RequeryImmediately,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionSearchRecencyBucket {
    Today,
    Yesterday,
    PastWeek,
    PastThirtyDays,
    Older,
}

impl NotionSearchRecencyBucket {
    const ALL: [Self; 5] = [
        Self::Today,
        Self::Yesterday,
        Self::PastWeek,
        Self::PastThirtyDays,
        Self::Older,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::PastWeek => "Past week",
            Self::PastThirtyDays => "Past 30 days",
            Self::Older => "Older",
        }
    }

    pub(crate) const fn selector_segment(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::PastWeek => "past-week",
            Self::PastThirtyDays => "past-30-days",
            Self::Older => "older",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionSearchListRow {
    QueryHeader,
    RecencyHeader(NotionSearchRecencyBucket),
    QueryResult(usize),
    RecentResult(usize),
}

impl NotionSearchListRow {
    const fn result_index(self) -> Option<usize> {
        match self {
            Self::QueryResult(index) | Self::RecentResult(index) => Some(index),
            Self::QueryHeader | Self::RecencyHeader(_) => None,
        }
    }
}

#[derive(Clone, Default)]
enum NotionSearchRecentsCache {
    #[default]
    Empty,
    Loaded {
        refreshed_at: Option<Instant>,
        results: Arc<[RecentPageResult]>,
    },
}

#[derive(Clone)]
struct NotionSearchQueryCacheEntry {
    query: String,
    scope: SearchWorkspaceScope,
    total: u32,
    has_more: bool,
    local_result_count: usize,
    results: Arc<[PageShellSearchResult]>,
}

#[derive(Clone)]
struct NotionSearchLocalRequest {
    token: NotionSearchRequestToken,
    query_was_indexed: bool,
    results: Arc<[PageShellSearchResult]>,
}

#[derive(Clone)]
struct NotionSearchPreviewCacheEntry {
    block_id: String,
    page: LoadedCardPage,
}

const NOTION_SEARCH_RECENTS_FRESH_FOR: Duration = Duration::from_secs(60);
const NOTION_SEARCH_QUERY_CACHE_LIMIT: usize = 8;
const NOTION_SEARCH_PREVIEW_CACHE_LIMIT: usize = 12;
const NOTION_SEARCH_LOCAL_RESULT_LIMIT: usize = 3;
const NOTION_SEARCH_RECENT_PAGE_WEIGHT: f64 = 10.0;
const NOTION_SEARCH_RECENCY_RANK_WEIGHT: f64 = 100.0;
const NOTION_SEARCH_TITLE_MATCH_DENSITY_WEIGHT: f64 = 25.0;

pub(crate) struct NotionSearchState {
    pub(crate) input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) query: String,
    pub(crate) scope: SearchWorkspaceScope,
    pub(crate) session_id: String,
    pub(crate) flow_number: u32,
    pub(crate) selected_index: usize,
    pub(crate) result_limit: u32,
    pub(crate) results: NotionSearchResultsState,
    pub(crate) preview: NotionSearchPreviewState,
    pub(crate) preview_scroll_handle: ScrollHandle,
    pub(crate) list_state: ListState,
    list_rows: Arc<[NotionSearchListRow]>,
    request_token: NotionSearchRequestToken,
    query_request_dispatched: bool,
    pagination_request: Option<(NotionSearchRequestToken, u32)>,
    preview_requests: HashMap<String, NotionSearchRequestToken>,
    recents_refresh_token: Option<NotionSearchRequestToken>,
    recents_dirty_during_refresh: HashSet<String>,
    cache_scope: Option<String>,
    recents_snapshot_prepared: bool,
    local_search_snapshot_prepared: bool,
    current_page_id: Option<String>,
    current_page_url: Option<String>,
    recents_cache: NotionSearchRecentsCache,
    local_search_cache: QuickFindLocalSearchCache,
    local_request: Option<NotionSearchLocalRequest>,
    query_cache: VecDeque<NotionSearchQueryCacheEntry>,
    preview_cache: VecDeque<NotionSearchPreviewCacheEntry>,
    dirty_page_identities: HashSet<(String, String)>,
}

impl Default for NotionSearchState {
    fn default() -> Self {
        Self {
            input: RefCell::new(None),
            query: String::new(),
            scope: SearchWorkspaceScope::default(),
            session_id: String::new(),
            flow_number: 0,
            selected_index: 0,
            result_limit: 20,
            results: NotionSearchResultsState::Idle,
            preview: NotionSearchPreviewState::Idle,
            preview_scroll_handle: ScrollHandle::new(),
            list_state: ListState::new(0, ListAlignment::Top, px(36.0)),
            list_rows: Arc::default(),
            request_token: next_notion_search_request_token(),
            query_request_dispatched: false,
            pagination_request: None,
            preview_requests: HashMap::new(),
            recents_refresh_token: None,
            recents_dirty_during_refresh: HashSet::new(),
            cache_scope: None,
            recents_snapshot_prepared: false,
            local_search_snapshot_prepared: false,
            current_page_id: None,
            current_page_url: None,
            recents_cache: NotionSearchRecentsCache::Empty,
            local_search_cache: QuickFindLocalSearchCache::default(),
            local_request: None,
            query_cache: VecDeque::new(),
            preview_cache: VecDeque::new(),
            dirty_page_identities: HashSet::new(),
        }
    }
}

fn next_notion_search_request_token() -> NotionSearchRequestToken {
    static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
    let token = NEXT_TOKEN
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |token| {
            token.checked_add(1)
        })
        .expect("Notion search request token overflowed");
    NotionSearchRequestToken(token)
}
