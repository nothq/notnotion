use super::property::plain_text_from_property_value;
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const NOTION_ID_EPOCH_MS: u64 = 1_704_067_200_000;
const MILLIS_PER_DAY: u64 = 86_400_000;
const NOTION_ID_DAY_LIMIT: u64 = 1 << 12;
const NOTION_SPACE_SHORT_ID_LIMIT: u64 = 1 << 36;
const NOTION_BLOCK_TABLE_MARKER: u8 = 192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NotionSpaceShortId(u64);

impl TryFrom<&str> for NotionSpaceShortId {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value
            .parse::<u64>()
            .map_err(|error| format!("invalid Notion space short_id_str: {error}"))?;
        if value >= NOTION_SPACE_SHORT_ID_LIMIT {
            return Err(format!(
                "Notion space short ID {value} exceeds the 36-bit ID schema"
            ));
        }
        Ok(Self(value))
    }
}

pub(crate) fn page_property_rank(label: &str) -> usize {
    match label {
        "Status" => 0,
        "Assign" => 1,
        "Blocked by" => 2,
        "Blocking" => 3,
        "Date" => 4,
        "Schedule" => 5,
        "Last Enqueued At" => 6,
        "Generated From" => 7,
        "Generated Runs" => 8,
        _ => 9,
    }
}

pub(crate) fn record_map_table<'a>(
    root: &'a Value,
    table: &str,
) -> Result<&'a Map<String, Value>, String> {
    root.get("recordMap")
        .and_then(|record_map| record_map.get(table))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("missing recordMap.{table}"))
}

pub(crate) fn optional_record_map_table<'a>(
    root: &'a Value,
    table: &str,
) -> Option<&'a Map<String, Value>> {
    root.get("recordMap")
        .and_then(|record_map| record_map.get(table))
        .and_then(Value::as_object)
}

pub(crate) fn unwrap_record_value(entry: &Value) -> Option<&Value> {
    entry
        .get("value")
        .and_then(|value| value.get("value").or(Some(value)))
}

pub(crate) fn loaded_record_value(entry: &Value) -> Option<&Value> {
    if let Some(nested_value) = entry.get("value").and_then(|wrapper| wrapper.get("value")) {
        return (!nested_value.is_null()).then_some(nested_value);
    }
    let candidate = entry.get("value").unwrap_or(entry);
    (candidate.get("id").is_some() || candidate.get("version").is_some()).then_some(candidate)
}

#[derive(Clone, Copy)]
pub(crate) enum RecordValueState<'a> {
    Incomplete,
    Unavailable,
    Present(&'a Value),
}

pub(crate) fn record_value_state(entry: Option<&Value>) -> RecordValueState<'_> {
    let Some(entry) = entry else {
        return RecordValueState::Incomplete;
    };
    let outer_role = entry.get("role").and_then(Value::as_str);
    let wrapped_role = entry
        .get("value")
        .and_then(Value::as_object)
        .and_then(|wrapper| wrapper.get("role"))
        .and_then(Value::as_str);
    if outer_role == Some("none") || wrapped_role == Some("none") {
        return RecordValueState::Unavailable;
    }
    loaded_record_value(entry)
        .map(RecordValueState::Present)
        .unwrap_or(RecordValueState::Incomplete)
}

pub(crate) fn record_is_deleted(value: &Value) -> bool {
    value.get("alive").and_then(Value::as_bool) == Some(false)
}

pub(crate) fn block_value<'a>(
    blocks: &'a Map<String, Value>,
    block_id: &str,
) -> Result<&'a Value, String> {
    let normalized_block_id = normalize_uuid(block_id);
    blocks
        .get(block_id)
        .or_else(|| {
            blocks
                .iter()
                .find(|(candidate_id, _)| normalize_uuid(candidate_id) == normalized_block_id)
                .map(|(_, entry)| entry)
        })
        .and_then(unwrap_record_value)
        .ok_or_else(|| format!("missing block {block_id}"))
}

pub(crate) fn collection_view_entry<'a>(
    views: &'a Map<String, Value>,
    view_id: &str,
) -> Result<&'a Value, String> {
    views
        .get(view_id)
        .and_then(unwrap_record_value)
        .ok_or_else(|| format!("missing collection view {view_id}"))
}

pub(crate) fn collection_entry<'a>(
    collections: &'a Map<String, Value>,
    collection_id: &str,
) -> Result<&'a Value, String> {
    collections
        .get(collection_id)
        .and_then(unwrap_record_value)
        .ok_or_else(|| format!("missing collection {collection_id}"))
}

pub(crate) fn collection_name(collection: &Value) -> Result<Option<String>, String> {
    let Some(name) = collection.get("name") else {
        return Ok(None);
    };
    let name = plain_text_from_property_value(name, None, None)?;
    Ok((!name.trim().is_empty()).then_some(name))
}

pub(crate) fn collection_database_name(collection: &Value) -> Result<String, String> {
    Ok(collection_name(collection)?.unwrap_or_else(|| "New database".to_string()))
}

