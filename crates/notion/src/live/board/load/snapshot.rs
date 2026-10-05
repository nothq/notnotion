use super::super::{
    board_is_locked, board_is_private, BoardColumn, BoardGroupWrite, BoardItem, BoardSnapshot,
    BoardTarget, CardPage, CompletePageResponse, DatabaseMutationContext, DatabaseProperty,
    FavoriteMutationContext, LiveBoardMutator, LoadedBoardSnapshot, Map, MutationContext,
    PagePresenceSnapshot, PageShellSnapshot, TableViewColumn, TimelineViewConfig, UserContext,
    Value, ViewTab,
};
use super::{BootstrapBoardData, LiveCollectionQueryState, LoadedBoardParts, QueryBoardData};
use crate::model::CalendarViewConfig;

pub(super) struct LoadedBoardSnapshotInput<'a> {
    pub(super) board_target: BoardTarget,
    pub(super) user_context: &'a UserContext,
    pub(super) presence: Option<PagePresenceSnapshot>,
    pub(super) page_shell: PageShellSnapshot,
    pub(super) bootstrap_data: BootstrapBoardData<'a>,
    pub(super) query_data: QueryBoardData<'a>,
    pub(super) collection_query_state: Option<LiveCollectionQueryState>,
    pub(super) database_query_state: Option<super::LiveDatabaseQueryState>,
    pub(super) parts: LoadedBoardParts,
}

pub(super) struct LoadedPageSnapshotInput<'a> {
    pub(super) board_target: BoardTarget,
    pub(super) user_context: &'a UserContext,
    pub(super) root: Value,
    pub(super) page: CardPage,
    pub(super) page_mutation_state: super::super::page_state::PageMutationState,
    pub(super) page_response: CompletePageResponse,
    pub(super) page_shell: PageShellSnapshot,
    pub(super) presence: Option<PagePresenceSnapshot>,
    pub(super) space_id: String,
    pub(super) edited_label: String,
}

struct PageSnapshotInput {
    page_block_id: String,
    user_time_zone: String,
    user_utc_offset_seconds: i32,
    root: Value,
    page: CardPage,
    page_shell: PageShellSnapshot,
    presence: Option<PagePresenceSnapshot>,
    edited_label: String,
    is_favorited: bool,
}

struct LoadedBoardMutatorInput {
    page_block_id: String,
    user_id: String,
    space_short_id: super::super::NotionSpaceShortId,
    collection_id: String,
    space_id: String,
    favorite_space_view_id: Option<String>,
    active_view_id: String,
    board_group: Option<BoardGroupWrite>,
    filter_state: crate::model::DatabaseViewFilterState,
    view_controls: super::super::DatabaseViewControlContext,
}

struct BoardSnapshotInput {
    page_title: String,
    database_title: String,
    edited_label: String,
    presence: Option<PagePresenceSnapshot>,
    collection_view_block: Value,
    favorite_space_view_id: Option<String>,
    columns: Vec<BoardColumn>,
    view_tabs: Vec<ViewTab>,
    items: Vec<BoardItem>,
    table_view_columns: Vec<TableViewColumn>,
    active_view_property_layout: crate::model::DatabaseViewPropertyLayout,
    active_view_sorts: crate::model::DatabaseViewSortState,
    active_view_group: crate::model::DatabaseViewGroupState,
    database_properties: Vec<DatabaseProperty>,
    timeline_view: Option<TimelineViewConfig>,
    calendar_view: Option<CalendarViewConfig>,
    page_shell: PageShellSnapshot,
}

pub(super) fn build_loaded_board_snapshot(
    input: LoadedBoardSnapshotInput<'_>,
) -> LoadedBoardSnapshot {
    let filter_state = input
        .parts
        .view_tabs
        .iter()
        .find(|view| view.active)
        .and_then(|view| view.filters.clone())
        .expect("active Notion database view must retain parsed filter state");
    let board_block_id = input.board_target.collection_view_block_id.clone();
    let mutator = build_loaded_board_mutator(loaded_board_mutator_input(
        &input,
        filter_state,
        board_block_id.clone(),
    ));
    let user_context = input.user_context;
    let collection_query_state = input.collection_query_state.clone();
    let database_query_state = input.database_query_state.clone();
    let snapshot = build_board_snapshot(board_snapshot_input(input), user_context, &board_block_id);
    LoadedBoardSnapshot {
        snapshot,
        mutator: Some(mutator),
        workspace_context: None,
        collection_query_state,
        database_query_state,
    }
}

