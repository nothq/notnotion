use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::HydratedPageBlockScope;
use crate::live::board::{
    optional_record_map_table, record_map::merge_record_map, record_value_state,
    view_collection_pointer, NotionPrivateApiEndpoint, RecordValueState, Value,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

/// A linked database view names its collection on each view record, so the
/// page hydrates those records before it hydrates collections.
pub(super) fn hydrate_linked_view_collections(
    session: &NotionDesktopSession,
    scope: &mut HydratedPageBlockScope,
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    hydrate_collection_views(session, &scope.linked_view_ids_by_space, response)?;
    let views = optional_record_map_table(response, "collection_view");
    for view_id in scope.linked_view_ids_by_space.values().flatten() {
        let view = match record_value_state(views.and_then(|views| views.get(view_id))) {
            RecordValueState::Unavailable => continue,
            RecordValueState::Incomplete => {
                return Err(NotionLiveError::Fatal(format!(
                    "Notion did not hydrate linked database view {view_id}"
                )));
            }
            RecordValueState::Present(view) => view,
        };
        // Readers of a view without a pointer fail when they need its collection.
        let Some(pointer) = view_collection_pointer(view, view_id)? else {
            continue;
        };
        scope
            .collection_ids_by_space
            .entry(pointer.space_id.to_string())
            .or_default()
            .insert(pointer.id.to_string());
    }
    Ok(())
}

/// Requests the listed view records that `response` lacks or holds only
/// partially, and merges them into its record map.
pub(in crate::live::board) fn hydrate_collection_views(
    session: &NotionDesktopSession,
    view_ids_by_space: &BTreeMap<String, BTreeSet<String>>,
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    let views = optional_record_map_table(response, "collection_view");
    let requests = view_ids_by_space
        .iter()
        .flat_map(|(space_id, view_ids)| view_ids.iter().map(move |id| (space_id, id)))
        .filter(|(_, view_id)| view_needs_hydration(views.and_then(|views| views.get(*view_id))))
        .map(|(space_id, view_id)| {
            json!({
                "pointer": { "table": "collection_view", "id": view_id, "spaceId": space_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    if requests.is_empty() {
        return Ok(());
    }
    let mut hydrated: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({ "requests": requests }),
    )?;
    let incoming = hydrated
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in hydrated Notion collection views".to_string())?;
    let existing = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in Notion page response".to_string())?;
    merge_record_map(existing, std::mem::take(incoming))?;
    Ok(())
}

fn view_needs_hydration(entry: Option<&Value>) -> bool {
    match record_value_state(entry) {
        RecordValueState::Incomplete => true,
        RecordValueState::Unavailable => false,
        RecordValueState::Present(view) => ["id", "type", "format"]
            .iter()
            .any(|field| view.get(field).is_none()),
    }
}
