use std::collections::HashMap;

use crate::model::{
    DatabaseViewGroup, DatabaseViewGroupKind, DatabaseViewGroupState, DatabaseViewPropertyLayout,
    DatabaseViewPropertyVisibility, DatabaseViewSort, DatabaseViewSortDirection,
    DatabaseViewSortState, NotionDatabasePropertyId, TableViewColumn, ViewTabKind,
};
use serde_json::{Map, Value};

use super::super::{
    property::property_schema_label,
    record::{collection_view_entry, required_array, required_string},
};
use super::{database_view_query, view_tab_kind};

mod layout;
mod properties;

use layout::{property_is_in_schema, view_layout_entries};
pub(crate) use properties::{
    board_database_properties, database_property_filter_type, database_property_options,
};

pub(crate) fn status_option_colors(
    collection: &Value,
    status_property_id: Option<&str>,
) -> Result<HashMap<String, String>, String> {
    let Some(status_property_id) = status_property_id else {
        return Ok(HashMap::new());
    };
    let schema = collection
        .get("schema")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing collection schema".to_string())?;
    // Boards also group by properties without options, such as people.
    let Some(options) = schema
        .get(status_property_id)
        .and_then(|property| property.get("options"))
        .and_then(Value::as_array)
    else {
        return Ok(HashMap::new());
    };

    Ok(options
        .iter()
        .filter_map(|option| {
            Some((
                option.get("value")?.as_str()?.to_string(),
                option.get("color")?.as_str()?.to_string(),
            ))
        })
        .collect())
}

pub(crate) fn board_group_by(view: &Value) -> Result<Value, String> {
    if let Some(group_by) = view
        .get("format")
        .and_then(|format| format.get("board_columns_by"))
    {
        return Ok(group_by.clone());
    }
    // Boards created before `board_columns_by` group through `query2.group_by`.
    let property_id = view
        .get("query2")
        .and_then(|query| query.get("group_by"))
        .and_then(Value::as_str)
        .ok_or_else(|| "missing board group-by configuration".to_string())?;
    Ok(serde_json::json!({ "property": property_id }))
}

pub(crate) fn resolve_active_view_id(
    collection_view_block: &Value,
    collection_views: &Map<String, Value>,
    preferred_view_id: Option<&str>,
) -> Result<String, String> {
    if let Some(preferred_view_id) = preferred_view_id {
        let mut belongs_to_collection = false;
        for (index, view_id) in required_array(collection_view_block, "view_ids")?
            .iter()
            .enumerate()
        {
            let view_id = view_id
                .as_str()
                .ok_or_else(|| format!("invalid Notion collection view ID at view_ids[{index}]"))?;
            belongs_to_collection |= view_id == preferred_view_id;
        }
        if !belongs_to_collection {
            return Err(format!(
                "Notion collection does not contain requested view {preferred_view_id}"
            ));
        }
        let preferred_view = collection_view_entry(collection_views, preferred_view_id)?;
        let preferred_view_type = required_string(preferred_view, "type")?;
        if view_tab_kind(Some(preferred_view_type)) == ViewTabKind::Unknown {
            return Err(format!(
                "unsupported preferred Notion database view type {preferred_view_type}"
            ));
        }
        return Ok(preferred_view_id.to_string());
    }

    for (index, view_id) in required_array(collection_view_block, "view_ids")?
        .iter()
        .enumerate()
    {
        let view_id = view_id
            .as_str()
            .ok_or_else(|| format!("invalid Notion collection view ID at view_ids[{index}]"))?;
        let view = collection_view_entry(collection_views, view_id)?;
        let view_type = required_string(view, "type")?;
        if view_tab_kind(Some(view_type)) != ViewTabKind::Unknown {
            return Ok(view_id.to_string());
        }
    }
    Err("missing supported Notion database view".to_string())
}

pub(crate) fn board_table_view_columns(
    view: &Value,
    collection_schema: Option<&Map<String, Value>>,
    blocks: &Map<String, Value>,
    users: Option<&Map<String, Value>>,
) -> Result<Vec<TableViewColumn>, String> {
    let properties = view_layout_entries(view, ViewTabKind::Table, collection_schema)?;
    let mut columns = Vec::new();
    for property in &properties {
        if property.get("visible").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let Some(property_id) = property.get("property").and_then(Value::as_str) else {
            continue;
        };
        let label = match collection_schema.and_then(|schema| schema.get(property_id)) {
            Some(schema_property) => property_schema_label(schema_property, blocks, users)?
                .unwrap_or_else(|| property_id.to_string()),
            None => property_id.to_string(),
        };
        let width = property
            .get("width")
            .and_then(Value::as_f64)
            .unwrap_or_else(|| if property_id == "title" { 280.0 } else { 200.0 })
            as f32;
        columns.push(TableViewColumn {
            property_id: property_id.to_string(),
            label,
            width,
            wrap: property
                .get("wrap")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        });
    }
    Ok(columns)
}

