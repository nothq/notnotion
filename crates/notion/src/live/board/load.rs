mod database_query;
mod database_references;
mod database_view;
mod date_results;
mod filter;
mod page;
mod sidebar;
pub(super) mod snapshot;
mod workspace;

pub(crate) use date_results::{
    query_calendar_items, query_date_undated_count, LiveCollectionQueryState,
};
pub(super) use filter::{filter_group_value, property_filter_value};
pub(crate) use filter::{
    load_database_filter_users, load_visible_workspace_users,
    search_database_filter_relation_pages, LiveDatabaseQueryState,
};
pub(super) use sidebar::explicit_page_shell_icon;
pub(crate) use sidebar::{
    load_recent_pages, load_sidebar, load_sidebar_calendar_for_context, load_sidebar_chats,
    load_sidebar_children, load_sidebar_inbox, mutate_sidebar_inbox, search_workspace,
};
pub use workspace::load_board_snapshot;
pub(crate) use workspace::{
    load_board_snapshot_for_bootstrap, load_board_snapshot_for_navigation,
    load_database_filter_snapshot_for_query, load_inline_database_snapshot_from_bootstrap,
};

use super::{
    block_value, board_active_view_group, board_active_view_property_layout,
    board_active_view_sorts, board_calendar_view, board_columns, board_database_properties,
    board_group_by, board_item_snapshot, board_page_title, board_table_view_columns,
    board_timeline_view, collection_database_name, collection_entry, collection_view_collection_id,
    collection_view_entry, format_edited_label, optional_record_map_table, record_map_table,
    required_array, required_string, required_string_array, required_u64, resolve_active_view_id,
    status_option_colors, view_tab_kind, BoardColumn, BoardItem, BoardTarget, DatabaseProperty,
    DatabaseViewControlContext, HashMap, HashSet, LoadedBoardSnapshot, Map, PropertyLookup,
    TableViewColumn, TimelineViewConfig, UserContext, Value, ViewTab,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};
use crate::model::{
    CalendarViewConfig, DatabaseViewFilterState, DatabaseViewPropertyLayout,
    NotionCollectionViewId, PagePresenceSnapshot, PageShellSnapshot, ViewTabKind,
};
use database_query::{
    database_collection_query_state, database_filter_query_states, load_database_query_response,
    load_database_shell_and_presence, query_block_ids, query_board_data, validate_filter_query,
};
use database_view::{
    board_calendar_config, board_timeline_config, bootstrap_board_data, load_database_parts,
    DatabaseDateViews,
};
use filter::database_view_filter_state;
use page::database_page_shell_snapshot;
use snapshot::{build_loaded_board_snapshot, LoadedBoardSnapshotInput};

pub(crate) struct DatabaseFilterQueryInput {
    pub(crate) expected_view_id: NotionCollectionViewId,
    pub(crate) filter_state: DatabaseViewFilterState,
    pub(crate) persisted_filter_state: DatabaseViewFilterState,
    pub(crate) query_state: LiveDatabaseQueryState,
}

pub(super) struct BootstrapBoardData<'a> {
    pub(super) blocks: &'a Map<String, Value>,
    pub(super) collection_views: &'a Map<String, Value>,
    pub(super) collection_view_block: Value,
    pub(super) collection_id: String,
    pub(super) space_id: String,
    pub(super) favorite_space_view_id: Option<String>,
    pub(super) edited_label: String,
    pub(super) view_ids: Vec<Value>,
    pub(super) active_view_id: String,
    pub(super) active_view_kind: ViewTabKind,
    pub(super) group_by: Option<Value>,
}

pub(super) struct QueryBoardData<'a> {
    pub(super) reducer_name: &'static str,
    pub(super) collection_schema: Option<&'a Map<String, Value>>,
    pub(super) users: Option<&'a Map<String, Value>>,
    pub(super) query_blocks: &'a Map<String, Value>,
    pub(super) block_results: &'a Value,
    pub(super) database_title: String,
    pub(super) page_title: String,
    pub(super) option_colors: HashMap<String, String>,
}

