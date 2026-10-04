use std::collections::BTreeMap;

use serde_json::json;

use super::HydratedPageBlockScope;
use crate::live::board::{
    loaded_record_value, optional_record_map_table,
    record_map::{merge_record_map, record_value_mut},
    record_value_state, Map, NotionPrivateApiEndpoint, RecordValueState, Value,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

const COLLECTION_HYDRATION_MAX_WAVES: usize = 64;

#[derive(Clone, Copy)]
struct CollectionOmissions {
    name: bool,
    schema: bool,
    format: bool,
}

enum CollectionRecordState {
    NoValue,
    Unavailable,
    Incomplete,
    Complete,
}

enum AuthoritativeCollectionState {
    Present(CollectionOmissions),
    Unavailable,
}

pub(super) fn hydrate_page_collections(
    session: &NotionDesktopSession,
    scope: &HydratedPageBlockScope,
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    for _ in 0..COLLECTION_HYDRATION_MAX_WAVES {
        let missing = missing_collection_pointers(scope, response)?;
        if missing.is_empty() {
            return Ok(());
        }
        let hydrated = request_collections(session, &missing)?;
        merge_authoritative_collections(response, hydrated, &missing)?;
    }
    if missing_collection_pointers(scope, response)?.is_empty() {
        return Ok(());
    }
    Err(NotionLiveError::Fatal(format!(
        "Notion page exceeded {COLLECTION_HYDRATION_MAX_WAVES} collection hydration waves"
    )))
}

type MissingCollectionIdsBySpace = BTreeMap<String, Vec<String>>;

fn missing_collection_pointers(
    scope: &HydratedPageBlockScope,
    response: &Value,
) -> Result<MissingCollectionIdsBySpace, String> {
    let collections = optional_record_map_table(response, "collection");
    let mut missing = BTreeMap::new();
    for (space_id, collection_ids) in &scope.collection_ids_by_space {
        let mut space_missing = Vec::new();
        for collection_id in collection_ids {
            if collection_is_missing(collections, collection_id)? {
                space_missing.push(collection_id.clone());
            }
        }
        space_missing.sort();
        if !space_missing.is_empty() {
            missing.insert(space_id.clone(), space_missing);
        }
    }
    Ok(missing)
}

fn collection_is_missing(
    collections: Option<&Map<String, Value>>,
    collection_id: &str,
) -> Result<bool, String> {
    let Some(entry) = collections.and_then(|collections| collections.get(collection_id)) else {
        return Ok(true);
    };
    Ok(matches!(
        collection_record_state(entry, collection_id)?,
        CollectionRecordState::NoValue | CollectionRecordState::Incomplete
    ))
}

fn collection_record_state(
    entry: &Value,
    collection_id: &str,
) -> Result<CollectionRecordState, String> {
    let value = match record_value_state(Some(entry)) {
        RecordValueState::Incomplete => return Ok(CollectionRecordState::NoValue),
        RecordValueState::Unavailable => return Ok(CollectionRecordState::Unavailable),
        RecordValueState::Present(value) => value,
    };
    let Some(value) = value.as_object() else {
        return Err(format!(
            "Notion collection {collection_id} contains a non-object value"
        ));
    };
    let id = optional_string(value, collection_id, "id")?;
    if let Some(id) = id {
        if id != collection_id {
            return Err(format!(
                "Notion collection key {collection_id} contains collection {id}"
            ));
        }
    }
    let complete = id.is_some()
        && optional_u64(value, collection_id, "version")?.is_some()
        && required_object_is_present(value, collection_id, "schema")?
        && required_object_is_present(value, collection_id, "format")?
        && required_array_is_present(value, collection_id, "name")?;
    Ok(if complete {
        CollectionRecordState::Complete
    } else {
        CollectionRecordState::Incomplete
    })
}

fn request_collections(
    session: &NotionDesktopSession,
    missing: &BTreeMap<String, Vec<String>>,
) -> Result<Value, NotionLiveError> {
    let requests = missing
        .iter()
        .flat_map(|(space_id, collection_ids)| {
            collection_ids.iter().map(move |collection_id| {
                json!({
                    "pointer": {
                        "table": "collection",
                        "id": collection_id,
                        "spaceId": space_id,
                    },
                    "version": -1,
                })
            })
        })
        .collect::<Vec<_>>();
    post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({ "requests": requests }),
    )
}

fn merge_authoritative_collections(
    response: &mut Value,
    mut hydrated: Value,
    missing: &BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    let record_map = hydrated
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in hydrated Notion collections".to_string())?;
    let omissions = normalize_hydrated_collections(record_map, missing)?;
    apply_collection_omissions(response, record_map, omissions)?;
    let incoming = std::mem::take(record_map);
    let response_record_map = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in Notion page response".to_string())?;
    merge_record_map(response_record_map, incoming)
}

fn normalize_hydrated_collections(
    record_map: &mut Map<String, Value>,
    missing: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, AuthoritativeCollectionState>, String> {
    let collections = record_map
        .entry("collection".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            "recordMap.collection in hydrated Notion collections is not an object".to_string()
        })?;
    let mut omissions = BTreeMap::new();
    for collection_id in missing.values().flatten() {
        collections
            .entry(collection_id.clone())
            .or_insert_with(|| json!({ "role": "none" }));
        let entry = collections
            .get_mut(collection_id)
            .expect("normalized Notion collection target must exist");
        let omitted = normalize_authoritative_collection(entry, collection_id)?;
        omissions.insert(collection_id.clone(), omitted);
    }
    Ok(omissions)
}

