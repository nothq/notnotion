use serde_json::{json, Value};
use std::collections::HashSet;

use super::super::super::super::{
    plain_text_from_property_value, required_string, title_property, unwrap_record_value,
    BoardTarget, NotionPrivateApiEndpoint,
};
use super::records::{
    newest_activity_author, non_empty_text, optional_string, optional_timestamp, resolve_actor,
    validate_space, InboxRecords,
};
use super::wire::{ActivityLogRequest, ActivityLogResponse};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use crate::model::{LoadSidebarInboxResult, PageShellInboxItem};

pub(super) fn load_workspace_updates(
    session: &NotionDesktopSession,
    space_id: &str,
    board_target: &BoardTarget,
    size: u32,
) -> Result<LoadSidebarInboxResult, NotionLiveError> {
    let request = ActivityLogRequest {
        space_id,
        limit: size,
        activity_types: Vec::new(),
    };
    let response: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::GetActivityLog,
        &serde_json::to_value(request)
            .map_err(|error| format!("failed to encode Notion activity-log request: {error}"))?,
    )?;
    let response = serde_json::from_value::<ActivityLogResponse>(response)
        .map_err(|error| format!("invalid Notion private API activity-log response: {error}"))?;
    let has_more = response.activity_ids.len() >= size as usize;
    let mut records = InboxRecords::new(response.record_map);
    hydrate_activity_actors(session, space_id, &mut records)?;
    let items = response
        .activity_ids
        .iter()
        .map(|activity_id| shape_workspace_activity(activity_id, space_id, board_target, &records))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadSidebarInboxResult { items, has_more })
}

