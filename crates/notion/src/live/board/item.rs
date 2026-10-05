use std::collections::HashMap;

use crate::model::{BoardItem, BoardItemProperty};
use serde_json::{Map, Value};

use super::load::explicit_page_shell_icon;
use super::support::{
    property::{
        extract_board_date_value, normalize_property_value,
        plain_text_from_property_value_with_page_titles, property_schema_label, PropertyLookup,
    },
    record::{page_property_rank, title_property_allow_empty},
};

pub(super) fn board_item_snapshot(
    block_id: &str,
    block: &Value,
    collection_schema: Option<&Map<String, Value>>,
    property_lookup: PropertyLookup<'_>,
) -> Result<BoardItem, String> {
    let mut page_title_cache = HashMap::new();
    let root_properties = block.get("properties").and_then(Value::as_object);
    let status = item_status(
        collection_schema,
        root_properties,
        property_lookup,
        &mut page_title_cache,
    )?;
    let mut properties = board_item_properties(
        root_properties,
        collection_schema,
        property_lookup,
        &mut page_title_cache,
    )?;
    properties.sort_by(|left, right| {
        page_property_rank(&left.label)
            .cmp(&page_property_rank(&right.label))
            .then_with(|| left.label.cmp(&right.label))
    });
    Ok(BoardItem {
        block_id: block_id.to_string(),
        title: title_property_allow_empty(block)?.unwrap_or_default(),
        status,
        icon: explicit_page_shell_icon(block, "page"),
        properties,
    })
}

fn item_status(
    collection_schema: Option<&Map<String, Value>>,
    root_properties: Option<&Map<String, Value>>,
    property_lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<String>, String> {
    let status_property_id = collection_schema.and_then(status_property_id);
    let Some(value) = status_property_id
        .and_then(|property_id| root_properties.and_then(|properties| properties.get(property_id)))
    else {
        return Ok(None);
    };
    let value =
        plain_text_from_property_value_with_page_titles(value, property_lookup, page_title_cache)?;
    Ok((!value.trim().is_empty()).then_some(value))
}

fn status_property_id(collection_schema: &Map<String, Value>) -> Option<&str> {
    collection_schema
        .iter()
        .find_map(|(property_id, property)| {
            (property.get("type").and_then(Value::as_str) == Some("status"))
                .then_some(property_id.as_str())
        })
}

fn board_item_properties(
    root_properties: Option<&Map<String, Value>>,
    collection_schema: Option<&Map<String, Value>>,
    property_lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Vec<BoardItemProperty>, String> {
    let Some(properties) = root_properties else {
        return Ok(Vec::new());
    };
    let mut items = Vec::new();
    for (property_id, value) in properties {
        if property_id == "title" {
            continue;
        }
        if let Some(property) = board_item_property(
            property_id,
            value,
            collection_schema,
            property_lookup,
            page_title_cache,
        )? {
            items.push(property);
        }
    }
    Ok(items)
}

fn board_item_property(
    property_id: &str,
    value: &Value,
    collection_schema: Option<&Map<String, Value>>,
    property_lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<BoardItemProperty>, String> {
    let schema_property = collection_schema.and_then(|schema| schema.get(property_id));
    // Pages keep values of deleted properties; Notion does not show them.
    if collection_schema.is_some() && schema_property.is_none() {
        return Ok(None);
    }
    let property_type = schema_property
        .and_then(|property| property.get("type").and_then(Value::as_str))
        .unwrap_or("unknown");
    let rendered = normalize_property_value(
        Some(property_type),
        plain_text_from_property_value_with_page_titles(value, property_lookup, page_title_cache)?,
    );
    let label = match property_lookup.blocks {
        Some(blocks) => match schema_property {
            Some(property) => property_schema_label(property, blocks, property_lookup.users)?
                .unwrap_or_else(|| property_id.to_string()),
            None => property_id.to_string(),
        },
        None => property_id.to_string(),
    };
    Ok(Some(BoardItemProperty {
        property_id: property_id.to_string(),
        label,
        property_type: property_type.to_string(),
        value: rendered,
        date: extract_board_date_value(value),
    }))
}
