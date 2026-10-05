use std::collections::HashSet;

use serde_json::{Map, Value};

use super::super::{loaded_record_value, normalize_uuid, page_collection_id};
use super::{
    InitialSyncRecordPointer, InitialSyncRecordRequest, PreviewRecordClosure, PreviewRecordKey,
    QUICK_FIND_PREVIEW_REQUEST_LIMIT,
};

pub(super) fn record_pointer(table: &str, id: &str, space_id: &str) -> InitialSyncRecordPointer {
    InitialSyncRecordPointer {
        table: table.to_string(),
        id: id.to_string(),
        space_id: space_id.to_string(),
    }
}

pub(super) fn preview_record_closure(
    record_map: &Map<String, Value>,
    space_id: &str,
    root_id: &str,
) -> Result<Vec<InitialSyncRecordPointer>, String> {
    let Some(root_block) = cached_record(record_map, "block", root_id) else {
        return Err(format!(
            "missing cached Notion Quick Find preview root {root_id}"
        ));
    };
    let root = record_pointer("block", root_id, space_id);
    let mut closure = PreviewRecordClosure::new(&root);
    closure.append_content(record_map, root_block, space_id)?;
    for pointer in closure.pointers.clone() {
        closure.append_block_dependencies(record_map, &pointer, false)?;
    }
    closure.append_block_dependencies(record_map, &root, true)?;
    Ok(closure.pointers)
}

impl PreviewRecordClosure {
    fn new(root: &InitialSyncRecordPointer) -> Self {
        Self {
            pointers: Vec::new(),
            seen: HashSet::from([preview_record_key(root)]),
        }
    }

    fn append_pointer(&mut self, pointer: InitialSyncRecordPointer) -> bool {
        let key = preview_record_key(&pointer);
        if self.seen.contains(&key) {
            return false;
        }
        self.seen.insert(key);
        self.pointers.push(pointer);
        true
    }

    fn append_content(
        &mut self,
        record_map: &Map<String, Value>,
        parent: &Value,
        parent_space_id: &str,
    ) -> Result<(), String> {
        let Some(content) = parent.get("content").and_then(Value::as_array) else {
            return Ok(());
        };
        for child_id in content.iter().filter_map(Value::as_str) {
            let pointer = record_pointer("block", child_id, parent_space_id);
            if !self.append_pointer(pointer.clone()) {
                continue;
            }
            if let Some(child) = cached_record(record_map, "block", child_id) {
                if preview_block_allows_content_traversal(child) {
                    let child_space_id = child
                        .get("space_id")
                        .and_then(Value::as_str)
                        .unwrap_or(&pointer.space_id);
                    self.append_content(record_map, child, child_space_id)?;
                }
            }
        }
        Ok(())
    }

    fn append_block_dependencies(
        &mut self,
        record_map: &Map<String, Value>,
        pointer: &InitialSyncRecordPointer,
        is_root: bool,
    ) -> Result<(), String> {
        let Some(block) = cached_record(record_map, "block", &pointer.id) else {
            return Ok(());
        };
        let block_space_id = block
            .get("space_id")
            .and_then(Value::as_str)
            .unwrap_or(&pointer.space_id);
        let block_type = block.get("type").and_then(Value::as_str);
        if let Some(reference) = preview_block_reference_pointer(block) {
            self.append_reference(record_map, reference)?;
        }
        let property_source = if is_root || block_type == Some("table_row") {
            block.get("properties")
        } else {
            block
                .get("properties")
                .and_then(Value::as_object)
                .and_then(|properties| properties.get("title"))
        };
        if let Some(property_source) = property_source {
            self.append_property_references(record_map, property_source, block_space_id)?;
        }

        if is_root || matches!(block_type, Some("collection_view" | "collection_view_page")) {
            if let Some(collection) = preview_collection_pointer(block, block_space_id) {
                self.append_pointer(collection);
            }
        }
        Ok(())
    }

    fn append_reference(
        &mut self,
        record_map: &Map<String, Value>,
        pointer: InitialSyncRecordPointer,
    ) -> Result<(), String> {
        if self.append_pointer(pointer.clone()) {
            self.append_block_dependencies(record_map, &pointer, false)?;
        }
        Ok(())
    }