fn loaded_board_mutator_input(
    input: &LoadedBoardSnapshotInput<'_>,
    filter_state: crate::model::DatabaseViewFilterState,
    page_block_id: String,
) -> LoadedBoardMutatorInput {
    LoadedBoardMutatorInput {
        page_block_id,
        user_id: input.user_context.user_id.clone(),
        space_short_id: input.user_context.space_short_id,
        collection_id: input.bootstrap_data.collection_id.clone(),
        space_id: input.bootstrap_data.space_id.clone(),
        favorite_space_view_id: input.bootstrap_data.favorite_space_view_id.clone(),
        active_view_id: input.bootstrap_data.active_view_id.clone(),
        board_group: board_group_write(
            input.bootstrap_data.group_by.as_ref(),
            input.query_data.collection_schema,
        ),
        filter_state,
        view_controls: input.parts.view_control_context.clone(),
    }
}

fn board_snapshot_input(input: LoadedBoardSnapshotInput<'_>) -> BoardSnapshotInput {
    BoardSnapshotInput {
        page_title: input.query_data.page_title,
        database_title: input.query_data.database_title,
        edited_label: input.bootstrap_data.edited_label,
        presence: input.presence,
        collection_view_block: input.bootstrap_data.collection_view_block,
        favorite_space_view_id: input.bootstrap_data.favorite_space_view_id,
        columns: input.parts.columns,
        view_tabs: input.parts.view_tabs,
        items: input.parts.items,
        table_view_columns: input.parts.table_view_columns,
        active_view_property_layout: input.parts.active_view_property_layout,
        active_view_sorts: input.parts.view_control_context.sorts.clone(),
        active_view_group: input.parts.view_control_context.group.clone(),
        database_properties: input.parts.database_properties,
        timeline_view: input.parts.timeline_view,
        calendar_view: input.parts.calendar_view,
        page_shell: input.page_shell,
    }
}

pub(super) fn build_loaded_page_snapshot(
    input: LoadedPageSnapshotInput<'_>,
) -> LoadedBoardSnapshot {
    let LoadedPageSnapshotInput {
        board_target,
        user_context,
        root,
        page,
        page_mutation_state,
        page_response,
        page_shell,
        presence,
        space_id,
        edited_label,
    } = input;
    let favorite_space_view_id = user_context
        .space_view_id_for_space(&space_id)
        .map(str::to_string);
    let page_block_id = board_target.collection_view_block_id;
    let is_favorited = favorite_space_view_id
        .as_deref()
        .is_some_and(|space_view_id| {
            user_context.is_page_bookmarked(space_view_id, &page_block_id)
        });
    let snapshot = build_page_snapshot(PageSnapshotInput {
        page_block_id: page_block_id.clone(),
        user_time_zone: user_context.time_zone.clone(),
        user_utc_offset_seconds: user_context.utc_offset_seconds,
        root,
        page,
        page_shell,
        presence,
        edited_label,
        is_favorited,
    });
    let mutator = build_page_mutator(
        FavoriteMutationContext {
            space_id,
            space_short_id: user_context.space_short_id,
            user_id: user_context.user_id.clone(),
            favorite_space_view_id,
            page_block_id,
        },
        page_mutation_state,
        page_response,
    );
    LoadedBoardSnapshot {
        snapshot,
        mutator: Some(mutator),
        workspace_context: None,
        collection_query_state: None,
        database_query_state: None,
    }
}