fn normalize_authoritative_collection(
    entry: &mut Value,
    collection_id: &str,
) -> Result<AuthoritativeCollectionState, String> {
    if matches!(
        record_value_state(Some(entry)),
        RecordValueState::Unavailable
    ) {
        return Ok(AuthoritativeCollectionState::Unavailable);
    }
    let value = record_value_mut(entry)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("authoritative Notion collection {collection_id} has no value"))?;
    let omissions = CollectionOmissions {
        name: normalize_array(value, collection_id, "name")?,
        schema: normalize_object(value, collection_id, "schema")?,
        format: normalize_object(value, collection_id, "format")?,
    };
    if !matches!(
        collection_record_state(entry, collection_id)?,
        CollectionRecordState::Complete
    ) {
        return Err(format!(
            "authoritative Notion collection {collection_id} is incomplete"
        ));
    }
    Ok(AuthoritativeCollectionState::Present(omissions))
}

fn apply_collection_omissions(
    response: &mut Value,
    authoritative: &Map<String, Value>,
    outcomes: BTreeMap<String, AuthoritativeCollectionState>,
) -> Result<(), String> {
    let Some(existing) = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .and_then(|record_map| record_map.get_mut("collection"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    let authoritative = authoritative
        .get("collection")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing authoritative Notion collection table".to_string())?;
    for (collection_id, outcome) in outcomes {
        let authoritative_entry = authoritative
            .get(&collection_id)
            .ok_or_else(|| format!("missing authoritative collection {collection_id}"))?;
        match outcome {
            AuthoritativeCollectionState::Unavailable => {
                existing.remove(&collection_id);
            }
            AuthoritativeCollectionState::Present(omitted) => {
                let Some(existing_entry) = existing.get_mut(&collection_id) else {
                    continue;
                };
                apply_collection_omission(
                    existing_entry,
                    authoritative_entry,
                    omitted,
                    &collection_id,
                )?;
            }
        }
    }
    Ok(())
}

fn apply_collection_omission(
    existing: &mut Value,
    authoritative: &Value,
    omitted: CollectionOmissions,
    collection_id: &str,
) -> Result<(), String> {
    if record_version(existing, collection_id)? != record_version(authoritative, collection_id)? {
        return Ok(());
    }
    let Some(existing) = record_value_mut(existing).and_then(Value::as_object_mut) else {
        return Ok(());
    };
    for (field, omitted) in [
        ("name", omitted.name),
        ("schema", omitted.schema),
        ("format", omitted.format),
    ] {
        if omitted {
            existing.remove(field);
        }
    }
    Ok(())
}

fn record_version(entry: &Value, collection_id: &str) -> Result<Option<u64>, String> {
    loaded_record_value(entry)
        .map(|value| {
            value
                .as_object()
                .ok_or_else(|| format!("Notion collection {collection_id} has non-object value"))
                .and_then(|value| optional_u64(value, collection_id, "version"))
        })
        .transpose()
        .map(Option::flatten)
}

fn normalize_object(
    value: &mut Map<String, Value>,
    collection_id: &str,
    field: &str,
) -> Result<bool, String> {
    match value.get(field) {
        Some(Value::Object(_)) => return Ok(false),
        Some(Value::Null) | None => {}
        Some(_) => {
            return Err(format!(
                "authoritative Notion collection {collection_id} contains non-object field {field}"
            ))
        }
    }
    value.insert(field.to_string(), Value::Object(Map::new()));
    Ok(true)
}

fn normalize_array(
    value: &mut Map<String, Value>,
    collection_id: &str,
    field: &str,
) -> Result<bool, String> {
    match value.get(field) {
        Some(Value::Array(_)) => return Ok(false),
        Some(Value::Null) | None => {}
        Some(_) => {
            return Err(format!(
                "authoritative Notion collection {collection_id} contains non-array field {field}"
            ))
        }
    }
    value.insert(field.to_string(), Value::Array(Vec::new()));
    Ok(true)
}

fn required_object_is_present(
    value: &Map<String, Value>,
    collection_id: &str,
    field: &str,
) -> Result<bool, String> {
    required_field_is_present(value, collection_id, field, Value::is_object, "object")
}

fn required_array_is_present(
    value: &Map<String, Value>,
    collection_id: &str,
    field: &str,
) -> Result<bool, String> {
    required_field_is_present(value, collection_id, field, Value::is_array, "array")
}

fn required_field_is_present(
    value: &Map<String, Value>,
    collection_id: &str,
    field: &str,
    predicate: impl FnOnce(&Value) -> bool,
    expected: &str,
) -> Result<bool, String> {
    let Some(value) = value.get(field) else {
        return Ok(false);
    };
    if predicate(value) {
        return Ok(true);
    }
    Err(format!(
        "Notion collection {collection_id} contains non-{expected} field {field}"
    ))
}

fn optional_string<'a>(
    value: &'a Map<String, Value>,
    collection_id: &str,
    field: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = value.get(field) else {
        return Ok(None);
    };
    value.as_str().map(Some).ok_or_else(|| {
        format!("Notion collection {collection_id} contains non-string field {field}")
    })
}

fn optional_u64(
    value: &Map<String, Value>,
    collection_id: &str,
    field: &str,
) -> Result<Option<u64>, String> {
    let Some(value) = value.get(field) else {
        return Ok(None);
    };
    value.as_u64().map(Some).ok_or_else(|| {
        format!("Notion collection {collection_id} contains non-integer field {field}")
    })
}
