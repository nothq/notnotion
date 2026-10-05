use serde_json::{Map, Value};

use super::super::super::super::{
    collection_name, format_edited_date_label, loaded_record_value, normalize_uuid,
    record_value_state, title_property, BoardTarget, LiveWorkspaceSearchContext, RecordValueState,
};
use super::super::page_shell_icon;
use super::breadcrumb::{resolve_search_breadcrumb, search_result_breadcrumb};
use super::transport::SearchResultWire;
use crate::model::{PageShellEditedAt, PageShellSearchBadge, PageShellSearchResult};

#[derive(Clone, Copy)]
pub(super) struct SearchRecordMaps<'a> {
    pub(super) blocks: &'a Map<String, Value>,
    pub(super) collections: &'a Map<String, Value>,
    pub(super) teams: Option<&'a Map<String, Value>>,
    pub(super) users: Option<&'a Map<String, Value>>,
    pub(super) bots: Option<&'a Map<String, Value>>,
}

pub(super) fn shape_search_results(
    results: &[SearchResultWire],
    search_context: &LiveWorkspaceSearchContext,
    board_target: &BoardTarget,
    records: SearchRecordMaps<'_>,
) -> Result<Vec<PageShellSearchResult>, String> {
    let mut shaped = Vec::with_capacity(results.len());
    for result in results {
        if let Some(result) = shape_search_result(result, search_context, board_target, records)? {
            shaped.push(result);
        }
    }
    Ok(shaped)
}

pub(super) fn shape_search_result(
    result: &SearchResultWire,
    search_context: &LiveWorkspaceSearchContext,
    board_target: &BoardTarget,
    records: SearchRecordMaps<'_>,
) -> Result<Option<PageShellSearchResult>, String> {
    if result.space_id.as_str() != search_context.space_id.as_str() {
        return Err("Notion search returned a result from a different space".to_string());
    }
    let Some((block_id, block)) = search_result_block(result, records.blocks)? else {
        return Ok(None);
    };
    let collection_id = search_result_collection_id(result, block);
    if !search_result_has_required_collection(block, collection_id.as_deref(), records.collections)?
    {
        return Ok(None);
    }
    let record_breadcrumb = search_result_breadcrumb(
        block,
        records.blocks,
        Some(records.collections),
        records.teams,
    )?;
    let breadcrumb = resolve_search_breadcrumb(record_breadcrumb, search_context, &block_id);
    let mut shaped = shape_page_shell_search_result(
        SearchResultPage {
            block_id: &block_id,
            collection_id: collection_id.as_deref(),
            block,
        },
        board_target,
        Some(records.collections),
        breadcrumb,
    )?;
    shaped.match_snippet = result
        .highlight
        .as_ref()
        .and_then(|highlight| highlight.text.as_deref())
        .and_then(search_match_snippet);
    shaped.editor_display_name =
        search_result_attribution_display_name(block, search_context, records.users, records.bots);
    shaped.edited_at = block
        .get("last_edited_time")
        .and_then(Value::as_u64)
        .map(|unix_millis| PageShellEditedAt {
            unix_millis,
            date_label: format_edited_date_label(unix_millis, &search_context.time_zone),
        });
    shaped.edited_label = None;
    Ok(Some(shaped))
}

/// A search result's normalized block ID and its block record.
type SearchResultBlock<'a> = (String, &'a Value);

pub(super) fn search_result_block<'a>(
    result: &SearchResultWire,
    blocks: &'a Map<String, Value>,
) -> Result<Option<SearchResultBlock<'a>>, String> {
    let block_id = uuid::Uuid::parse_str(&result.id)
        .map_err(|error| format!("invalid Notion search result page ID: {error}"))?
        .to_string();
    match record_value_state(record_entry_by_id(blocks, &result.id)) {
        RecordValueState::Present(block) => Ok(Some((block_id, block))),
        RecordValueState::Unavailable => Ok(None),
        RecordValueState::Incomplete => Err(format!("missing block {}", result.id)),
    }
}

pub(super) fn search_result_collection_id(
    result: &SearchResultWire,
    block: &Value,
) -> Option<String> {
    result
        .collection_id
        .as_deref()
        .or_else(|| block.get("collection_id").and_then(Value::as_str))
        .map(str::to_string)
}

pub(in super::super) fn search_result_has_required_collection(
    block: &Value,
    collection_id: Option<&str>,
    collections: &Map<String, Value>,
) -> Result<bool, String> {
    let Some(collection_id) = collection_id else {
        return Ok(true);
    };
    let requires_collection = matches!(
        block.get("type").and_then(Value::as_str),
        Some("collection_view" | "collection_view_page")
    ) || title_property(block)?.is_none();
    Ok(!requires_collection
        || matches!(
            search_collection_state(Some(collections), collection_id),
            RecordValueState::Present(_)
        ))
}

pub(in super::super) fn record_entry_by_id<'a>(
    records: &'a Map<String, Value>,
    record_id: &str,
) -> Option<&'a Value> {
    let normalized_id = normalize_uuid(record_id);
    records.get(record_id).or_else(|| {
        records
            .iter()
            .find(|(candidate_id, _)| normalize_uuid(candidate_id) == normalized_id)
            .map(|(_, entry)| entry)
    })
}

pub(super) fn search_collection_state<'a>(
    collections: Option<&'a Map<String, Value>>,
    collection_id: &str,
) -> RecordValueState<'a> {
    record_value_state(collections.and_then(|records| record_entry_by_id(records, collection_id)))
}

