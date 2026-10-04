use std::collections::HashSet;

use super::{record_map::merge_record_map, Map, Value};

mod block_record;
mod collection_views;
mod collections;
mod comments;
mod hydration;
mod opaque;
mod transport;
mod users;

pub(in crate::live::board) use collection_views::hydrate_collection_views;
pub(in crate::live::board) use opaque::ProvenOpaqueUnavailableBlocks;

use crate::live::{
    credentials::{current_notion_desktop_session, NotionDesktopSession},
    NotionLiveError, NotionResourceFailure,
};
use collection_views::hydrate_linked_view_collections;
use collections::hydrate_page_collections;
use comments::hydrate_page_comments;
use hydration::hydrate_reachable_page_blocks;
use transport::{
    continuation_cursors, load_page_chunk_wave, parse_page_chunk_response,
    request_initial_page_chunk_with_session, validate_continuation_dedupe_session,
};
use users::hydrate_page_users;

const PAGE_CHUNK_MAX_WAVES: usize = 64;

struct PageChunkResponse {
    remaining_fields: Map<String, Value>,
    record_map: Map<String, Value>,
    cursors: Vec<Value>,
    dedupe_session_id: Option<String>,
}

struct CompletePageChunks {
    remaining_fields: Map<String, Value>,
    record_map: Map<String, Value>,
}

struct HydratedPageBlockScope {
    root_block_id: String,
    space_id: String,
    block_ids: HashSet<String>,
    unavailable_reference_block_ids: HashSet<String>,
    opaque_unavailable_blocks: opaque::ProvenOpaqueUnavailableBlocks,
    collection_ids_by_space: std::collections::BTreeMap<String, HashSet<String>>,
    linked_view_ids_by_space:
        std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    user_ids: HashSet<String>,
}

#[derive(Clone, Debug)]
pub(in crate::live) struct CompletePageResponse {
    value: Value,
    unavailable_reference_block_ids: HashSet<String>,
    opaque_unavailable_blocks: opaque::ProvenOpaqueUnavailableBlocks,
}

impl CompletePageResponse {
    fn new(
        response: Value,
        unavailable_reference_block_ids: HashSet<String>,
        opaque_unavailable_blocks: opaque::ProvenOpaqueUnavailableBlocks,
    ) -> Self {
        Self {
            value: response,
            unavailable_reference_block_ids,
            opaque_unavailable_blocks,
        }
    }

    pub(in crate::live::board) fn as_value(&self) -> &Value {
        &self.value
    }

    pub(in crate::live::board) fn unavailable_reference_block_ids(&self) -> &HashSet<String> {
        &self.unavailable_reference_block_ids
    }

    pub(in crate::live::board) fn opaque_unavailable_blocks(
        &self,
    ) -> &opaque::ProvenOpaqueUnavailableBlocks {
        &self.opaque_unavailable_blocks
    }

    pub(in crate::live::board) fn record_map_mut(
        &mut self,
    ) -> Result<&mut Map<String, Value>, String> {
        self.value
            .get_mut("recordMap")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| "completed Notion page response is missing recordMap".to_string())
    }
}

pub(super) fn load_complete_page_response_typed(
    page_id: &str,
) -> Result<CompletePageResponse, String> {
    let session = current_notion_desktop_session().map_err(|error| error.to_string())?;
    load_complete_page_response_with_session(&session, page_id).map_err(|error| error.to_string())
}

pub(super) fn load_complete_page_response_with_session(
    session: &NotionDesktopSession,
    page_id: &str,
) -> Result<CompletePageResponse, NotionLiveError> {
    let first_response = request_initial_page_chunk_with_session(session, page_id)?;
    validate_initial_page_access(page_id, &first_response)?;
    complete_page_response_with_session(session, page_id, first_response)
}

fn validate_initial_page_access(page_id: &str, response: &Value) -> Result<(), NotionLiveError> {
    let role = response
        .get("recordMap")
        .and_then(|record_map| record_map.get("block"))
        .and_then(Value::as_object)
        .and_then(|blocks| blocks.get(page_id))
        .and_then(|entry| entry.get("role"))
        .and_then(Value::as_str);
    if role != Some("none") {
        return Ok(());
    }
    Err(NotionLiveError::Unavailable(
        NotionResourceFailure::Restricted {
            diagnostic: format!("Notion route block {page_id} is restricted"),
        },
    ))
}

fn complete_page_response_with_session(
    session: &NotionDesktopSession,
    page_id: &str,
    first_response: Value,
) -> Result<CompletePageResponse, NotionLiveError> {
    let first_chunk = parse_page_chunk_response(first_response)?;
    let chunks = complete_page_chunks(session, page_id, first_chunk)?;
    let mut response = chunks.remaining_fields;
    response.insert("recordMap".to_string(), Value::Object(chunks.record_map));
    response.insert("cursors".to_string(), Value::Array(Vec::new()));
    let mut response = Value::Object(response);
    let (unavailable_reference_block_ids, opaque_unavailable_blocks) =
        if let Some(mut scope) = hydrate_reachable_page_blocks(session, page_id, &mut response)? {
            hydrate_page_comments(session, &scope, &mut response)?;
            hydrate_linked_view_collections(session, &mut scope, &mut response)?;
            hydrate_page_collections(session, &scope, &mut response)?;
            hydrate_page_users(session, &scope, &mut response)?;
            (
                scope.unavailable_reference_block_ids,
                scope.opaque_unavailable_blocks,
            )
        } else {
            (HashSet::new(), opaque::ProvenOpaqueUnavailableBlocks::new())
        };
    Ok(CompletePageResponse::new(
        response,
        unavailable_reference_block_ids,
        opaque_unavailable_blocks,
    ))
}

fn complete_page_chunks(
    session: &NotionDesktopSession,
    page_id: &str,
    first_chunk: PageChunkResponse,
) -> Result<CompletePageChunks, NotionLiveError> {
    let dedupe_session_id = first_chunk.dedupe_session_id.clone();
    let mut merged_record_map = Map::new();
    merge_record_map(&mut merged_record_map, first_chunk.record_map)?;
    let mut seen_cursors = HashSet::new();
    let mut pending_cursors = continuation_cursors(first_chunk.cursors, &mut seen_cursors)?;
    if !pending_cursors.is_empty() && dedupe_session_id.is_none() {
        return Err(NotionLiveError::Fatal(
            "missing dedupeSessionId for paginated Notion page chunk response".to_string(),
        ));
    }
    for _ in 0..PAGE_CHUNK_MAX_WAVES {
        if pending_cursors.is_empty() {
            break;
        }
        let chunks = load_page_chunk_wave(
            session,
            page_id,
            pending_cursors,
            dedupe_session_id.as_deref(),
        )?;
        pending_cursors = Vec::new();
        for chunk in chunks {
            validate_continuation_dedupe_session(&chunk, dedupe_session_id.as_deref())?;
            merge_record_map(&mut merged_record_map, chunk.record_map)?;
            pending_cursors.extend(continuation_cursors(chunk.cursors, &mut seen_cursors)?);
        }
    }
    if !pending_cursors.is_empty() {
        return Err(NotionLiveError::Fatal(format!(
            "Notion page {page_id} exceeded {PAGE_CHUNK_MAX_WAVES} cursor waves"
        )));
    }
    Ok(CompletePageChunks {
        remaining_fields: first_chunk.remaining_fields,
        record_map: merged_record_map,
    })
}
