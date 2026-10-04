use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Condvar, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::live::http::NotionPrivateApiEndpoint;
use crate::model::{
    BoardColumn, BoardDateValue, BoardItem, BoardSnapshot, CardPage, CardPageBlock,
    CardPageBlockKind, CardPageProperty, CardPageStructuralBlock, DatabaseProperty,
    DatabaseViewFilterState, DatabaseViewGroupState, DatabaseViewPropertyLayout,
    DatabaseViewSortState, NotionCustomEmojiLibrary, PagePresenceProfile, PagePresenceSnapshot,
    PageShellNodeIdentity, PageShellSearchResult, PageShellSidebarItem, PageShellSnapshot,
    TableViewColumn, TimelineViewConfig, ViewTab, ViewTabKind,
};
use reqwest::Url;
use serde_json::{Map, Value};

mod column_ratio;
mod columns;
mod comments;
mod context;
mod item;
mod load;
mod mutate;
mod navigation_cache;
mod page;
mod page_chunk;
mod page_mutation;
mod page_state;
mod presence;
mod quick_find_records;
mod record_map;
mod sharing;
mod simple_table_rich_text;
mod support;
mod user;

use columns::board_columns;
use context::find_sidebar_item;
use item::board_item_snapshot;
pub use load::load_board_snapshot;
pub(crate) use load::{
    load_board_snapshot_for_bootstrap, load_board_snapshot_for_navigation,
    load_database_filter_snapshot_for_query, load_database_filter_users,
    load_inline_database_snapshot_from_bootstrap, load_recent_pages, load_sidebar_chats,
    load_sidebar_children, load_sidebar_inbox, load_visible_workspace_users, mutate_sidebar_inbox,
    query_calendar_items, query_date_undated_count, search_database_filter_relation_pages,
    search_workspace, DatabaseFilterQueryInput, LiveCollectionQueryState, LiveDatabaseQueryState,
};
use navigation_cache::NavigationBootstrapCache;
pub use page::load_card_page;
pub(in crate::live) use page::load_card_page_snapshot_with_session;
use page_chunk::{
    load_complete_page_response_typed, load_complete_page_response_with_session,
    CompletePageResponse, ProvenOpaqueUnavailableBlocks,
};
use presence::load_page_presence;
pub(crate) use sharing::{load_page_sharing, mutate_page_sharing};
pub use support::view::canonicalize_board_url;
pub(crate) use support::{property::*, record::*, view::*};
use user::{
    complete_user_context_with_session, load_user_context_with_session,
    request_initial_user_context_with_session, BoardTarget, UserContext,
};

#[derive(Clone, Debug)]
pub struct LoadedBoardSnapshot {
    pub snapshot: BoardSnapshot,
    pub mutator: Option<LiveBoardMutator>,
    pub(crate) workspace_context: Option<LiveWorkspaceContext>,
    pub(crate) collection_query_state: Option<LiveCollectionQueryState>,
    pub(crate) database_query_state: Option<LiveDatabaseQueryState>,
}

#[derive(Clone, Debug)]
pub(crate) struct LiveWorkspaceContext {
    board_url: String,
    page_block_id: String,
    workspace_cache: Arc<Mutex<LiveWorkspaceCache>>,
    page_sidebar_item: PageShellSidebarItem,
    quick_find_records: Arc<quick_find_records::QuickFindRecords>,
}

#[derive(Clone, Debug)]
pub(crate) struct LiveWorkspaceSearchContext {
    pub(crate) space_id: String,
    pub(crate) active_user_id: String,
    pub(crate) active_user_profile: PagePresenceProfile,
    pub(crate) time_zone: String,
    pub(crate) sidebar_breadcrumbs_by_block_id: Arc<HashMap<String, String>>,
    quick_find_records: Arc<quick_find_records::QuickFindRecords>,
}

#[derive(Debug)]
struct LiveWorkspaceCache {
    space_id: String,
    user_context: Arc<UserContext>,
    page_shell: PageShellSnapshot,
    quick_find_breadcrumbs_by_block_id: Arc<HashMap<String, String>>,
    sidebar_hydrated: bool,
    favorite_worker_running: bool,
    authoritative_favorites_by_page: HashMap<String, bool>,
    pending_favorite_mutations: VecDeque<PendingFavoriteMutation>,
    custom_emoji_library: Option<LiveCustomEmojiLibraryCache>,
    custom_emoji_library_revision: u64,
    custom_emoji_library_loading_revision: Option<u64>,
    custom_emoji_library_changed: Arc<Condvar>,
    custom_emoji_library_failure: Option<LiveCustomEmojiLibraryFailure>,
    navigation_bootstraps: NavigationBootstrapCache,
}

#[derive(Clone, Debug)]
struct LiveCustomEmojiLibraryCache {
    fetched_at: Instant,
    library: NotionCustomEmojiLibrary,
}

#[derive(Clone, Debug)]
struct LiveCustomEmojiLibraryFailure {
    failed_at: Instant,
    revision: u64,
    error: String,
}

#[derive(Clone, Debug)]
struct PendingFavoriteMutation {
    page_block_id: String,
    is_favorited: bool,
    page_sidebar_item: PageShellSidebarItem,
    mutation_context: FavoriteMutationContext,
}

