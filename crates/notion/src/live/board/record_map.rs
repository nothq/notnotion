use std::cmp::Ordering;

use super::{loaded_record_value, Map, Value};

#[derive(Clone, Copy)]
enum RecordEntryState {
    RoleOnly,
    Value { version: Option<u64> },
}

struct RecordEntryParts {
    value: Value,
    metadata: Map<String, Value>,
}

pub(in crate::live::board) fn merge_record_map(
    merged_record_map: &mut Map<String, Value>,
    incoming_record_map: Map<String, Value>,
) -> Result<(), String> {
    for (table, incoming_table) in incoming_record_map {
        let incoming_records = match incoming_table {
            Value::Object(incoming_records) => incoming_records,
            incoming_metadata => {
                merge_record_map_metadata(merged_record_map, &table, incoming_metadata)?;
                continue;
            }
        };
        let merged_table = merged_record_map
            .entry(table.clone())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| format!("merged Notion recordMap.{table} is not an object"))?;
        merge_record_table(&table, merged_table, incoming_records)?;
    }
    Ok(())
}

pub(in crate::live::board) fn merge_record_table(
    table: &str,
    merged_table: &mut Map<String, Value>,
    incoming_records: Map<String, Value>,
) -> Result<(), String> {
    for (record_id, incoming_entry) in incoming_records {
        let Some(existing_entry) = merged_table.get_mut(&record_id) else {
            merged_table.insert(record_id, incoming_entry);
            continue;
        };
        merge_record_entry(table, &record_id, existing_entry, incoming_entry)?;
    }
    Ok(())
}

fn merge_record_map_metadata(
    merged_record_map: &mut Map<String, Value>,
    table: &str,
    incoming_metadata: Value,
) -> Result<(), String> {
    match merged_record_map.get_mut(table) {
        Some(existing_metadata) => merge_json_value(
            existing_metadata,
            incoming_metadata,
            &format!("recordMap.{table}"),
        ),
        None => {
            merged_record_map.insert(table.to_string(), incoming_metadata);
            Ok(())
        }
    }
}

fn merge_record_entry(
    table: &str,
    record_id: &str,
    existing_entry: &mut Value,
    incoming_entry: Value,
) -> Result<(), String> {
    if *existing_entry == incoming_entry {
        return Ok(());
    }
    let existing_state = record_entry_state(existing_entry, table, record_id)?;
    let incoming_state = record_entry_state(&incoming_entry, table, record_id)?;
    let path = format!("recordMap.{table}.{record_id}");
    match (existing_state, incoming_state) {
        (RecordEntryState::RoleOnly, _) | (_, RecordEntryState::RoleOnly) => {
            merge_json_value(existing_entry, incoming_entry, &path)
        }
        (
            RecordEntryState::Value {
                version: existing_version,
            },
            RecordEntryState::Value {
                version: incoming_version,
            },
        ) => merge_versioned_record_entry(
            existing_entry,
            incoming_entry,
            existing_version,
            incoming_version,
            &path,
        ),
    }
}

fn merge_versioned_record_entry(
    existing_entry: &mut Value,
    incoming_entry: Value,
    existing_version: Option<u64>,
    incoming_version: Option<u64>,
    path: &str,
) -> Result<(), String> {
    match (existing_version, incoming_version) {
        (Some(existing), Some(incoming)) => match incoming.cmp(&existing) {
            Ordering::Greater => *existing_entry = incoming_entry,
            Ordering::Less => {}
            Ordering::Equal => merge_same_version_entry(existing_entry, incoming_entry, path)?,
        },
        (None, Some(_)) => *existing_entry = incoming_entry,
        (Some(_), None) => {}
        (None, None) => merge_same_version_entry(existing_entry, incoming_entry, path)?,
    }
    Ok(())
}

fn merge_same_version_entry(
    existing_entry: &mut Value,
    incoming_entry: Value,
    path: &str,
) -> Result<(), String> {
    let mut existing = record_entry_parts(existing_entry, path)?;
    let incoming = record_entry_parts(&incoming_entry, path)?;
    merge_json_value(
        &mut existing.value,
        incoming.value,
        &format!("{path}.value"),
    )?;
    let mut metadata = Value::Object(existing.metadata);
    merge_json_value(
        &mut metadata,
        Value::Object(incoming.metadata),
        &format!("{path}.metadata"),
    )?;
    let Value::Object(mut metadata) = metadata else {
        unreachable!("Notion record metadata merge retains an object");
    };
    metadata.insert("value".to_string(), existing.value);
    *existing_entry = Value::Object(metadata);
    Ok(())
}

fn record_entry_parts(entry: &Value, path: &str) -> Result<RecordEntryParts, String> {
    let Some(entry_object) = entry.as_object() else {
        return Err(format!("Notion record entry at {path} is not an object"));
    };
    let Some(wrapped) = entry_object.get("value") else {
        return Ok(RecordEntryParts {
            value: entry.clone(),
            metadata: Map::new(),
        });
    };
    let mut metadata = entry_object.clone();
    metadata.remove("value");
    let Some(nested) = wrapped.as_object().and_then(|wrapper| wrapper.get("value")) else {
        return Ok(RecordEntryParts {
            value: wrapped.clone(),
            metadata,
        });
    };
    let mut inner_metadata = wrapped
        .as_object()
        .expect("nested Notion record wrapper is an object")
        .clone();
    inner_metadata.remove("value");
    let mut metadata = Value::Object(metadata);
    merge_json_value(
        &mut metadata,
        Value::Object(inner_metadata),
        &format!("{path}.metadata"),
    )?;
    let Value::Object(metadata) = metadata else {
        unreachable!("Notion record metadata merge retains an object");
    };
    Ok(RecordEntryParts {
        value: nested.clone(),
        metadata,
    })
}

pub(in crate::live::board) fn record_value_mut(entry: &mut Value) -> Option<&mut Value> {
    let has_nested_value = entry
        .get("value")
        .and_then(Value::as_object)
        .is_some_and(|wrapper| wrapper.contains_key("value"));
    if has_nested_value {
        return entry.get_mut("value")?.get_mut("value");
    }
    if entry.get("value").is_some() {
        return entry.get_mut("value");
    }
    Some(entry)
}

fn record_entry_state(
    entry: &Value,
    table: &str,
    record_id: &str,
) -> Result<RecordEntryState, String> {
    let Some(value) = loaded_record_value(entry) else {
        return Ok(RecordEntryState::RoleOnly);
    };
    let value = value
        .as_object()
        .ok_or_else(|| format!("Notion {table} record {record_id} contains a non-object value"))?;
    let version = match value.get("version") {
        Some(version) => Some(version.as_u64().ok_or_else(|| {
            format!("Notion {table} record {record_id} contains a non-integer version")
        })?),
        None => None,
    };
    Ok(RecordEntryState::Value { version })
}

fn merge_json_value(existing: &mut Value, incoming: Value, path: &str) -> Result<(), String> {
    if *existing == incoming {
        return Ok(());
    }
    if existing.is_null() {
        *existing = incoming;
        return Ok(());
    }
    if incoming.is_null() {
        return Ok(());
    }
    let (Value::Object(existing), Value::Object(incoming)) = (existing, incoming) else {
        return Err(format!("conflicting Notion record field {path}"));
    };
    for (field, incoming_value) in incoming {
        let field_path = format!("{path}.{field}");
        match existing.get_mut(&field) {
            Some(existing_value) => {
                merge_json_value(existing_value, incoming_value, &field_path)?;
            }
            None => {
                existing.insert(field, incoming_value);
            }
        }
    }
    Ok(())
}
