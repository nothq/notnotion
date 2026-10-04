use std::{collections::VecDeque, sync::Arc};

use serde_json::{Map, Value};

use super::super::{
    loaded_record_value, normalize_uuid, record_is_deleted, record_map::merge_record_map,
    record_value_state, RecordValueState,
};
use super::preview::record_by_id;
use super::{
    HydratedPreviewRoot, QuickFindRecordState, QuickFindUserRecordState, RecordKey,
    QUICK_FIND_RECORD_LIMIT,
};

pub(super) fn merge_quick_find_record_map(
    state: &mut QuickFindRecordState,
    record_map: Map<String, Value>,
) -> Result<(), String> {
    let (mut record_map, touched) = object_record_tables(record_map);
    let notion_users = record_map.remove("notion_user");
    merge_record_map(Arc::make_mut(&mut state.record_map), record_map)?;
    if let Some(Value::Object(notion_users)) = notion_users {
        merge_quick_find_notion_users(Arc::make_mut(&mut state.record_map), notion_users)?;
    }
    for key in touched {
        touch_record(&mut state.recency, key);
    }
    evict_excess_records(state);
    Ok(())
}

pub(super) fn preview_root_hydration(
    record_map: &Map<String, Value>,
    block_id: &str,
) -> Result<Option<HydratedPreviewRoot>, String> {
    let Some((_, entry)) = record_by_id(record_map, "block", block_id) else {
        return Ok(None);
    };
    let root = match record_value_state(Some(entry)) {
        RecordValueState::Present(root) if !record_is_deleted(root) => root,
        RecordValueState::Incomplete
        | RecordValueState::Unavailable
        | RecordValueState::Present(_) => return Ok(None),
    };
    let version = root
        .get("version")
        .map(|version| {
            version.as_u64().ok_or_else(|| {
                format!("Notion block record {block_id} contains a non-integer version")
            })
        })
        .transpose()?;
    Ok(Some(HydratedPreviewRoot { version }))
}

fn merge_quick_find_notion_users(
    record_map: &mut Map<String, Value>,
    incoming_users: Map<String, Value>,
) -> Result<(), String> {
    let users = record_map
        .entry("notion_user".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "merged Notion recordMap.notion_user is not an object".to_string())?;
    for (user_id, incoming) in incoming_users {
        let Some(existing) = users.get_mut(&user_id) else {
            users.insert(user_id, incoming);
            continue;
        };
        let existing_state = quick_find_user_record_state(existing, &user_id)?;
        let incoming_state = quick_find_user_record_state(&incoming, &user_id)?;
        let replace = match (existing_state, incoming_state) {
            (QuickFindUserRecordState::RoleOnly, QuickFindUserRecordState::Value { .. }) => true,
            (QuickFindUserRecordState::Value { .. }, QuickFindUserRecordState::RoleOnly) => false,
            (QuickFindUserRecordState::RoleOnly, QuickFindUserRecordState::RoleOnly) => true,
            (
                QuickFindUserRecordState::Value {
                    version: existing_version,
                },
                QuickFindUserRecordState::Value {
                    version: incoming_version,
                },
            ) => incoming_version >= existing_version,
        };
        if replace {
            *existing = incoming;
        }
    }
    Ok(())
}

fn quick_find_user_record_state(
    entry: &Value,
    user_id: &str,
) -> Result<QuickFindUserRecordState, String> {
    let Some(value) = loaded_record_value(entry) else {
        return Ok(QuickFindUserRecordState::RoleOnly);
    };
    let value = value.as_object().ok_or_else(|| {
        format!("Notion notion_user record {user_id} contains a non-object value")
    })?;
    let version = value
        .get("version")
        .map(|version| {
            version.as_u64().ok_or_else(|| {
                format!("Notion notion_user record {user_id} contains a non-integer version")
            })
        })
        .transpose()?;
    Ok(QuickFindUserRecordState::Value { version })
}

/// Object-valued record tables with the keys of every record they contain.
type ObjectRecordTables = (Map<String, Value>, Vec<RecordKey>);

fn object_record_tables(record_map: Map<String, Value>) -> ObjectRecordTables {
    let mut tables = Map::new();
    let mut touched = Vec::new();
    for (table, records) in record_map {
        if table.starts_with("__") {
            continue;
        }
        let Value::Object(records) = records else {
            continue;
        };
        touched.extend(records.keys().map(|id| RecordKey {
            table: table.clone(),
            id: id.clone(),
        }));
        tables.insert(table, Value::Object(records));
    }
    (tables, touched)
}

pub(super) fn touch_record(recency: &mut VecDeque<RecordKey>, key: RecordKey) {
    recency.retain(|candidate| candidate != &key);
    recency.push_back(key);
}

fn evict_excess_records(state: &mut QuickFindRecordState) {
    while state.recency.len() > QUICK_FIND_RECORD_LIMIT {
        let Some(key) = state.recency.pop_front() else {
            break;
        };
        let removed = Arc::make_mut(&mut state.record_map)
            .get_mut(&key.table)
            .and_then(Value::as_object_mut)
            .and_then(|records| records.remove(&key.id))
            .is_some();
        if removed && key.table == "block" {
            state
                .hydrated_preview_roots
                .remove(&normalize_uuid(&key.id));
        }
    }
    Arc::make_mut(&mut state.record_map).retain(|_, records| {
        records
            .as_object()
            .is_some_and(|records| !records.is_empty())
    });
}
