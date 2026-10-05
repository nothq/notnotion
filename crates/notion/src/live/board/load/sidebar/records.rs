use super::super::super::{
    loaded_record_value, normalize_uuid, record_map::merge_record_table, record_value_state,
    unwrap_record_value, Map, RecordValueState, Value,
};

pub(super) fn is_available_sidebar_root(blocks: &Map<String, Value>, block_id: &str) -> bool {
    sidebar_record_entry_by_id(blocks, block_id)
        .and_then(loaded_record_value)
        .is_some_and(|block| {
            block.get("alive").and_then(Value::as_bool) == Some(true)
                && matches!(
                    block.get("type").and_then(Value::as_str),
                    Some("page" | "transcription" | "collection_view" | "collection_view_page")
                )
        })
}

pub(super) fn is_resolved_sidebar_block(blocks: &Map<String, Value>, block_id: &str) -> bool {
    let entry = sidebar_record_entry_by_id(blocks, block_id);
    !matches!(record_value_state(entry), RecordValueState::Incomplete)
}

fn sidebar_record_entry_by_id<'a>(
    records: &'a Map<String, Value>,
    record_id: &str,
) -> Option<&'a Value> {
    let normalized_record_id = normalize_uuid(record_id);
    records.get(record_id).or_else(|| {
        records
            .iter()
            .find(|(candidate_id, _)| normalize_uuid(candidate_id) == normalized_record_id)
            .map(|(_, entry)| entry)
    })
}

pub(in crate::live::board::load) fn combined_block_records<'a>(
    sources: impl IntoIterator<Item = &'a Map<String, Value>>,
) -> Result<Map<String, Value>, String> {
    let mut blocks = Map::new();
    for source in sources {
        merge_record_table("block", &mut blocks, source.clone())?;
    }
    Ok(blocks)
}

pub(super) fn record_value<'a>(
    records: &'a Map<String, Value>,
    record_id: &str,
    table: &str,
) -> Result<&'a Value, String> {
    records
        .get(record_id)
        .and_then(unwrap_record_value)
        .ok_or_else(|| format!("missing Notion {table} record {record_id}"))
}

pub(super) fn required_bool(value: &Value, key: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing bool field {key}"))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Map};

    use super::{is_available_sidebar_root, is_resolved_sidebar_block};

    #[test]
    fn resolved_sidebar_blocks_accept_uuid_aliases_and_unavailable_records() {
        let compact_id = "00000000000000000000000000000004";
        let dashed_id = "00000000-0000-0000-0000-000000000004";
        let unavailable_id = "00000000-0000-0000-0000-000000000005";
        let mut blocks = Map::new();
        blocks.insert(
            compact_id.to_string(),
            json!({ "role": "reader", "value": { "id": compact_id, "version": 1 } }),
        );
        blocks.insert(unavailable_id.to_string(), json!({ "role": "none" }));

        assert!(is_resolved_sidebar_block(&blocks, dashed_id));
        assert!(!is_available_sidebar_root(&blocks, dashed_id));
        blocks.insert(
            compact_id.to_string(),
            json!({
                "role": "reader",
                "value": { "id": compact_id, "version": 1, "alive": true, "type": "page" }
            }),
        );
        assert!(is_available_sidebar_root(&blocks, dashed_id));
        assert!(is_resolved_sidebar_block(&blocks, unavailable_id));
        assert!(!is_resolved_sidebar_block(
            &blocks,
            "00000000-0000-0000-0000-000000000006"
        ));
    }
}
