use std::collections::BTreeSet;

use serde_json::json;

use super::HydratedPageBlockScope;
use crate::live::board::{
    loaded_record_value, optional_record_map_table, record_map::merge_record_map, record_map_table,
    required_array, required_string, Map, NotionPrivateApiEndpoint, Value,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

pub(super) fn hydrate_page_comments(
    session: &NotionDesktopSession,
    scope: &HydratedPageBlockScope,
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    let discussion_ids = page_discussion_ids(scope, response)?;
    hydrate_space_records(
        session,
        &scope.space_id,
        "discussion",
        &discussion_ids,
        response,
    )?;

    let comment_ids = page_comment_ids(&discussion_ids, &scope.space_id, response)?;
    hydrate_space_records(session, &scope.space_id, "comment", &comment_ids, response)?;

    let user_ids = page_comment_user_ids(&comment_ids, &scope.space_id, response)?;
    hydrate_comment_users(session, &scope.space_id, &user_ids, response)
}

fn page_discussion_ids(
    scope: &HydratedPageBlockScope,
    response: &Value,
) -> Result<Vec<String>, String> {
    let blocks = record_map_table(response, "block")?;
    let mut discussion_ids = BTreeSet::new();
    for block_id in &scope.block_ids {
        let block = blocks
            .get(block_id)
            .and_then(loaded_record_value)
            .ok_or_else(|| {
                format!("missing hydrated Notion page block {block_id} while loading comments")
            })?;
        if required_string(block, "space_id")? != scope.space_id {
            return Err(format!(
                "Notion comment target block {block_id} belongs to a different space"
            ));
        }
        let Some(raw_discussions) = block.get("discussions") else {
            continue;
        };
        let discussions = raw_discussions.as_array().ok_or_else(|| {
            format!("Notion block {block_id} contains a non-array discussions field")
        })?;
        for discussion_id in discussions {
            let discussion_id = discussion_id
                .as_str()
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| {
                    format!("Notion block {block_id} contains an invalid discussion ID")
                })?;
            discussion_ids.insert(discussion_id.to_string());
        }
    }
    Ok(discussion_ids.into_iter().collect())
}

fn page_comment_ids(
    discussion_ids: &[String],
    space_id: &str,
    response: &Value,
) -> Result<Vec<String>, String> {
    let discussions = table_for_references(response, "discussion", discussion_ids)?;
    let mut comment_ids = BTreeSet::new();
    for discussion_id in discussion_ids {
        let discussion = referenced_value(discussions, "discussion", discussion_id)?;
        validate_record_space(discussion, "discussion", discussion_id, space_id)?;
        let comments = required_array(discussion, "comments").map_err(|error| {
            format!("Notion discussion {discussion_id} has invalid comments: {error}")
        })?;
        for comment_id in comments {
            let comment_id = comment_id
                .as_str()
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| {
                    format!("Notion discussion {discussion_id} contains an invalid comment ID")
                })?;
            comment_ids.insert(comment_id.to_string());
        }
    }
    Ok(comment_ids.into_iter().collect())
}

fn page_comment_user_ids(
    comment_ids: &[String],
    space_id: &str,
    response: &Value,
) -> Result<Vec<String>, String> {
    let comments = table_for_references(response, "comment", comment_ids)?;
    let mut user_ids = BTreeSet::new();
    for comment_id in comment_ids {
        let comment = referenced_value(comments, "comment", comment_id)?;
        validate_record_space(comment, "comment", comment_id, space_id)?;
        let alive = comment
            .get("alive")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("Notion comment {comment_id} is missing boolean alive"))?;
        if !alive {
            continue;
        }
        if comment.get("created_by_table").and_then(Value::as_str) != Some("notion_user") {
            return Err(format!(
                "Notion comment {comment_id} has an unsupported author table"
            ));
        }
        user_ids.insert(required_string(comment, "created_by_id")?.to_string());
        if let Some(text) = comment.get("text") {
            collect_mention_user_ids(text, &mut user_ids);
        }
    }
    Ok(user_ids.into_iter().collect())
}

