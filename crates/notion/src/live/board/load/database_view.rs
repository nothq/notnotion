use crate::live::board::support::view::database_view_query;

use super::{
    block_value, board_active_view_group, board_active_view_property_layout,
    board_active_view_sorts, board_calendar_view, board_columns, board_database_properties,
    board_group_by, board_item_snapshot, board_table_view_columns, board_timeline_view,
    collection_view_collection_id, collection_view_entry, format_edited_label, query_block_ids,
    record_map_table, required_array, required_string, required_u64, resolve_active_view_id,
    view_tab_kind, BoardColumn, BoardItem, BoardTarget, BootstrapBoardData, CalendarViewConfig,
    DatabaseViewControlContext, DatabaseViewFilterState, HashMap, HashSet, LoadedBoardParts, Map,
    NotionCollectionViewId, PropertyLookup, QueryBoardData, TableViewColumn, TimelineViewConfig,
    UserContext, Value, ViewTab, ViewTabKind,
};

pub(super) struct DatabaseDateViews {
    pub(super) timeline_view: Option<TimelineViewConfig>,
    pub(super) calendar_view: Option<CalendarViewConfig>,
}

pub(super) fn load_database_parts(
    query_response: &Value,
    bootstrap: &BootstrapBoardData<'_>,
    query_data: &QueryBoardData<'_>,
    active_filter_state: &DatabaseViewFilterState,
    date_views: DatabaseDateViews,
) -> Result<LoadedBoardParts, String> {
    let DatabaseDateViews {
        timeline_view,
        calendar_view,
    } = date_views;
    let mut ordered_item_ids = Vec::new();
    let view_tabs = board_view_tabs(
        &bootstrap.view_ids,
        bootstrap.collection_views,
        &bootstrap.active_view_id,
        active_filter_state,
    )?;
    let columns =
        load_database_columns(query_response, bootstrap, query_data, &mut ordered_item_ids)?;
    let table_view_columns = board_table_columns(bootstrap, query_data)?;
    let active_view_property_layout = board_property_layout(bootstrap, query_data)?;
    let view_control_context =
        board_view_control_context(bootstrap, query_data, active_view_property_layout.clone())?;
    let database_properties = board_database_properties(
        query_data.collection_schema,
        bootstrap.blocks,
        query_data.users,
    )?;
    let items = board_items(&ordered_item_ids, query_data)?;
    Ok(LoadedBoardParts {
        columns,
        view_tabs,
        items,
        table_view_columns,
        active_view_property_layout,
        view_control_context,
        database_properties,
        timeline_view,
        calendar_view,
    })
}

fn board_view_control_context(
    bootstrap_data: &BootstrapBoardData<'_>,
    query_data: &QueryBoardData<'_>,
    property_layout: crate::model::DatabaseViewPropertyLayout,
) -> Result<DatabaseViewControlContext, String> {
    let view = collection_view_entry(
        bootstrap_data.collection_views,
        &bootstrap_data.active_view_id,
    )?;
    let query2 = database_view_query(view)?.cloned().unwrap_or_default();
    let format = view
        .get("format")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| "Notion active database view format must be an object".to_string())?;
    let schema = query_data
        .collection_schema
        .ok_or_else(|| "missing Notion collection property schema".to_string())?;
    let property_types = schema
        .iter()
        .map(|(property_id, property)| {
            Ok((
                property_id.clone(),
                required_string(property, "type")?.to_string(),
            ))
        })
        .collect::<Result<HashMap<_, _>, String>>()?;
    Ok(DatabaseViewControlContext {
        view_kind: bootstrap_data.active_view_kind,
        query2,
        format,
        sorts: board_active_view_sorts(view, query_data.collection_schema)?,
        group: board_active_view_group(
            view,
            bootstrap_data.active_view_kind,
            query_data.collection_schema,
        )?,
        property_layout,
        property_types,
    })
}

fn board_property_layout(
    bootstrap_data: &BootstrapBoardData<'_>,
    query_data: &QueryBoardData<'_>,
) -> Result<crate::model::DatabaseViewPropertyLayout, String> {
    let view = collection_view_entry(
        bootstrap_data.collection_views,
        &bootstrap_data.active_view_id,
    )?;
    board_active_view_property_layout(
        view,
        bootstrap_data.active_view_kind,
        query_data.collection_schema,
    )
}

fn load_database_columns(
    query_response: &Value,
    bootstrap: &BootstrapBoardData<'_>,
    query_data: &QueryBoardData<'_>,
    ordered_item_ids: &mut Vec<String>,
) -> Result<Vec<BoardColumn>, String> {
    if bootstrap.active_view_kind != ViewTabKind::Board {
        *ordered_item_ids = query_block_ids(query_data)?;
        return Ok(Vec::new());
    }
    let mut seen_item_ids = HashSet::new();
    let group_property_id = bootstrap
        .group_by
        .as_ref()
        .and_then(|group_by| group_by.get("property"))
        .and_then(Value::as_str);
    board_columns(
        query_response,
        query_data,
        group_property_id,
        ordered_item_ids,
        &mut seen_item_ids,
    )
}

