use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::super::super::{
    record_map_table, required_string, required_u64, unwrap_record_value, NotionPrivateApiEndpoint,
};
use super::context::load_sidebar_workspace_context;
use super::records::required_bool;
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use crate::model::{LoadSidebarChatsResult, PageShellChatCursor, PageShellChatThread};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatHistoryRequest<'a> {
    thread_parent_pointer: ChatThreadParentPointer<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<Value>,
    include_workflow_threads: bool,
    include_writer_chats: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatThreadParentPointer<'a> {
    table: &'static str,
    id: &'a str,
    space_id: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatHistoryResponse {
    next_cursor: Option<Value>,
    thread_ids: Vec<String>,
    unread_thread_ids: Option<Vec<String>>,
}

pub(crate) fn load_sidebar_chats(
    session: &NotionDesktopSession,
    current_board_url: &str,
    cursor: Option<PageShellChatCursor>,
) -> Result<LoadSidebarChatsResult, NotionLiveError> {
    let context = load_sidebar_workspace_context(session, current_board_url)?;
    let sidebar = context.user.sidebar_for_space(&context.space_id)?;
    let cursor = cursor
        .map(|cursor| {
            serde_json::from_str(cursor.serialized())
                .map_err(|error| format!("invalid Notion chat cursor: {error}"))
        })
        .transpose()?;
    let request = ChatHistoryRequest {
        thread_parent_pointer: ChatThreadParentPointer {
            table: "space",
            id: &context.space_id,
            space_id: &context.space_id,
        },
        cursor,
        include_workflow_threads: true,
        include_writer_chats: false,
    };
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::GetInferenceTranscriptsForUser,
        &serde_json::to_value(request)
            .map_err(|error| format!("failed to encode Notion chat history request: {error}"))?,
    )?;
    let response = serde_json::from_value::<ChatHistoryResponse>(response)
        .map_err(|error| format!("invalid Notion private API chat history response: {error}"))?;
    let thread_ids = combined_thread_ids(&response.thread_ids, &sidebar.pinned_chat_ids);
    let thread_records = load_thread_records(session, &context.space_id, &thread_ids)?;
    let unread = response
        .unread_thread_ids
        .unwrap_or_default()
        .into_iter()
        .collect::<HashSet<_>>();
    let pinned = sidebar
        .pinned_chat_ids
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    let threads = shape_chat_threads(&response.thread_ids, &pinned, &unread, &thread_records)?;
    let next_cursor = response
        .next_cursor
        .map(|cursor| {
            serde_json::to_string(&cursor)
                .map(PageShellChatCursor::from_serialized)
                .map_err(|error| format!("failed to encode Notion chat cursor: {error}"))
        })
        .transpose()?;
    Ok(LoadSidebarChatsResult {
        threads,
        next_cursor,
    })
}

fn combined_thread_ids(history: &[String], pinned: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    pinned
        .iter()
        .take(8)
        .chain(history)
        .filter(|thread_id| seen.insert(thread_id.as_str()))
        .cloned()
        .collect()
}

fn load_thread_records(
    session: &NotionDesktopSession,
    space_id: &str,
    thread_ids: &[String],
) -> Result<Map<String, Value>, NotionLiveError> {
    if thread_ids.is_empty() {
        return Ok(Map::new());
    }
    let requests = thread_ids
        .iter()
        .map(|thread_id| {
            json!({
                "pointer": { "table": "thread", "id": thread_id, "spaceId": space_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesSpace,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    Ok(record_map_table(&response, "thread")?.clone())
}

fn shape_chat_threads(
    history: &[String],
    pinned: &[String],
    unread: &HashSet<String>,
    records: &Map<String, Value>,
) -> Result<Vec<PageShellChatThread>, String> {
    let pinned_set = pinned.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut by_id = Map::new();
    for thread_id in pinned.iter().chain(history) {
        if by_id.contains_key(thread_id) {
            continue;
        }
        let record = records
            .get(thread_id)
            .and_then(unwrap_record_value)
            .ok_or_else(|| format!("missing hydrated Notion thread {thread_id}"))?;
        validate_required_thread_record(record)?;
        if !required_bool(record, "alive")? {
            continue;
        }
        by_id.insert(thread_id.clone(), record.clone());
    }
    let mut shaped = Vec::with_capacity(by_id.len());
    for thread_id in pinned {
        if let Some(record) = by_id.get(thread_id) {
            shaped.push(shape_chat_thread(thread_id, record, unread, true)?);
        }
    }
    let mut recent = history
        .iter()
        .filter(|thread_id| !pinned_set.contains(thread_id.as_str()))
        .filter_map(|thread_id| {
            by_id
                .get(thread_id)
                .map(|record| shape_chat_thread(thread_id, record, unread, false))
        })
        .collect::<Result<Vec<_>, _>>()?;
    recent.sort_by_key(|thread| std::cmp::Reverse(thread.sort_time));
    shaped.extend(recent);
    Ok(shaped)
}

fn validate_required_thread_record(record: &Value) -> Result<(), String> {
    required_u64(record, "version")?;
    required_string(record, "parent_id")?;
    required_string(record, "parent_table")?;
    required_string(record, "space_id")?;
    required_u64(record, "created_time")?;
    required_string(record, "created_by_id")?;
    required_string(record, "created_by_table")?;
    Ok(())
}

fn shape_chat_thread(
    thread_id: &str,
    record: &Value,
    unread: &HashSet<String>,
    pinned: bool,
) -> Result<PageShellChatThread, String> {
    let created_time = required_u64(record, "created_time")?;
    let sort_time = optional_u64(record, "updated_time")?.unwrap_or(created_time);
    let title = record
        .get("data")
        .and_then(|data| data.get("title"))
        .map(|title| {
            title
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "invalid Notion chat title".to_string())
        })
        .transpose()?
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "New AI chat".to_string());
    Ok(PageShellChatThread {
        thread_id: thread_id.to_string(),
        title,
        created_time,
        sort_time,
        unread: unread.contains(thread_id),
        pinned,
    })
}

fn optional_u64(value: &Value, key: &str) -> Result<Option<u64>, String> {
    value
        .get(key)
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("invalid u64 field {key}"))
        })
        .transpose()
}
