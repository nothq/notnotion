use std::collections::HashSet;

use serde_json::json;

use super::{
    block_record::{page_block_record_state, PageBlockRecordState, PageBlockRequirement},
    HydratedPageBlockScope,
};
use crate::live::board::{
    loaded_record_value, optional_record_map_table, record_map::merge_record_map, record_map_table,
    Map, NotionPrivateApiEndpoint, Value,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

pub(super) fn hydrate_page_users(
    session: &NotionDesktopSession,
    scope: &HydratedPageBlockScope,
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    let user_ids = page_user_ids(scope, response)?;
    if user_ids.is_empty() {
        return Ok(());
    }

    let hydrated_record_map = load_page_user_records(session, &scope.space_id, &user_ids)?;
    let response_record_map = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in Notion page response".to_string())?;
    merge_record_map(response_record_map, hydrated_record_map)?;
    remove_unusable_page_users(response, &user_ids).map_err(NotionLiveError::Fatal)
}

fn page_user_ids(scope: &HydratedPageBlockScope, response: &Value) -> Result<Vec<String>, String> {
    let blocks = record_map_table(response, "block")?;
    let loaded_users = optional_record_map_table(response, "notion_user");
    let mut user_ids = scope
        .user_ids
        .iter()
        .filter(|user_id| !loaded_page_user_is_valid(loaded_users, user_id))
        .cloned()
        .collect::<HashSet<_>>();
    for block_id in &scope.block_ids {
        if block_id == &scope.root_block_id {
            continue;
        }
        let block =
            match page_block_record_state(blocks, block_id, PageBlockRequirement::ReferenceTarget)?
            {
                PageBlockRecordState::Complete(block) => block,
                PageBlockRecordState::NoValue
                | PageBlockRecordState::Unavailable
                | PageBlockRecordState::Incomplete => {
                    return Err(format!("missing hydrated Notion page block {block_id}"));
                }
            };
        let Some(editor_id) = block.last_editor_id() else {
            continue;
        };
        if !loaded_page_user_is_valid(loaded_users, editor_id) {
            user_ids.insert(editor_id.to_string());
        }
    }
    let mut user_ids = user_ids.into_iter().collect::<Vec<_>>();
    user_ids.sort();
    Ok(user_ids)
}

fn load_page_user_records(
    session: &NotionDesktopSession,
    space_id: &str,
    user_ids: &[String],
) -> Result<Map<String, Value>, NotionLiveError> {
    let requests = user_ids
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
    hydrated
        .get("recordMap")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| {
            NotionLiveError::Fatal("missing recordMap in hydrated Notion page users".to_string())
        })
}

fn loaded_page_user_is_valid(users: Option<&Map<String, Value>>, editor_id: &str) -> bool {
    let Some(entry) = users.and_then(|users| users.get(editor_id)) else {
        return false;
    };
    let Some(user) = loaded_record_value(entry) else {
        return false;
    };
    page_user_is_usable(user, editor_id)
}

fn remove_unusable_page_users(response: &mut Value, editor_ids: &[String]) -> Result<(), String> {
    let Some(users) = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .and_then(|record_map| record_map.get_mut("notion_user"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    for editor_id in editor_ids {
        let usable = users
            .get(editor_id)
            .and_then(loaded_record_value)
            .is_some_and(|user| page_user_is_usable(user, editor_id));
        if !usable {
            users.remove(editor_id);
        }
    }
    Ok(())
}

fn page_user_is_usable(user: &Value, editor_id: &str) -> bool {
    user.get("id").and_then(Value::as_str) == Some(editor_id)
        && user
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| !name.trim().is_empty())
}