pub(super) fn bootstrap_board_data<'a>(
    bootstrap: &'a Value,
    board_target: &BoardTarget,
    user_context: &UserContext,
) -> Result<BootstrapBoardData<'a>, String> {
    let blocks = record_map_table(bootstrap, "block")?;
    let collection_views = record_map_table(bootstrap, "collection_view")?;
    let collection_view_block =
        block_value(blocks, &board_target.collection_view_block_id)?.clone();
    let space_id = required_string(&collection_view_block, "space_id")?.to_string();
    let favorite_space_view_id = user_context
        .space_view_id_for_space(&space_id)
        .map(str::to_string);
    let edited_label = format_edited_label(
        required_u64(&collection_view_block, "last_edited_time")?,
        &user_context.time_zone,
    );
    let view_ids = required_array(&collection_view_block, "view_ids")?.clone();
    let active_view_id = resolve_active_view_id(
        &collection_view_block,
        collection_views,
        board_target.collection_view_id.as_deref(),
    )?;
    let collection_id = collection_view_collection_id(
        &collection_view_block,
        Some(collection_views),
        Some(&active_view_id),
    )?
    .to_string();
    let active_view = collection_view_entry(collection_views, &active_view_id)?;
    let active_view_kind = view_tab_kind(required_string(active_view, "type").ok());
    let group_by = (active_view_kind == ViewTabKind::Board)
        .then(|| board_group_by(active_view))
        .transpose()?;
    Ok(BootstrapBoardData {
        blocks,
        collection_views,
        collection_view_block,
        collection_id,
        space_id,
        favorite_space_view_id,
        edited_label,
        view_ids,
        active_view_id,
        active_view_kind,
        group_by,
    })
}

fn board_view_tabs(
    view_ids: &[Value],
    collection_views: &Map<String, Value>,
    active_view_id: &str,
    active_filter_state: &crate::model::DatabaseViewFilterState,
) -> Result<Vec<ViewTab>, String> {
    view_ids
        .iter()
        .map(|view_id| {
            let view_id = view_id
                .as_str()
                .ok_or_else(|| "Notion collection view ID must be a string".to_string())?;
            let provider_view_id = NotionCollectionViewId::try_from(view_id.to_string())?;
            let view = collection_view_entry(collection_views, provider_view_id.as_str())?;
            let view_type = required_string(view, "type")?;
            let active = provider_view_id.as_str() == active_view_id;
            Ok(ViewTab {
                provider_view_id,
                label: collection_view_label(view, view_type)?,
                kind: view_tab_kind(Some(view_type)),
                active,
                filters: active.then(|| active_filter_state.clone()),
            })
        })
        .collect()
}

fn collection_view_label(view: &Value, view_type: &str) -> Result<String, String> {
    match view.get("name") {
        Some(Value::String(label)) if !label.is_empty() => return Ok(label.clone()),
        Some(Value::String(_)) | None => {}
        Some(_) => return Err("Notion collection view name must be a string".to_string()),
    }
    let default_label = match view_type {
        "board" => "Board",
        "calendar" => "Calendar",
        "chart" => "Chart",
        "feed" => "Feed",
        "form" => "Form",
        "gallery" => "Gallery",
        "list" => "List",
        "map" => "Map",
        "table" => "Table",
        "timeline" => "Timeline",
        unsupported => {
            return Err(format!(
                "unnamed Notion collection view has unsupported type {unsupported}"
            ));
        }
    };
    Ok(default_label.to_string())
}

fn board_table_columns(
    bootstrap_data: &BootstrapBoardData<'_>,
    query_data: &QueryBoardData<'_>,
) -> Result<Vec<TableViewColumn>, String> {
    if bootstrap_data.active_view_kind != ViewTabKind::Table {
        return Ok(Vec::new());
    }
    let view = collection_view_entry(
        bootstrap_data.collection_views,
        &bootstrap_data.active_view_id,
    )?;
    board_table_view_columns(
        view,
        query_data.collection_schema,
        bootstrap_data.blocks,
        query_data.users,
    )
}

pub(super) fn board_timeline_config(
    bootstrap_data: &BootstrapBoardData<'_>,
) -> Result<Option<TimelineViewConfig>, String> {
    if bootstrap_data.active_view_kind != ViewTabKind::Timeline {
        return Ok(None);
    }
    let view = collection_view_entry(
        bootstrap_data.collection_views,
        &bootstrap_data.active_view_id,
    )?;
    let timeline_view = board_timeline_view(view).ok_or_else(|| {
        format!(
            "missing timeline configuration for active Notion collection view {}",
            bootstrap_data.active_view_id
        )
    })?;
    Ok(Some(timeline_view))
}

pub(super) fn board_calendar_config(
    bootstrap_data: &BootstrapBoardData<'_>,
) -> Result<Option<CalendarViewConfig>, String> {
    if bootstrap_data.active_view_kind != ViewTabKind::Calendar {
        return Ok(None);
    }
    let view = collection_view_entry(
        bootstrap_data.collection_views,
        &bootstrap_data.active_view_id,
    )?;
    Ok(Some(board_calendar_view(view)?))
}

fn board_items(
    ordered_item_ids: &[String],
    query_data: &QueryBoardData<'_>,
) -> Result<Vec<BoardItem>, String> {
    let property_lookup =
        PropertyLookup::preloaded(Some(query_data.query_blocks), query_data.users);
    let mut items = Vec::new();
    for block_id in ordered_item_ids {
        let Ok(block) = block_value(query_data.query_blocks, block_id) else {
            continue;
        };
        items.push(board_item_snapshot(
            block_id,
            block,
            query_data.collection_schema,
            property_lookup,
        )?);
    }
    Ok(items)
}