fn collect_mention_user_ids(text: &Value, user_ids: &mut BTreeSet<String>) {
    let Some(segments) = text.as_array() else {
        return;
    };
    for segment in segments {
        let Some(segment) = segment.as_array() else {
            continue;
        };
        if segment.first().and_then(Value::as_str) != Some("‣") {
            continue;
        }
        let Some(annotations) = segment.get(1).and_then(Value::as_array) else {
            continue;
        };
        for annotation in annotations {
            let Some(annotation) = annotation.as_array() else {
                continue;
            };
            if annotation.first().and_then(Value::as_str) != Some("u") {
                continue;
            }
            if let Some(user_id) = annotation
                .get(1)
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
            {
                user_ids.insert(user_id.to_string());
            }
        }
    }
}

fn hydrate_space_records(
    session: &NotionDesktopSession,
    space_id: &str,
    table: &str,
    record_ids: &[String],
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    let loaded = optional_record_map_table(response, table);
    let missing = record_ids
        .iter()
        .filter(|record_id| {
            loaded
                .and_then(|records| records.get(record_id.as_str()))
                .and_then(loaded_record_value)
                .is_none()
        })
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    let requests = missing
        .iter()
        .map(|record_id| {
            json!({
                "pointer": { "table": table, "id": record_id, "spaceId": space_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let hydrated: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesSpace,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    merge_hydrated_record_map(response, hydrated, table)?;
    let records = table_for_references(response, table, &missing)?;
    for record_id in &missing {
        let value = referenced_value(records, table, record_id)?;
        validate_record_space(value, table, record_id, space_id)?;
    }
    Ok(())
}

fn hydrate_comment_users(
    session: &NotionDesktopSession,
    space_id: &str,
    user_ids: &[String],
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    let loaded = optional_record_map_table(response, "notion_user");
    let missing = user_ids
        .iter()
        .filter(|user_id| !loaded_comment_user_is_valid(loaded, user_id))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    let requests = missing
        .iter()
        .map(|user_id| {
            json!({
                "pointer": { "table": "notion_user", "id": user_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let hydrated: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    merge_hydrated_record_map(response, hydrated, "notion_user")?;
    let users = optional_record_map_table(response, "notion_user");
    for user_id in &missing {
        if !loaded_comment_user_is_valid(users, user_id) {
            return Err(NotionLiveError::Fatal(format!(
                "Notion comment user {user_id} was not hydrated with a display name"
            )));
        }
    }
    Ok(())
}

fn merge_hydrated_record_map(
    response: &mut Value,
    mut hydrated: Value,
    table: &str,
) -> Result<(), NotionLiveError> {
    let incoming = hydrated
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .map(std::mem::take)
        .ok_or_else(|| {
            NotionLiveError::Fatal(format!(
                "missing recordMap in hydrated Notion {table} records"
            ))
        })?;
    let destination = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            NotionLiveError::Fatal("missing recordMap in Notion page response".to_string())
        })?;
    merge_record_map(destination, incoming).map_err(NotionLiveError::Fatal)
}

fn table_for_references<'a>(
    response: &'a Value,
    table: &str,
    referenced_ids: &[String],
) -> Result<&'a Map<String, Value>, String> {
    if referenced_ids.is_empty() {
        return Ok(optional_record_map_table(response, table).unwrap_or(empty_record_table()));
    }
    record_map_table(response, table)
}

fn empty_record_table() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn referenced_value<'a>(
    records: &'a Map<String, Value>,
    table: &str,
    record_id: &str,
) -> Result<&'a Value, String> {
    let value = records
        .get(record_id)
        .and_then(loaded_record_value)
        .ok_or_else(|| format!("missing hydrated Notion {table} record {record_id}"))?;
    if let Some(hydrated_id) = value.get("id") {
        if hydrated_id.as_str() != Some(record_id) {
            return Err(format!(
                "Notion {table} key {record_id} contains a different record ID"
            ));
        }
    }
    Ok(value)
}

fn validate_record_space(
    value: &Value,
    table: &str,
    record_id: &str,
    space_id: &str,
) -> Result<(), String> {
    if required_string(value, "space_id")? != space_id {
        return Err(format!(
            "Notion {table} record {record_id} belongs to a different space"
        ));
    }
    Ok(())
}

fn loaded_comment_user_is_valid(users: Option<&Map<String, Value>>, user_id: &str) -> bool {
    users
        .and_then(|users| users.get(user_id))
        .and_then(loaded_record_value)
        .is_some_and(|user| {
            user.get("id").and_then(Value::as_str) == Some(user_id)
                && user
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| !name.trim().is_empty())
        })
}
