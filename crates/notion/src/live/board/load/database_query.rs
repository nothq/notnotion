use serde_json::json;

use super::database_references::hydrate_row_references;
use super::page::load_optional_page_presence;
use crate::live::board::record_map::merge_record_map;
use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_with_session, NotionPrivateApiEndpoint},
    NotionLiveError,
};

use super::{
    board_page_title, collection_database_name, collection_entry, collection_view_entry,
    database_page_shell_snapshot, database_view_filter_state, optional_record_map_table,
    record_map_table, required_string, required_string_array, status_option_colors,
    BootstrapBoardData, CalendarViewConfig, DatabaseFilterQueryInput, DatabasePresentation,
    DatabaseViewFilterState, LiveCollectionQueryState, LiveDatabaseLoadRequest,
    LiveDatabaseQueryState, NotionCollectionViewId, PagePresenceSnapshot, PageShellSnapshot,
    QueryBoardData, TimelineViewConfig, UserContext, Value, ViewTabKind,
};

pub(super) fn validate_filter_query(
    bootstrap: &BootstrapBoardData<'_>,
    query: Option<&DatabaseFilterQueryInput>,
) -> Result<(), String> {
    let Some(query) = query else {
        return Ok(());
    };
    if query.expected_view_id.as_str() == bootstrap.active_view_id {
        return Ok(());
    }
    Err(format!(
        "Notion database filter query expected view {}, active view is {}",
        query.expected_view_id.as_str(),
        bootstrap.active_view_id
    ))
}

pub(super) fn load_database_query_response(
    request: &LiveDatabaseLoadRequest<'_>,
    bootstrap_data: &BootstrapBoardData<'_>,
) -> Result<Value, NotionLiveError> {
    let session = request.session;
    let response = match request.query.as_ref() {
        Some(query) => post_private_api_with_session(
            session,
            NotionPrivateApiEndpoint::QueryCollectionInitialLoad,
            &query
                .query_state
                .query_request(&query.expected_view_id, &query.filter_state)?,
        )?,
        None => load_board_query(
            session,
            bootstrap_data,
            request.user_context,
            request.presentation,
        )?,
    };
    let mut response =
        complete_query_response(&request.bootstrap, response).map_err(NotionLiveError::Fatal)?;
    hydrate_row_references(
        session,
        &bootstrap_data.space_id,
        &bootstrap_data.collection_id,
        &mut response,
    )?;
    Ok(response)
}

pub(super) fn database_collection_query_state(
    query_response: &Value,
    bootstrap: &BootstrapBoardData<'_>,
    timeline_view: Option<&TimelineViewConfig>,
    calendar_view: Option<&CalendarViewConfig>,
) -> Result<Option<LiveCollectionQueryState>, String> {
    active_date_property_id(bootstrap.active_view_kind, timeline_view, calendar_view)
        .map(|date_property_id| {
            LiveCollectionQueryState::from_initial_query_response(
                query_response,
                bootstrap.collection_id.clone(),
                date_property_id.to_string(),
                bootstrap.active_view_kind == ViewTabKind::Calendar,
            )
        })
        .transpose()
}

/// The active filter state and, when compiled, the live database query state.
type DatabaseFilterQueryStates = (DatabaseViewFilterState, Option<LiveDatabaseQueryState>);

pub(super) fn database_filter_query_states(
    query_response: &Value,
    bootstrap: &BootstrapBoardData<'_>,
    query_data: &QueryBoardData<'_>,
    query: Option<&DatabaseFilterQueryInput>,
) -> Result<DatabaseFilterQueryStates, String> {
    let active_view = collection_view_entry(bootstrap.collection_views, &bootstrap.active_view_id)?;
    let parsed = database_view_filter_state(active_view, query_data.collection_schema);
    let active = query
        .map(|query| query.persisted_filter_state.clone())
        .unwrap_or_else(|| parsed.clone());
    let query_state = match query {
        Some(query) => Some(query.query_state.clone()),
        None => LiveDatabaseQueryState::from_initial_query_response(
            query_response,
            query_data.reducer_name,
            NotionCollectionViewId::try_from(bootstrap.active_view_id.clone())?,
            parsed,
        ),
    };
    Ok((active, query_state))
}

fn active_date_property_id<'a>(
    active_view_kind: ViewTabKind,
    timeline_view: Option<&'a TimelineViewConfig>,
    calendar_view: Option<&'a CalendarViewConfig>,
) -> Option<&'a str> {
    match active_view_kind {
        ViewTabKind::Timeline => Some(
            timeline_view
                .expect("active timeline view must include timeline configuration")
                .date_property_id
                .as_str(),
        ),
        ViewTabKind::Calendar => Some(
            calendar_view
                .expect("active calendar view must include calendar configuration")
                .date_property_id
                .as_str(),
        ),
        ViewTabKind::Board
        | ViewTabKind::Table
        | ViewTabKind::List
        | ViewTabKind::Gallery
        | ViewTabKind::Unknown => None,
    }
}

