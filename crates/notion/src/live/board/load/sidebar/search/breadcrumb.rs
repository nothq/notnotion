use std::collections::HashSet;

use serde_json::{Map, Value};

use super::super::super::super::{
    collection_name, record_value_state, title_property, LiveWorkspaceSearchContext,
    RecordValueState,
};
use super::result::{record_entry_by_id, search_collection_state};

struct SearchBreadcrumbParent {
    title: Option<String>,
    next: Option<(String, String)>,
}

pub(in super::super) fn resolve_search_breadcrumb(
    record_breadcrumb: Option<String>,
    search_context: &LiveWorkspaceSearchContext,
    block_id: &str,
) -> Option<String> {
    record_breadcrumb.or_else(|| {
        search_context
            .sidebar_breadcrumbs_by_block_id
            .get(block_id)
            .cloned()
    })
}

pub(in super::super) fn search_result_breadcrumb(
    block: &Value,
    blocks: &Map<String, Value>,
    collections: Option<&Map<String, Value>>,
    teams: Option<&Map<String, Value>>,
) -> Result<Option<String>, String> {
    let Some((mut parent_id, mut parent_table)) = search_parent_pointer(block) else {
        return Ok(None);
    };
    let mut nearest_first = Vec::new();
    let mut visited_parents = HashSet::new();
    loop {
        if !visited_parents.insert((parent_table.clone(), parent_id.clone())) {
            return Err(format!(
                "cyclic Notion search result parent chain at {parent_table} {parent_id}"
            ));
        }
        let Some(parent) =
            search_breadcrumb_parent(&parent_table, &parent_id, blocks, collections, teams)?
        else {
            break;
        };
        if let Some(title) = parent.title {
            nearest_first.push(title);
        }
        let Some((next_parent_id, next_parent_table)) = parent.next else {
            break;
        };
        parent_id = next_parent_id;
        parent_table = next_parent_table;
    }
    nearest_first.reverse();
    Ok(collapsed_search_breadcrumb(nearest_first))
}

fn search_breadcrumb_parent(
    table: &str,
    id: &str,
    blocks: &Map<String, Value>,
    collections: Option<&Map<String, Value>>,
    teams: Option<&Map<String, Value>>,
) -> Result<Option<SearchBreadcrumbParent>, String> {
    match table {
        "block" => search_block_breadcrumb_parent(id, blocks, collections),
        "collection" => search_collection_breadcrumb_parent(id, collections),
        "team" => Ok(search_team_breadcrumb_parent(id, teams)),
        _ => Ok(None),
    }
}

fn search_block_breadcrumb_parent(
    id: &str,
    blocks: &Map<String, Value>,
    collections: Option<&Map<String, Value>>,
) -> Result<Option<SearchBreadcrumbParent>, String> {
    let RecordValueState::Present(block) = record_value_state(record_entry_by_id(blocks, id))
    else {
        return Ok(None);
    };
    let title = if matches!(
        block.get("type").and_then(Value::as_str),
        Some("collection_view" | "collection_view_page")
    ) {
        None
    } else {
        search_breadcrumb_block_title(block, collections)?
    };
    Ok(Some(SearchBreadcrumbParent {
        title,
        next: search_parent_pointer(block),
    }))
}

fn search_collection_breadcrumb_parent(
    id: &str,
    collections: Option<&Map<String, Value>>,
) -> Result<Option<SearchBreadcrumbParent>, String> {
    let RecordValueState::Present(collection) = search_collection_state(collections, id) else {
        return Ok(None);
    };
    Ok(Some(SearchBreadcrumbParent {
        title: Some(collection_name(collection)?.unwrap_or_else(|| "New database".to_string())),
        next: search_parent_pointer(collection),
    }))
}

fn search_team_breadcrumb_parent(
    id: &str,
    teams: Option<&Map<String, Value>>,
) -> Option<SearchBreadcrumbParent> {
    let RecordValueState::Present(team) =
        record_value_state(teams.and_then(|teams| record_entry_by_id(teams, id)))
    else {
        return None;
    };
    let title = team
        .get("name")
        .or_else(|| team.get("title"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string);
    Some(SearchBreadcrumbParent {
        title,
        next: search_parent_pointer(team),
    })
}

fn search_parent_pointer(record: &Value) -> Option<(String, String)> {
    Some((
        record.get("parent_id")?.as_str()?.to_string(),
        record.get("parent_table")?.as_str()?.to_string(),
    ))
}

fn search_breadcrumb_block_title(
    block: &Value,
    collections: Option<&Map<String, Value>>,
) -> Result<Option<String>, String> {
    if let Some(title) = title_property(block)? {
        return Ok((!title.trim().is_empty()).then_some(title));
    }
    let Some(collection_id) = block.get("collection_id").and_then(Value::as_str) else {
        return Ok(None);
    };
    optional_search_collection_title(collection_id, collections)
}

fn optional_search_collection_title(
    collection_id: &str,
    collections: Option<&Map<String, Value>>,
) -> Result<Option<String>, String> {
    let RecordValueState::Present(collection) = search_collection_state(collections, collection_id)
    else {
        return Ok(None);
    };
    Ok(Some(
        collection_name(collection)?.unwrap_or_else(|| "New database".to_string()),
    ))
}

fn collapsed_search_breadcrumb(ancestors: Vec<String>) -> Option<String> {
    match ancestors.as_slice() {
        [] => None,
        [only] => Some(only.clone()),
        [first, second] => Some(format!("{first} / {second}")),
        [first, .., last] => Some(format!("{first} / … / {last}")),
    }
}
