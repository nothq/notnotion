use serde_json::{json, Map, Value};

pub(super) fn entry(id: &str, version: u64, content: Option<Vec<&str>>) -> Value {
    block_entry(id, version, "page", content, Map::new())
}

pub(super) fn block_entry(
    id: &str,
    version: u64,
    block_type: &str,
    content: Option<Vec<&str>>,
    extra: Map<String, Value>,
) -> Value {
    let mut value = Map::from_iter([
        ("id".to_string(), Value::String(id.to_string())),
        ("version".to_string(), Value::from(version)),
        ("space_id".to_string(), Value::String("space-1".to_string())),
        ("type".to_string(), Value::String(block_type.to_string())),
    ]);
    value.extend(extra);
    if let Some(content) = content {
        value.insert(
            "content".to_string(),
            Value::Array(
                content
                    .into_iter()
                    .map(|id| Value::String(id.to_string()))
                    .collect(),
            ),
        );
    }
    json!({ "role": "editor", "value": { "role": "editor", "value": value } })
}

pub(super) fn block_table(
    entries: impl IntoIterator<Item = (&'static str, Value)>,
) -> Map<String, Value> {
    Map::from_iter([(
        "block".to_string(),
        Value::Object(
            entries
                .into_iter()
                .map(|(id, entry)| (id.to_string(), entry))
                .collect(),
        ),
    )])
}