fn complete_query_response(bootstrap: &Value, mut query_response: Value) -> Result<Value, String> {
    let bootstrap_record_map = bootstrap
        .get("recordMap")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing recordMap in Notion database bootstrap".to_string())?;
    let query_response_object = query_response
        .as_object_mut()
        .ok_or_else(|| "Notion query collection response is not an object".to_string())?;
    let query_record_map = match query_response_object.remove("recordMap") {
        Some(Value::Object(record_map)) => record_map,
        _ => {
            return Err("missing recordMap object in Notion query collection response".to_string());
        }
    };
    let mut record_map = bootstrap_record_map.clone();
    merge_record_map(&mut record_map, query_record_map)?;
    query_response_object.insert("recordMap".to_string(), Value::Object(record_map));
    Ok(query_response)
}

type DatabaseShellAndPresence = (PageShellSnapshot, Option<PagePresenceSnapshot>);

pub(super) fn load_database_shell_and_presence(
    request: &LiveDatabaseLoadRequest<'_>,
    bootstrap_data: &BootstrapBoardData<'_>,
) -> Result<DatabaseShellAndPresence, NotionLiveError> {
    let page_shell = database_page_shell_snapshot(
        bootstrap_data,
        &request.bootstrap,
        &request.board_target,
        request.user_context,
        request.current_page_shell,
    )?;
    let presence = if request.load_presence {
        load_optional_page_presence(
            request.session,
            &request.board_target,
            &bootstrap_data.space_id,
            request.user_context,
        )?
    } else {
        None
    };
    Ok((page_shell, presence))
}

pub(super) fn query_block_ids(query_data: &QueryBoardData<'_>) -> Result<Vec<String>, String> {
    required_string_array(query_data.block_results, "blockIds")
}

fn load_board_query(
    session: &NotionDesktopSession,
    bootstrap: &BootstrapBoardData<'_>,
    user_context: &UserContext,
    presentation: DatabasePresentation,
) -> Result<Value, NotionLiveError> {
    post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::QueryCollectionInitialLoad,
        &json!({
            "collectionView": { "id": bootstrap.active_view_id },
            "collectionViewBlock": {
                "id": required_string(&bootstrap.collection_view_block, "id")?
            },
            "clientType": "notion_app",
            "userTimeZone": user_context.time_zone,
            "isFullScreen": presentation.is_full_screen(),
            "isMobile": false,
        }),
    )
}

pub(super) fn query_board_data<'a>(
    query_response: &'a Value,
    bootstrap: &'a Value,
    bootstrap_data: &BootstrapBoardData<'a>,
    board_block_id: &str,
) -> Result<QueryBoardData<'a>, String> {
    let collection = collection_entry(
        record_map_table(query_response, "collection")?,
        &bootstrap_data.collection_id,
    )?;
    let collection_schema = collection.get("schema").and_then(Value::as_object);
    let users = optional_record_map_table(query_response, "notion_user")
        .or_else(|| optional_record_map_table(bootstrap, "notion_user"));
    let database_title = collection_database_name(collection)?;
    let page_title = board_page_title(
        bootstrap_data.blocks,
        &bootstrap_data.collection_view_block,
        &database_title,
    )?
    .ok_or_else(|| format!("missing Notion page title for board {board_block_id}"))?;
    let option_colors = status_option_colors(
        collection,
        bootstrap_data
            .group_by
            .as_ref()
            .and_then(|group_by| group_by.get("property"))
            .and_then(Value::as_str),
    )?;
    let (reducer_name, reducer) = active_reducer(query_response, bootstrap_data.active_view_kind)?;
    let block_results = if bootstrap_data.active_view_kind == ViewTabKind::Calendar {
        reducer
    } else {
        reducer.get("blockResults").unwrap_or(reducer)
    };
    Ok(QueryBoardData {
        reducer_name,
        collection_schema,
        users,
        query_blocks: record_map_table(query_response, "block")?,
        block_results,
        database_title,
        page_title,
        option_colors,
    })
}

fn active_reducer(
    query_response: &Value,
    active_view_kind: ViewTabKind,
) -> Result<(&'static str, &Value), String> {
    let names: &[&str] = match active_view_kind {
        ViewTabKind::Board => &["board_columns"],
        ViewTabKind::Table => &["collection_group_results"],
        ViewTabKind::List => &["collection_group_results", "list_groups"],
        ViewTabKind::Gallery => &["collection_group_results", "gallery_groups"],
        ViewTabKind::Timeline => &["timeline_results"],
        ViewTabKind::Calendar => &["calendar_results"],
        ViewTabKind::Unknown => return Err("unsupported active Notion database view".to_string()),
    };
    let results = query_response
        .get("result")
        .and_then(|result| result.get("reducerResults"))
        .and_then(Value::as_object)
        .ok_or_else(|| "missing Notion database reducer results".to_string())?;
    let mut matches = names
        .iter()
        .filter_map(|name| results.get(*name).map(|value| (*name, value)));
    let reducer = matches.next().ok_or_else(|| {
        format!(
            "missing Notion database reducer: expected {}",
            names.join(" or ")
        )
    })?;
    if matches.next().is_some() {
        return Err("ambiguous Notion database result reducers".to_string());
    }
    Ok(reducer)
}
