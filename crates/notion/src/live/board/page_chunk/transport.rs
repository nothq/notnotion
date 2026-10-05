use std::{collections::HashSet, thread};

use serde_json::json;

use super::PageChunkResponse;
use crate::live::board::{NotionPrivateApiEndpoint, Value};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

pub(super) fn validate_continuation_dedupe_session(
    chunk: &PageChunkResponse,
    original_dedupe_session_id: Option<&str>,
) -> Result<(), String> {
    if chunk
        .dedupe_session_id
        .as_deref()
        .is_some_and(|response_id| Some(response_id) != original_dedupe_session_id)
    {
        return Err("Notion page chunk response changed dedupeSessionId".to_string());
    }
    Ok(())
}

pub(super) fn load_page_chunk_wave(
    session: &NotionDesktopSession,
    page_id: &str,
    cursors: Vec<Value>,
    dedupe_session_id: Option<&str>,
) -> Result<Vec<PageChunkResponse>, NotionLiveError> {
    thread::scope(|scope| {
        let handles = cursors
            .into_iter()
            .map(|cursor| {
                scope.spawn(move || {
                    request_page_chunk(session, page_id, cursor, dedupe_session_id).and_then(
                        |response| {
                            parse_page_chunk_response(response).map_err(NotionLiveError::Fatal)
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| {
                handle.join().map_err(|_| {
                    NotionLiveError::Fatal("Notion page chunk request thread panicked".to_string())
                })?
            })
            .collect()
    })
}

fn request_page_chunk(
    session: &NotionDesktopSession,
    page_id: &str,
    cursor: Value,
    dedupe_session_id: Option<&str>,
) -> Result<Value, NotionLiveError> {
    let request = page_chunk_request(page_id, cursor, dedupe_session_id);
    post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::LoadCachedPageChunkV2,
        &request,
    )
}

pub(super) fn request_initial_page_chunk_with_session(
    session: &NotionDesktopSession,
    page_id: &str,
) -> Result<Value, NotionLiveError> {
    let request = page_chunk_request(page_id, json!({ "stack": [] }), None);
    post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::LoadCachedPageChunkV2,
        &request,
    )
}

fn page_chunk_request(page_id: &str, cursor: Value, dedupe_session_id: Option<&str>) -> Value {
    let mut request = json!({
        "page": { "id": page_id },
        "cursor": cursor,
        "verticalColumns": false,
    });
    if let Some(dedupe_session_id) = dedupe_session_id {
        request
            .as_object_mut()
            .expect("Notion page chunk request is an object")
            .insert(
                "dedupeSessionId".to_string(),
                Value::String(dedupe_session_id.to_string()),
            );
    }
    request
}

pub(super) fn parse_page_chunk_response(response: Value) -> Result<PageChunkResponse, String> {
    let Value::Object(mut response) = response else {
        return Err("Notion page chunk response is not an object".to_string());
    };
    let record_map = match response.remove("recordMap") {
        Some(Value::Object(record_map)) => record_map,
        _ => return Err("missing object field recordMap in Notion page chunk".to_string()),
    };
    let cursors = match response.remove("cursors") {
        Some(Value::Array(cursors)) => cursors,
        _ => return Err("missing array field cursors in Notion page chunk".to_string()),
    };
    let dedupe_session_id = match response.get("dedupeSessionId") {
        Some(Value::String(dedupe_session_id)) => Some(dedupe_session_id.clone()),
        Some(_) => return Err("invalid dedupeSessionId in Notion page chunk".to_string()),
        None => None,
    };
    Ok(PageChunkResponse {
        remaining_fields: response,
        record_map,
        cursors,
        dedupe_session_id,
    })
}

pub(super) fn continuation_cursors(
    cursors: Vec<Value>,
    seen_cursors: &mut HashSet<String>,
) -> Result<Vec<Value>, String> {
    let mut continuations = Vec::new();
    for cursor in cursors {
        let stack = cursor
            .get("stack")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing array field stack in Notion page cursor".to_string())?;
        if stack.is_empty() {
            continue;
        }
        let cursor_key = serde_json::to_string(&cursor)
            .map_err(|error| format!("failed to serialize Notion page cursor: {error}"))?;
        if !seen_cursors.insert(cursor_key) {
            return Err("Notion page chunk response repeated a continuation cursor".to_string());
        }
        continuations.push(cursor);
    }
    Ok(continuations)
}