pub(crate) fn board_active_view_property_layout(
    view: &Value,
    view_kind: ViewTabKind,
    collection_schema: Option<&Map<String, Value>>,
) -> Result<DatabaseViewPropertyLayout, String> {
    let format_key = match view_kind {
        ViewTabKind::Table => "table_properties",
        ViewTabKind::List => "list_properties",
        ViewTabKind::Gallery => "gallery_properties",
        ViewTabKind::Board
        | ViewTabKind::Timeline
        | ViewTabKind::Calendar
        | ViewTabKind::Unknown => return Ok(DatabaseViewPropertyLayout::default()),
    };
    let schema =
        collection_schema.ok_or_else(|| "missing Notion collection property schema".to_string())?;
    let entries = view_layout_entries(view, view_kind, Some(schema))?
        .iter()
        .enumerate()
        .map(|(index, property)| {
            let property_id = required_string(property, "property")?
                .parse::<NotionDatabasePropertyId>()
                .map_err(str::to_string)?;
            let visible = match property.get("visible") {
                Some(Value::Bool(visible)) => *visible,
                None if view_kind == ViewTabKind::Table => true,
                None => {
                    return Err(format!(
                        "Notion view format.{format_key}[{index}].visible is required"
                    ));
                }
                Some(_) => {
                    return Err(format!(
                        "Notion view format.{format_key}[{index}].visible must be a boolean"
                    ));
                }
            };
            Ok(DatabaseViewPropertyVisibility::new(property_id, visible))
        })
        .collect::<Result<Vec<_>, String>>()?;
    DatabaseViewPropertyLayout::try_from(entries)
}

pub(crate) fn board_active_view_sorts(
    view: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> Result<DatabaseViewSortState, String> {
    let query = database_view_query(view)?;
    let schema =
        collection_schema.ok_or_else(|| "missing Notion collection property schema".to_string())?;
    let sorts = match query.and_then(|query| query.get("sort")) {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(sorts)) => sorts
            .iter()
            .filter(|sort| property_is_in_schema(sort, schema))
            .map(|sort| {
                let property_id = required_string(sort, "property")?
                    .parse::<NotionDatabasePropertyId>()
                    .map_err(str::to_string)?;
                let direction = required_string(sort, "direction")?
                    .parse::<DatabaseViewSortDirection>()
                    .map_err(str::to_string)?;
                Ok(DatabaseViewSort::new(property_id, direction))
            })
            .collect::<Result<Vec<_>, String>>()?,
        Some(_) => return Err("Notion active database view query2.sort must be an array".into()),
    };
    DatabaseViewSortState::try_from(sorts)
}

pub(crate) fn board_active_view_group(
    view: &Value,
    view_kind: ViewTabKind,
    collection_schema: Option<&Map<String, Value>>,
) -> Result<DatabaseViewGroupState, String> {
    if !matches!(
        view_kind,
        ViewTabKind::Table | ViewTabKind::List | ViewTabKind::Gallery
    ) {
        return Ok(DatabaseViewGroupState::None);
    }
    let format = view
        .get("format")
        .and_then(Value::as_object)
        .ok_or_else(|| "Notion active database view format must be an object".to_string())?;
    let Some(raw_group) = format.get("collection_group_by") else {
        return Ok(DatabaseViewGroupState::None);
    };
    if raw_group.is_null() {
        return Ok(DatabaseViewGroupState::None);
    }
    let group = raw_group
        .as_object()
        .ok_or_else(|| "Notion format.collection_group_by must be an object or null".to_string())?;
    let property_id = group
        .get("property")
        .and_then(Value::as_str)
        .map(str::to_string);
    let property_type = group
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_string);
    let (Some(property_id), Some(property_type)) = (&property_id, &property_type) else {
        return Ok(DatabaseViewGroupState::Unsupported {
            property_id,
            property_type,
        });
    };
    supported_group_state(property_id, property_type, collection_schema)
}

fn supported_group_state(
    property_id: &str,
    property_type: &str,
    collection_schema: Option<&Map<String, Value>>,
) -> Result<DatabaseViewGroupState, String> {
    let schema =
        collection_schema.ok_or_else(|| "missing Notion collection property schema".to_string())?;
    let Some(schema_property) = schema.get(property_id) else {
        return Err(format!(
            "Notion format.collection_group_by references unknown property {property_id}"
        ));
    };
    let schema_type = required_string(schema_property, "type")?;
    let Ok(kind) = DatabaseViewGroupKind::parse_schema_type(schema_type) else {
        return Ok(DatabaseViewGroupState::Unsupported {
            property_id: Some(property_id.to_string()),
            property_type: Some(property_type.to_string()),
        });
    };
    if property_type != schema_type {
        return Err(format!(
            "Notion grouping type {property_type} does not match schema type {schema_type} for property {property_id}"
        ));
    }
    Ok(DatabaseViewGroupState::Supported {
        group: DatabaseViewGroup::new(property_id.parse().map_err(str::to_string)?, kind),
    })
}
