use crate::live::board::{Map, Value};

pub(super) fn known_object_field(
    value: &Value,
    block_id: &str,
    field: &str,
) -> Result<bool, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Object(_)) => Ok(true),
        Some(_) => Err(format!(
            "Notion block {block_id} contains non-object field {field}"
        )),
    }
}

pub(super) fn optional_string_field<'a>(
    value: &'a Value,
    block_id: &str,
    field: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = value.get(field) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| format!("Notion block {block_id} contains non-string field {field}"))
}

pub(super) fn optional_u64_field(
    value: &Value,
    block_id: &str,
    field: &str,
) -> Result<Option<u64>, String> {
    let Some(value) = value.get(field) else {
        return Ok(None);
    };
    value
        .as_u64()
        .map(Some)
        .ok_or_else(|| format!("Notion block {block_id} contains non-integer field {field}"))
}

pub(super) fn optional_bool_field(
    value: &Value,
    block_id: &str,
    field: &str,
) -> Result<Option<bool>, String> {
    let Some(value) = value.get(field) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| format!("Notion block {block_id} contains non-boolean field {field}"))
}

pub(super) fn nonempty_pointer_field<'a>(
    pointer: &'a Map<String, Value>,
    field: &str,
) -> Option<&'a str> {
    pointer
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}