    fn append_property_references(
        &mut self,
        record_map: &Map<String, Value>,
        value: &Value,
        space_id: &str,
    ) -> Result<(), String> {
        match value {
            Value::Array(values) => {
                if values.first().and_then(Value::as_str) == Some("p") {
                    if let Some(id) = values
                        .get(1)
                        .and_then(Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                    {
                        self.append_reference(record_map, record_pointer("block", id, space_id))?;
                        return Ok(());
                    }
                }
                for value in values {
                    self.append_property_references(record_map, value, space_id)?;
                }
            }
            Value::Object(values) => {
                for value in values.values() {
                    self.append_property_references(record_map, value, space_id)?;
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
        Ok(())
    }
}

fn preview_block_allows_content_traversal(block: &Value) -> bool {
    !matches!(
        block.get("type").and_then(Value::as_str),
        None | Some(
            "page"
                | "link_to_page"
                | "alias"
                | "code"
                | "divider"
                | "collection_view"
                | "collection_view_page"
                | "image"
                | "table_row"
        )
    )
}

fn preview_block_reference_pointer(block: &Value) -> Option<InitialSyncRecordPointer> {
    let pointer_key = match block.get("type").and_then(Value::as_str) {
        Some("alias") => "alias_pointer",
        Some("link_to_page") => "page_pointer",
        Some("transclusion_reference") => "transclusion_reference_pointer",
        _ => return None,
    };
    let pointer = block
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get(pointer_key))
        .and_then(Value::as_object)?;
    (pointer.get("table").and_then(Value::as_str) == Some("block")).then_some(())?;
    let id = pointer
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())?;
    let space_id = pointer
        .get("spaceId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())?;
    Some(record_pointer("block", id, space_id))
}

fn preview_collection_pointer(
    block: &Value,
    block_space_id: &str,
) -> Option<InitialSyncRecordPointer> {
    let collection_id = page_collection_id(block)?.trim();
    if collection_id.is_empty() {
        return None;
    }
    let pointer_space_id = block
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("collection_pointer"))
        .and_then(Value::as_object)
        .filter(|pointer| pointer.get("id").and_then(Value::as_str) == Some(collection_id))
        .and_then(|pointer| pointer.get("spaceId"))
        .and_then(Value::as_str)
        .filter(|space_id| !space_id.trim().is_empty())
        .unwrap_or(block_space_id);
    Some(record_pointer(
        "collection",
        collection_id,
        pointer_space_id,
    ))
}

pub(super) fn preview_requests(
    record_map: &Map<String, Value>,
    preview_records: &[InitialSyncRecordPointer],
    requested: &HashSet<PreviewRecordKey>,
) -> Vec<InitialSyncRecordRequest> {
    preview_records
        .iter()
        .filter(|pointer| pointer.table == "block")
        .filter(|pointer| !requested.contains(&preview_record_key(pointer)))
        .take(QUICK_FIND_PREVIEW_REQUEST_LIMIT)
        .map(|pointer| InitialSyncRecordRequest {
            pointer: pointer.clone(),
            version: cached_record(record_map, &pointer.table, &pointer.id)
                .and_then(|record| record.get("version"))
                .and_then(Value::as_u64)
                .and_then(|version| i64::try_from(version).ok())
                .unwrap_or(-1),
        })
        .collect()
}

pub(super) fn preview_record_key(pointer: &InitialSyncRecordPointer) -> PreviewRecordKey {
    (
        pointer.table.clone(),
        normalize_uuid(&pointer.id),
        normalize_uuid(&pointer.space_id),
    )
}

pub(super) fn cached_record<'a>(
    record_map: &'a Map<String, Value>,
    table: &str,
    record_id: &str,
) -> Option<&'a Value> {
    record_by_id(record_map, table, record_id).and_then(|(_, entry)| loaded_record_value(entry))
}

pub(super) fn record_by_id<'a>(
    record_map: &'a Map<String, Value>,
    table: &str,
    record_id: &str,
) -> Option<(&'a str, &'a Value)> {
    let records = record_map.get(table)?.as_object()?;
    if let Some((id, entry)) = records.get_key_value(record_id) {
        return Some((id.as_str(), entry));
    }
    let normalized_id = normalize_uuid(record_id);
    records
        .iter()
        .find(|(id, _)| normalize_uuid(id) == normalized_id)
        .map(|(id, entry)| (id.as_str(), entry))
}