fn build_page_snapshot(input: PageSnapshotInput) -> BoardSnapshot {
    let PageSnapshotInput {
        page_block_id,
        user_time_zone,
        user_utc_offset_seconds,
        root,
        page,
        page_shell,
        presence,
        edited_label,
        is_favorited,
    } = input;
    let page_title = page.title.clone();
    BoardSnapshot {
        share_target_id: Some(
            page_block_id
                .parse()
                .expect("parsed Notion page target must have a non-empty block ID"),
        ),
        page_title: page_title.clone(),
        database_title: page_title,
        edited_label,
        user_time_zone,
        user_utc_offset_seconds,
        is_private: board_is_private(&root),
        is_locked: board_is_locked(&root),
        is_favorited,
        presence,
        columns: Vec::new(),
        view_tabs: Vec::new(),
        items: Vec::new(),
        table_view_columns: Vec::new(),
        active_view_property_layout: Default::default(),
        active_view_sorts: Default::default(),
        active_view_group: Default::default(),
        database_properties: Vec::new(),
        timeline_view: None,
        calendar_view: None,
        date_undated_count: None,
        page_content: Some(page),
        page_shell: Some(page_shell),
    }
}

fn build_page_mutator(
    context: FavoriteMutationContext,
    page_mutation_state: super::super::page_state::PageMutationState,
    page_response: CompletePageResponse,
) -> LiveBoardMutator {
    let page_block_id = context.page_block_id.clone();
    LiveBoardMutator {
        context: MutationContext::Page(context),
        page_states: [(
            page_mutation_state.page_block_id.clone(),
            page_mutation_state,
        )]
        .into_iter()
        .collect(),
        page_responses: [(page_block_id, page_response)].into_iter().collect(),
        page_crdt_clocks: Default::default(),
    }
}

fn build_board_snapshot(
    input: BoardSnapshotInput,
    user_context: &UserContext,
    collection_view_block_id: &str,
) -> BoardSnapshot {
    BoardSnapshot {
        share_target_id: Some(
            collection_view_block_id
                .parse()
                .expect("parsed Notion board target must have a non-empty block ID"),
        ),
        page_title: input.page_title,
        database_title: input.database_title,
        edited_label: input.edited_label,
        user_time_zone: user_context.time_zone.clone(),
        user_utc_offset_seconds: user_context.utc_offset_seconds,
        is_private: board_is_private(&input.collection_view_block),
        is_locked: board_is_locked(&input.collection_view_block),
        is_favorited: input
            .favorite_space_view_id
            .as_deref()
            .is_some_and(|space_view_id| {
                user_context.is_page_bookmarked(space_view_id, collection_view_block_id)
            }),
        presence: input.presence,
        columns: input.columns,
        view_tabs: input.view_tabs,
        items: input.items,
        table_view_columns: input.table_view_columns,
        active_view_property_layout: input.active_view_property_layout,
        active_view_sorts: input.active_view_sorts,
        active_view_group: input.active_view_group,
        database_properties: input.database_properties,
        timeline_view: input.timeline_view,
        calendar_view: input.calendar_view,
        date_undated_count: None,
        page_content: None,
        page_shell: Some(input.page_shell),
    }
}

fn board_group_write(
    group_by: Option<&Value>,
    collection_schema: Option<&Map<String, Value>>,
) -> Option<BoardGroupWrite> {
    let property_id = group_by?.get("property")?.as_str()?;
    let property = collection_schema?.get(property_id)?;
    let multi_valued = match property.get("type").and_then(Value::as_str) {
        Some("status" | "select") => false,
        Some("multi_select") => true,
        _ => return None,
    };
    let options = property
        .get("options")?
        .as_array()?
        .iter()
        .filter_map(|option| option.get("value").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    Some(BoardGroupWrite {
        property_id: property_id.to_string(),
        options,
        multi_valued,
    })
}

fn build_loaded_board_mutator(input: LoadedBoardMutatorInput) -> LiveBoardMutator {
    LiveBoardMutator {
        context: MutationContext::Database(Box::new(DatabaseMutationContext {
            favorite: FavoriteMutationContext {
                space_id: input.space_id,
                space_short_id: input.space_short_id,
                user_id: input.user_id,
                favorite_space_view_id: input.favorite_space_view_id,
                page_block_id: input.page_block_id,
            },
            collection_id: input.collection_id,
            collection_view_id: input.active_view_id,
            board_group: input.board_group,
            filter_state: input.filter_state,
            view_controls: input.view_controls,
        })),
        page_states: Default::default(),
        page_responses: Default::default(),
        page_crdt_clocks: Default::default(),
    }
}