pub(super) struct LoadedBoardParts {
    pub(super) columns: Vec<BoardColumn>,
    pub(super) view_tabs: Vec<ViewTab>,
    pub(super) items: Vec<BoardItem>,
    pub(super) table_view_columns: Vec<TableViewColumn>,
    pub(super) active_view_property_layout: DatabaseViewPropertyLayout,
    pub(super) view_control_context: DatabaseViewControlContext,
    pub(super) database_properties: Vec<DatabaseProperty>,
    pub(super) timeline_view: Option<TimelineViewConfig>,
    pub(super) calendar_view: Option<CalendarViewConfig>,
}

struct LiveDatabaseSnapshotInput<'a> {
    query_response: &'a Value,
    bootstrap_data: BootstrapBoardData<'a>,
    query_data: QueryBoardData<'a>,
    collection_query_state: Option<LiveCollectionQueryState>,
    timeline_view: Option<TimelineViewConfig>,
    calendar_view: Option<CalendarViewConfig>,
    page_shell: PageShellSnapshot,
    presence: Option<PagePresenceSnapshot>,
    query: Option<&'a DatabaseFilterQueryInput>,
}

struct LiveDatabaseLoadRequest<'a> {
    session: &'a NotionDesktopSession,
    board_target: BoardTarget,
    user_context: &'a UserContext,
    bootstrap: Value,
    current_page_shell: Option<&'a PageShellSnapshot>,
    load_presence: bool,
    presentation: DatabasePresentation,
    query: Option<DatabaseFilterQueryInput>,
}

#[derive(Clone, Copy)]
pub(super) enum DatabasePresentation {
    FullPage,
    Inline,
}

impl DatabasePresentation {
    const fn is_full_screen(self) -> bool {
        matches!(self, Self::FullPage)
    }
}

fn load_live_database_snapshot(
    request: LiveDatabaseLoadRequest<'_>,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let bootstrap_data = bootstrap_board_data(
        &request.bootstrap,
        &request.board_target,
        request.user_context,
    )?;
    let query = request.query.as_ref();
    validate_filter_query(&bootstrap_data, query)?;
    let (page_shell, presence) = load_database_shell_and_presence(&request, &bootstrap_data)?;
    let timeline_view = board_timeline_config(&bootstrap_data)?;
    let calendar_view = board_calendar_config(&bootstrap_data)?;
    let query_response = load_database_query_response(&request, &bootstrap_data)?;
    let collection_query_state = database_collection_query_state(
        &query_response,
        &bootstrap_data,
        timeline_view.as_ref(),
        calendar_view.as_ref(),
    )?;
    let query_data = query_board_data(
        &query_response,
        &request.bootstrap,
        &bootstrap_data,
        &request.board_target.collection_view_block_id,
    )?;
    build_live_database_snapshot(
        request.board_target,
        request.user_context,
        LiveDatabaseSnapshotInput {
            query_response: &query_response,
            bootstrap_data,
            query_data,
            collection_query_state,
            timeline_view,
            calendar_view,
            page_shell,
            presence,
            query,
        },
    )
}

fn build_live_database_snapshot(
    board_target: BoardTarget,
    user_context: &UserContext,
    input: LiveDatabaseSnapshotInput<'_>,
) -> Result<LoadedBoardSnapshot, NotionLiveError> {
    let LiveDatabaseSnapshotInput {
        query_response,
        bootstrap_data,
        query_data,
        collection_query_state,
        timeline_view,
        calendar_view,
        page_shell,
        presence,
        query,
    } = input;
    let (active_filter_state, database_query_state) =
        database_filter_query_states(query_response, &bootstrap_data, &query_data, query)?;
    let parts = load_database_parts(
        query_response,
        &bootstrap_data,
        &query_data,
        &active_filter_state,
        DatabaseDateViews {
            timeline_view,
            calendar_view,
        },
    )?;
    Ok(build_loaded_board_snapshot(LoadedBoardSnapshotInput {
        board_target,
        user_context,
        presence,
        page_shell,
        bootstrap_data,
        query_data,
        collection_query_state,
        database_query_state,
        parts,
    }))
}
