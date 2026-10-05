use std::collections::BTreeSet;

use serde_json::json;

use crate::live::board::{
    loaded_record_value, optional_record_map_table, record_map::merge_record_map,
    record_value_state, Map, RecordValueState, Value,
};
use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_with_session, NotionPrivateApiEndpoint},
    NotionLiveError,
};

const REFERENCE_REQUEST_CHUNK: usize = 100;

/// A referenced page ID and the space ID its pointer names, when present.
type PageReference<'a> = (&'a str, Option<&'a str>);

#[derive(Default)]
struct RowReferences<'a> {
    pages: BTreeSet<PageReference<'a>>,
    people: BTreeSet<&'a str>,
}

/// Loads the pages and people that database rows reference in their
/// properties, such as relation targets, so cells show their titles and names
/// as Notion does.
pub(super) fn hydrate_row_references(
    session: &NotionDesktopSession,
    space_id: &str,
    collection_id: &str,
    query_response: &mut Value,
) -> Result<(), NotionLiveError> {
    let (pages, people) = missing_reference_requests(query_response, space_id, collection_id);
    for chunk in pages.chunks(REFERENCE_REQUEST_CHUNK) {
        merge_references(session, json!({ "requests": chunk }), query_response)?;
    }
    for chunk in people.chunks(REFERENCE_REQUEST_CHUNK) {
        let body = json!({
            "requests": chunk,
            "spacePointer": { "table": "space", "id": space_id },
        });
        merge_references(session, body, query_response)?;
    }
    Ok(())
}

fn merge_references(
    session: &NotionDesktopSession,
    body: Value,
    query_response: &mut Value,
) -> Result<(), NotionLiveError> {
    let mut hydrated: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &body,
    )?;
    let incoming = hydrated
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in hydrated Notion row references".to_string())?;
    let existing = query_response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in Notion query collection response".to_string())?;
    merge_record_map(existing, std::mem::take(incoming))?;
    Ok(())
}

fn missing_reference_requests(
    query_response: &Value,
    space_id: &str,
    collection_id: &str,
) -> (Vec<Value>, Vec<Value>) {
    let blocks = optional_record_map_table(query_response, "block");
    let users = optional_record_map_table(query_response, "notion_user");
    let schema = optional_record_map_table(query_response, "collection")
        .and_then(|collections| collections.get(collection_id))
        .and_then(loaded_record_value)
        .and_then(|collection| collection.get("schema"))
        .and_then(Value::as_object);
    let mut references = RowReferences::default();
    let rows = blocks
        .into_iter()
        .flat_map(Map::values)
        .filter_map(loaded_record_value)
        .filter(|block| block.get("parent_id").and_then(Value::as_str) == Some(collection_id));
    for row in rows {
        let properties = row.get("properties").and_then(Value::as_object);
        for (property_id, value) in properties.into_iter().flatten() {
            if schema.is_some_and(|schema| schema.contains_key(property_id)) {
                collect_references(value, &mut references);
            }
        }
    }
    let missing = |table: Option<&Map<String, Value>>, id: &str| {
        matches!(
            record_value_state(table.and_then(|table| table.get(id))),
            RecordValueState::Incomplete
        )
    };
    let pages = references
        .pages
        .into_iter()
        .filter(|(id, _)| missing(blocks, id))
        .map(|(id, page_space_id)| {
            let space_id = page_space_id.unwrap_or(space_id);
            json!({ "pointer": { "table": "block", "id": id, "spaceId": space_id }, "version": -1 })
        })
        .collect();
    let people = references
        .people
        .into_iter()
        .filter(|id| missing(users, id))
        .map(|id| json!({ "pointer": { "table": "notion_user", "id": id }, "version": -1 }))
        .collect();
    (pages, people)
}

fn collect_references<'a>(value: &'a Value, references: &mut RowReferences<'a>) {
    for chunk in value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
    {
        if chunk.first().and_then(Value::as_str) != Some("‣") {
            continue;
        }
        let pointers = chunk.get(1).and_then(Value::as_array).into_iter().flatten();
        for pointer in pointers.filter_map(Value::as_array) {
            let id = pointer.get(1).and_then(Value::as_str);
            match (pointer.first().and_then(Value::as_str), id) {
                (Some("p"), Some(id)) => {
                    references
                        .pages
                        .insert((id, pointer.get(2).and_then(Value::as_str)));
                }
                (Some("u"), Some(id)) => {
                    references.people.insert(id);
                }
                _ => {}
            }
        }
    }
}
