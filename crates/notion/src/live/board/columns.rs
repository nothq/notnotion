use std::collections::HashSet;

use crate::model::{BoardColumn, CardSummary};
use serde_json::{Map, Value};

use super::{
    load::{explicit_page_shell_icon, QueryBoardData},
    support::{
        record::{block_value, loaded_record_value, title_property_allow_empty},
        view::card_height_for_title,
    },
};

/// One group of Notion's `board_columns` reducer: the key of its
/// `blockResults` entry, its display title, and its option for option groups.
struct BoardGroup {
    key: String,
    title: String,
    option: Option<String>,
}

pub(super) fn board_columns(
    query_response: &Value,
    query_data: &QueryBoardData<'_>,
    group_property_id: Option<&str>,
    ordered_item_ids: &mut Vec<String>,
    seen_item_ids: &mut HashSet<String>,
) -> Result<Vec<BoardColumn>, String> {
    let group_property = group_property_id.and_then(|property_id| {
        query_data
            .collection_schema
            .and_then(|schema| schema.get(property_id))
    });
    let mut columns = Vec::new();
    let results = query_response
        .get("result")
        .and_then(|result| result.get("reducerResults"))
        .and_then(|results| results.get("board_columns"))
        .and_then(|reducer| reducer.get("results"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for result in results {
        // Notion hides groups the view marks invisible.
        if result.get("visible").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let Some(group) = result
            .get("value")
            .and_then(|value| board_group(value, query_data, group_property))
        else {
            continue;
        };
        let Some(cards) =
            board_column_cards(&group.key, query_data, ordered_item_ids, seen_item_ids)?
        else {
            continue;
        };
        columns.push(BoardColumn {
            option_color: group
                .option
                .and_then(|option| query_data.option_colors.get(&option).cloned()),
            title: group.title,
            cards,
        });
    }
    Ok(columns)
}

fn board_group(
    value: &Value,
    query_data: &QueryBoardData<'_>,
    group_property: Option<&Value>,
) -> Option<BoardGroup> {
    let group_type = value.get("type")?.as_str()?;
    let (key, title, option) = match value.get("value") {
        None | Some(Value::Null) => {
            let name = group_property
                .and_then(|property| property.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("value");
            ("uncategorized".to_string(), format!("No {name}"), None)
        }
        Some(Value::String(option)) => (option.clone(), option.clone(), Some(option.clone())),
        Some(Value::Bool(checked)) => {
            let title = if *checked { "Checked" } else { "Unchecked" };
            (checked.to_string(), title.to_string(), None)
        }
        Some(Value::Number(number)) => (number.to_string(), number.to_string(), None),
        Some(Value::Object(group)) => object_board_group(group, query_data, group_property)?,
        Some(Value::Array(_)) => return None,
    };
    Some(BoardGroup {
        key: format!("{group_type}:{key}"),
        title,
        option,
    })
}

/// A group's unprefixed key, its title, and its select option.
type BoardGroupParts = (String, String, Option<String>);

fn object_board_group(
    group: &Map<String, Value>,
    query_data: &QueryBoardData<'_>,
    group_property: Option<&Value>,
) -> Option<BoardGroupParts> {
    if let Some(option) = group.get("option").and_then(Value::as_str) {
        let kind = group.get("type")?.as_str()?;
        return Some((
            format!("{kind}-{option}"),
            option.to_string(),
            Some(option.to_string()),
        ));
    }
    if let Some(group_id) = group.get("group").and_then(Value::as_str) {
        let kind = group.get("type")?.as_str()?;
        let title = group_property
            .and_then(|property| property.get("groups"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(group_id))
            .and_then(|candidate| candidate.get("name").and_then(Value::as_str))
            .unwrap_or(group_id);
        return Some((format!("{kind}-{group_id}"), title.to_string(), None));
    }
    let id = group.get("id")?.as_str()?;
    let title = match group.get("table").and_then(Value::as_str) {
        Some("notion_user") => query_data
            .users
            .and_then(|users| users.get(id))
            .and_then(loaded_record_value)
            .and_then(|user| user.get("name").and_then(Value::as_str))
            .unwrap_or(id)
            .to_string(),
        Some("block") => block_value(query_data.query_blocks, id)
            .ok()
            .and_then(|block| title_property_allow_empty(block).ok().flatten())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "Untitled".to_string()),
        _ => id.to_string(),
    };
    Some((id.to_string(), title, None))
}

fn board_column_cards(
    block_result_key: &str,
    query_data: &QueryBoardData<'_>,
    ordered_item_ids: &mut Vec<String>,
    seen_item_ids: &mut HashSet<String>,
) -> Result<Option<Vec<CardSummary>>, String> {
    let Some(card_ids) = query_data
        .block_results
        .get(block_result_key)
        .and_then(|result| result.get("blockIds"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    let mut cards = Vec::new();
    for block_id in card_ids.iter().filter_map(Value::as_str) {
        if let Some(card) = board_column_card(
            block_id,
            query_data.query_blocks,
            ordered_item_ids,
            seen_item_ids,
        )? {
            cards.push(card);
        }
    }
    Ok(Some(cards))
}

fn board_column_card(
    block_id: &str,
    query_blocks: &Map<String, Value>,
    ordered_item_ids: &mut Vec<String>,
    seen_item_ids: &mut HashSet<String>,
) -> Result<Option<CardSummary>, String> {
    let Ok(card_block) = block_value(query_blocks, block_id) else {
        return Ok(None);
    };
    let Some(title) = title_property_allow_empty(card_block)? else {
        return Ok(None);
    };
    if seen_item_ids.insert(block_id.to_string()) {
        ordered_item_ids.push(block_id.to_string());
    }
    Ok(Some(CardSummary {
        block_id: block_id.to_string(),
        height: card_height_for_title(&title),
        title,
        has_content: card_block
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|content| !content.is_empty()),
        icon: explicit_page_shell_icon(card_block, "page"),
    }))
}