pub(crate) fn board_page_title(
    blocks: &Map<String, Value>,
    collection_view_block: &Value,
    database_title: &str,
) -> Result<Option<String>, String> {
    if collection_view_block
        .get("parent_table")
        .and_then(Value::as_str)
        == Some("space")
    {
        return Ok(Some(database_title.to_string()));
    }

    let Some(mut parent_id) = collection_view_block
        .get("parent_id")
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    loop {
        let Ok(parent) = block_value(blocks, parent_id) else {
            return Ok(None);
        };
        if let Some(title) = title_property(parent)? {
            return Ok(Some(title));
        }

        if parent.get("parent_table").and_then(Value::as_str) != Some("block") {
            return Ok(None);
        }
        let Some(next_parent_id) = parent.get("parent_id").and_then(Value::as_str) else {
            return Ok(None);
        };
        parent_id = next_parent_id;
    }
}

pub(crate) fn title_property(block: &Value) -> Result<Option<String>, String> {
    Ok(title_property_allow_empty(block)?.filter(|title| !title.trim().is_empty()))
}

pub(crate) fn title_property_allow_empty(block: &Value) -> Result<Option<String>, String> {
    let Some(title) = block
        .get("properties")
        .and_then(|properties| properties.get("title"))
    else {
        return Ok(None);
    };
    plain_text_from_property_value(title, None, None).map(Some)
}

pub(crate) fn notion_block_id(
    space_short_id: NotionSpaceShortId,
    now_ms: u64,
) -> Result<String, String> {
    let offset_in_days = now_ms
        .checked_sub(NOTION_ID_EPOCH_MS)
        .ok_or_else(|| "system time predates the Notion ID epoch".to_string())?
        / MILLIS_PER_DAY;
    if offset_in_days >= NOTION_ID_DAY_LIMIT {
        return Err(format!(
            "Notion ID day offset {offset_in_days} exceeds the 12-bit ID schema"
        ));
    }

    let mut bytes = uuid::Uuid::new_v4().into_bytes();
    bytes[0] = (offset_in_days >> 4) as u8;
    bytes[1] = ((offset_in_days << 4) & 0xf0) as u8 | ((space_short_id.0 >> 32) & 0x0f) as u8;
    bytes[2..6].copy_from_slice(&(space_short_id.0 as u32).to_be_bytes());
    bytes[6] = 0x80;
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    bytes[10] = NOTION_BLOCK_TABLE_MARKER | (bytes[10] & 0x3f);
    Ok(uuid::Uuid::from_bytes(bytes).to_string())
}

pub(crate) fn notion_title_crdt_token(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{seed}/title").as_bytes());
    let digest = hasher.finalize();
    STANDARD_NO_PAD
        .encode(&digest[..16])
        .replace('/', "-")
        .replace('+', "_")
}

pub(crate) fn page_collection_id(block: &Value) -> Option<&str> {
    block
        .get("collection_id")
        .and_then(Value::as_str)
        .or_else(|| {
            block
                .get("format")
                .and_then(Value::as_object)
                .and_then(|format| format.get("collection_pointer"))
                .and_then(Value::as_object)
                .filter(|pointer| {
                    pointer.get("table").and_then(Value::as_str) == Some("collection")
                })
                .and_then(|pointer| pointer.get("id"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            (block.get("parent_table").and_then(Value::as_str) == Some("collection"))
                .then(|| block.get("parent_id").and_then(Value::as_str))
                .flatten()
        })
}

pub(crate) fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string field {key}"))
}

pub(crate) fn required_u64(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing u64 field {key}"))
}

pub(crate) fn required_array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array field {key}"))
}

pub(crate) fn required_string_array(value: &Value, key: &str) -> Result<Vec<String>, String> {
    required_array(value, key)?
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("invalid string field {key}[{index}]"))
        })
        .collect()
}

pub(crate) fn optional_string_array(value: &Value, key: &str) -> Result<Vec<String>, String> {
    match value.get(key) {
        Some(_) => required_string_array(value, key),
        None => Ok(Vec::new()),
    }
}

pub(crate) fn normalize_uuid(value: &str) -> String {
    if value.contains('-') && value.len() == 36 {
        return value.to_string();
    }

    let compact = value
        .rsplit('-')
        .next()
        .filter(|candidate| candidate.len() == 32)
        .unwrap_or(value);

    if compact.len() != 32 || !compact.is_ascii() {
        return value.to_string();
    }

    format!(
        "{}-{}-{}-{}-{}",
        &compact[0..8],
        &compact[8..12],
        &compact[12..16],
        &compact[16..20],
        &compact[20..32]
    )
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value};

    use super::{block_value, normalize_uuid};

    #[test]
    fn normalize_uuid_only_slices_ascii_compact_ids() {
        assert_eq!(
            normalize_uuid("00000000000000000000000000000004"),
            "00000000-0000-0000-0000-000000000004"
        );
        let malformed_unicode = format!("{}é{}", "a".repeat(7), "b".repeat(23));
        assert_eq!(malformed_unicode.len(), 32);
        assert_eq!(normalize_uuid(&malformed_unicode), malformed_unicode);
    }

    #[test]
    fn block_value_accepts_compact_and_dashed_uuid_aliases() {
        let compact_id = "00000000000000000000000000000004";
        let dashed_id = "00000000-0000-0000-0000-000000000004";
        let mut blocks = Map::new();
        blocks.insert(
            compact_id.to_string(),
            serde_json::json!({ "value": { "id": compact_id, "title": "Aliased" } }),
        );

        assert_eq!(
            block_value(&blocks, dashed_id)
                .expect("resolve dashed UUID through compact record key")
                .get("title")
                .and_then(Value::as_str),
            Some("Aliased")
        );
    }
}