fn hydrate_activity_actors(
    session: &NotionDesktopSession,
    space_id: &str,
    records: &mut InboxRecords,
) -> Result<(), NotionLiveError> {
    let pointers = activity_actor_pointers(records)?;
    let pointers = missing_actor_pointers(records, pointers)?;
    if pointers.is_empty() {
        return Ok(());
    }
    let requests = pointers
        .iter()
        .map(|(table, id)| {
            json!({
                "pointer": { "table": table, "id": id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let response: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    let hydrated = response
        .get("recordMap")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing Notion activity actor recordMap".to_string())?;
    merge_hydrated_actors(records, hydrated, pointers).map_err(NotionLiveError::Fatal)
}

/// The record table and ID of each activity actor.
type ActorPointers = HashSet<(String, String)>;

fn activity_actor_pointers(records: &InboxRecords) -> Result<ActorPointers, String> {
    let mut pointers = HashSet::new();
    if let Some(activities) = records.table("activity")? {
        for entry in activities.values() {
            let Some(activity) = unwrap_record_value(entry) else {
                continue;
            };
            let Some(pointer) = newest_activity_author(activity)? else {
                continue;
            };
            if matches!(pointer.table, "notion_user" | "bot") {
                pointers.insert((pointer.table.to_string(), pointer.id.to_string()));
            }
        }
    }
    Ok(pointers)
}

fn missing_actor_pointers(
    records: &InboxRecords,
    pointers: ActorPointers,
) -> Result<ActorPointers, String> {
    let mut missing_pointers = HashSet::new();
    for (table, id) in pointers {
        if records.record(&table, &id)?.is_none() {
            missing_pointers.insert((table, id));
        }
    }
    Ok(missing_pointers)
}

fn merge_hydrated_actors(
    records: &mut InboxRecords,
    hydrated: &serde_json::Map<String, Value>,
    pointers: ActorPointers,
) -> Result<(), String> {
    for (table, id) in pointers {
        let Some(entry) = hydrated
            .get(&table)
            .and_then(Value::as_object)
            .and_then(|table| table.get(&id))
        else {
            continue;
        };
        records.insert_hydrated_actor(table, id, entry.clone())?;
    }
    Ok(())
}

fn shape_workspace_activity(
    activity_id: &str,
    space_id: &str,
    board_target: &BoardTarget,
    records: &InboxRecords,
) -> Result<PageShellInboxItem, String> {
    let activity = records.required_record("activity", activity_id)?;
    validate_space(activity, space_id, "activity")?;
    let activity_type = required_string(activity, "type")?.to_string();
    let target_block_id = workspace_activity_target_block_id(activity, records)?;
    let block = target_block_id
        .as_deref()
        .map(|block_id| records.record("block", block_id))
        .transpose()?
        .flatten();
    let title = workspace_activity_title(activity, block, records)?;
    let body = workspace_activity_preview(activity, records)?;
    let actor = resolve_actor(Some(activity), None, records)?;
    let event_time_ms =
        optional_timestamp(activity, "end_time")?.or(optional_timestamp(activity, "start_time")?);
    let target_board_url = target_block_id
        .as_deref()
        .map(|block_id| board_target.child_url(block_id));
    Ok(PageShellInboxItem {
        notification_id: format!("activity-{activity_id}"),
        activity_id: Some(activity_id.to_string()),
        notification_type: activity_type,
        actor,
        title,
        body,
        event_time_ms,
        read: true,
        archived: false,
        target_board_url,
    })
}

fn workspace_activity_target_block_id(
    activity: &Value,
    records: &InboxRecords,
) -> Result<Option<String>, String> {
    for key in ["navigable_block_id", "collection_row_id", "context_id"] {
        let Some(block_id) = optional_string(activity, key)? else {
            continue;
        };
        if records.record("block", block_id)?.is_some() {
            return Ok(Some(block_id.to_string()));
        }
    }
    let Some(collection_id) = optional_string(activity, "collection_id")? else {
        return Ok(None);
    };
    let Some(blocks) = records.table("block")? else {
        return Ok(None);
    };
    for entry in blocks.values() {
        let Some(block) = unwrap_record_value(entry) else {
            continue;
        };
        if optional_string(block, "collection_id")? != Some(collection_id) {
            continue;
        }
        if let Some(block_id) = optional_string(block, "id")? {
            return Ok(Some(block_id.to_string()));
        }
    }
    Ok(None)
}

fn workspace_activity_title(
    activity: &Value,
    block: Option<&Value>,
    records: &InboxRecords,
) -> Result<String, String> {
    if let Some(title) = block.map(title_property).transpose()?.flatten() {
        return Ok(title);
    }
    let collection = optional_string(activity, "collection_id")?
        .map(|collection_id| records.record("collection", collection_id))
        .transpose()?
        .flatten();
    if let Some(name) = collection.and_then(|collection| collection.get("name")) {
        let title = plain_text_from_property_value(
            name,
            records.table("block")?,
            records.table("notion_user")?,
        )?;
        if let Some(title) = non_empty_text(title) {
            return Ok(title);
        }
    }
    Ok("Workspace update".to_string())
}

fn workspace_activity_preview(
    activity: &Value,
    records: &InboxRecords,
) -> Result<Option<String>, String> {
    let Some(edits) = activity.get("edits") else {
        return Ok(None);
    };
    let edits = edits
        .as_array()
        .ok_or_else(|| "invalid Notion activity-log edits".to_string())?;
    for edit in edits.iter().rev() {
        for key in ["block_data", "collection_property_data"] {
            let Some(data) = edit.get(key) else {
                continue;
            };
            let candidate = data.get("after").or_else(|| {
                data.get("block_value")
                    .and_then(|block| block.get("properties"))
            });
            let Some(candidate) = candidate else {
                continue;
            };
            let values = candidate
                .as_object()
                .map(|properties| properties.values().collect::<Vec<_>>())
                .unwrap_or_else(|| vec![candidate]);
            for value in values {
                let text = plain_text_from_property_value(
                    value,
                    records.table("block")?,
                    records.table("notion_user")?,
                )?;
                if let Some(text) = non_empty_text(text) {
                    return Ok(Some(text));
                }
            }
        }
    }
    Ok(None)
}