fn search_collection_value<'a>(
    collections: Option<&'a Map<String, Value>>,
    collection_id: &str,
) -> Result<&'a Value, String> {
    match search_collection_state(collections, collection_id) {
        RecordValueState::Present(collection) => Ok(collection),
        RecordValueState::Unavailable => Err(format!("unavailable collection {collection_id}")),
        RecordValueState::Incomplete => Err(format!("missing collection {collection_id}")),
    }
}

pub(in super::super) struct SearchResultPage<'a> {
    pub(in super::super) block_id: &'a str,
    pub(in super::super) collection_id: Option<&'a str>,
    pub(in super::super) block: &'a Value,
}

pub(in super::super) fn shape_page_shell_search_result(
    page: SearchResultPage<'_>,
    board_target: &BoardTarget,
    collections: Option<&Map<String, Value>>,
    breadcrumb: Option<String>,
) -> Result<PageShellSearchResult, String> {
    let SearchResultPage {
        block_id,
        collection_id,
        block,
    } = page;
    let title = search_result_title(block, collection_id, collections)?;
    let fallback_icon = match block.get("type").and_then(Value::as_str) {
        Some("collection_view" | "collection_view_page") => "database",
        Some("transcription") => "meetings",
        _ => "page",
    };
    Ok(PageShellSearchResult {
        block_id: block_id.to_string(),
        title,
        icon: page_shell_icon(block, fallback_icon),
        target_board_url: board_target.search_result_url(block_id),
        highlight: breadcrumb,
        match_snippet: None,
        editor_display_name: None,
        edited_label: None,
        edited_at: None,
        badges: search_result_badges(block_id, block, board_target),
    })
}

fn search_result_title(
    block: &Value,
    collection_id: Option<&str>,
    collections: Option<&Map<String, Value>>,
) -> Result<String, String> {
    let collection_id =
        collection_id.or_else(|| block.get("collection_id").and_then(Value::as_str));
    if matches!(
        block.get("type").and_then(Value::as_str),
        Some("collection_view" | "collection_view_page")
    ) {
        if let Some(collection_id) = collection_id {
            let collection = search_collection_value(collections, collection_id)?;
            return Ok(collection_name(collection)?.unwrap_or_else(|| "New database".to_string()));
        }
    }
    if let Some(title) = title_property(block)? {
        return Ok(title);
    }
    let Some(collection_id) = collection_id else {
        return Ok("Untitled".to_string());
    };
    let collection = search_collection_value(collections, collection_id)?;
    Ok(collection_name(collection)?.unwrap_or_else(|| "New database".to_string()))
}

fn search_result_badges(
    block_id: &str,
    block: &Value,
    board_target: &BoardTarget,
) -> Vec<PageShellSearchBadge> {
    let mut badges = Vec::with_capacity(2);
    if normalize_uuid(block_id) == board_target.collection_view_block_id {
        badges.push(PageShellSearchBadge::CurrentPage);
    }
    if matches!(
        block.get("type").and_then(Value::as_str),
        Some("collection_view" | "collection_view_page")
    ) {
        badges.push(PageShellSearchBadge::Database);
    }
    badges
}

pub(super) fn search_result_attribution_pointer(block: &Value) -> Option<(&str, &str)> {
    ["created_by", "last_edited_by"]
        .into_iter()
        .find_map(|prefix| {
            let table = block
                .get(format!("{prefix}_table"))
                .and_then(Value::as_str)?;
            let id = block.get(format!("{prefix}_id")).and_then(Value::as_str)?;
            Some((table, id))
        })
}

pub(in super::super) fn search_result_attribution_display_name(
    block: &Value,
    search_context: &LiveWorkspaceSearchContext,
    users: Option<&Map<String, Value>>,
    bots: Option<&Map<String, Value>>,
) -> Option<String> {
    let (attribution_table, attribution_id) = search_result_attribution_pointer(block)?;
    let attribution_records = match attribution_table {
        "notion_user" => users,
        "bot" => bots,
        _ => return None,
    };
    if let Some(name) = attribution_records
        .and_then(|users| {
            users.get(attribution_id).or_else(|| {
                users
                    .iter()
                    .find(|(user_id, _)| normalize_uuid(user_id) == normalize_uuid(attribution_id))
                    .map(|(_, user)| user)
            })
        })
        .and_then(loaded_record_value)
        .and_then(|user| user.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        return Some(name.to_string());
    }
    if attribution_table != "notion_user" {
        return None;
    }
    let active_user_id = normalize_uuid(&search_context.active_user_id);
    if normalize_uuid(attribution_id) != active_user_id
        || normalize_uuid(&search_context.active_user_profile.user_id) != active_user_id
    {
        return None;
    }
    let name = search_context.active_user_profile.name.trim();
    (!name.is_empty()).then(|| name.to_string())
}

pub(super) fn search_match_snippet(marked_text: &str) -> Option<String> {
    let without_markers = marked_text
        .replace("<gzkNfoUU>", "")
        .replace("</gzkNfoUU>", "");
    let mut plain = String::with_capacity(without_markers.len());
    let mut line_break_pending = false;
    for character in without_markers.chars() {
        if matches!(character, '\n' | '\r' | '\t') {
            line_break_pending = true;
            continue;
        }
        if line_break_pending {
            if !character.is_whitespace() && !plain.chars().last().is_some_and(char::is_whitespace)
            {
                plain.push(' ');
            }
            line_break_pending = false;
        }
        plain.push(character);
    }
    let plain = plain.trim();
    (!plain.is_empty()).then(|| plain.to_string())
}