struct LiveWorkspaceCacheSnapshot {
    space_id: String,
    user_context: Arc<UserContext>,
    page_shell: PageShellSnapshot,
    sidebar_hydrated: bool,
    shared: Arc<Mutex<LiveWorkspaceCache>>,
}

#[derive(Clone, Debug)]
pub(crate) struct FavoriteMutationContext {
    space_id: String,
    space_short_id: NotionSpaceShortId,
    user_id: String,
    favorite_space_view_id: Option<String>,
    page_block_id: String,
}

#[derive(Clone, Debug)]
struct DatabaseMutationContext {
    favorite: FavoriteMutationContext,
    collection_id: String,
    collection_view_id: String,
    board_group: Option<BoardGroupWrite>,
    filter_state: DatabaseViewFilterState,
    view_controls: DatabaseViewControlContext,
}

/// The option property a board writes when a card lands in a column: a
/// status, select, or multi-select property whose options name the columns.
#[derive(Clone, Debug)]
struct BoardGroupWrite {
    property_id: String,
    options: HashSet<String>,
    multi_valued: bool,
}

impl BoardGroupWrite {
    fn property_for_new_card(&self, column_title: &str) -> Result<&str, String> {
        if self.options.contains(column_title) {
            return Ok(&self.property_id);
        }
        Err(format!(
            "Notion board column {column_title} is not an option of property {}",
            self.property_id
        ))
    }

    /// Moving a card between multi-select columns would also have to drop the
    /// source option from the card's other options.
    fn property_for_moved_card(&self, column_title: &str) -> Result<&str, String> {
        if self.multi_valued {
            return Err("Notion board cards cannot move between multi-select columns".to_string());
        }
        self.property_for_new_card(column_title)
    }
}

impl DatabaseMutationContext {
    fn board_group(&self) -> Result<&BoardGroupWrite, String> {
        self.board_group.as_ref().ok_or_else(|| {
            "Notion board columns accept cards only for status, select, or multi-select groups"
                .to_string()
        })
    }
}

#[derive(Clone, Debug)]
struct DatabaseViewControlContext {
    view_kind: ViewTabKind,
    query2: Map<String, Value>,
    format: Map<String, Value>,
    sorts: DatabaseViewSortState,
    group: DatabaseViewGroupState,
    property_layout: DatabaseViewPropertyLayout,
    property_types: HashMap<String, String>,
}

#[derive(Clone, Debug)]
enum MutationContext {
    Page(FavoriteMutationContext),
    Database(Box<DatabaseMutationContext>),
}

#[derive(Clone, Debug)]
pub struct LiveBoardMutator {
    context: MutationContext,
    page_states: HashMap<String, page_state::PageMutationState>,
    page_responses: HashMap<String, CompletePageResponse>,
    page_crdt_clocks: HashMap<String, page_mutation::CrdtClock>,
}

impl LiveBoardMutator {
    pub(crate) fn database_filter_state_for_view(
        &self,
        expected_view_id: &crate::model::NotionCollectionViewId,
    ) -> Result<DatabaseViewFilterState, String> {
        let MutationContext::Database(context) = &self.context else {
            return Err("Notion database filter query is unavailable for a page".to_string());
        };
        if expected_view_id.as_str() != context.collection_view_id {
            return Err(format!(
                "Notion database filter query expected view {}, active view is {}",
                expected_view_id.as_str(),
                context.collection_view_id
            ));
        }
        Ok(context.filter_state.clone())
    }

    pub(crate) fn set_database_filter_state_for_view(
        &mut self,
        expected_view_id: &crate::model::NotionCollectionViewId,
        filter_state: DatabaseViewFilterState,
    ) -> Result<(), String> {
        let MutationContext::Database(context) = &mut self.context else {
            return Err("Notion database filter state is unavailable for a page".to_string());
        };
        if expected_view_id.as_str() != context.collection_view_id {
            return Err(format!(
                "Notion database filter state expected view {}, active view is {}",
                expected_view_id.as_str(),
                context.collection_view_id
            ));
        }
        context.filter_state = filter_state;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoveCardMutationRequest<'a> {
    pub block_id: &'a str,
    pub source_column_title: &'a str,
    pub target_column_title: &'a str,
    pub before_block_id: Option<&'a str>,
    pub after_block_id: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub(crate) struct CalendarDateMutation {
    pub(crate) block_id: String,
    pub(crate) date_property_id: String,
    pub(crate) date_value: Value,
    pub(crate) source: crate::model::CalendarDateAssignmentSource,
}

#[derive(Clone, Debug)]
pub(crate) struct CalendarPageCreation {
    pub(crate) date_property_id: String,
    pub(crate) date_value: Value,
}

fn reject_transaction_errors(response: &Value, transaction_kind: &str) -> Result<(), String> {
    let client_data = response
        .get("clientData")
        .or_else(|| response.get("body").and_then(|body| body.get("clientData")));
    let Some(errors) = client_data
        .and_then(|data| data.get("errors"))
        .and_then(Value::as_array)
        .filter(|errors| !errors.is_empty())
    else {
        return Ok(());
    };
    let names = errors
        .iter()
        .filter_map(|error| error.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    Err(if names.is_empty() {
        format!(
            "Notion rejected {} {transaction_kind} transaction(s)",
            errors.len()
        )
    } else {
        format!(
            "Notion rejected {transaction_kind} transaction: {}",
            names.join(", ")
        )
    })
}
