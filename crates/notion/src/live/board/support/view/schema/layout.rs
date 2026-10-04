use serde_json::{json, Map, Value};

use crate::model::ViewTabKind;

/// Returns a view's property layout entries in display order, as Notion shows
/// them: a Table view without a stored layout lists every property, List and
/// Gallery views without one list none, and entries naming a property the
/// collection no longer has are left out.
pub(super) fn view_layout_entries(
    view: &Value,
    view_kind: ViewTabKind,
    collection_schema: Option<&Map<String, Value>>,
) -> Result<Vec<Value>, String> {
    let format_key = match view_kind {
        ViewTabKind::Table => "table_properties",
        ViewTabKind::List => "list_properties",
        ViewTabKind::Gallery => "gallery_properties",
        ViewTabKind::Board
        | ViewTabKind::Timeline
        | ViewTabKind::Calendar
        | ViewTabKind::Unknown => {
            return Ok(Vec::new());
        }
    };
    let entries = match view.get("format").and_then(|format| format.get(format_key)) {
        None | Some(Value::Null) => default_layout_entries(view_kind, collection_schema),
        Some(Value::Array(entries)) => entries.clone(),
        Some(_) => return Err(format!("Notion view format.{format_key} must be an array")),
    };
    Ok(entries
        .into_iter()
        .filter(|entry| collection_schema.is_none_or(|schema| property_is_in_schema(entry, schema)))
        .collect())
}

/// Whether an entry that names a property refers to one the collection still
/// has. Entries without a property name are kept so their parsers reject them.
pub(super) fn property_is_in_schema(entry: &Value, schema: &Map<String, Value>) -> bool {
    entry
        .get("property")
        .and_then(Value::as_str)
        .is_none_or(|property_id| schema.contains_key(property_id))
}

fn default_layout_entries(
    view_kind: ViewTabKind,
    collection_schema: Option<&Map<String, Value>>,
) -> Vec<Value> {
    if view_kind != ViewTabKind::Table {
        return Vec::new();
    }
    let others = collection_schema
        .into_iter()
        .flat_map(Map::keys)
        .filter(|property_id| property_id.as_str() != "title");
    std::iter::once("title")
        .chain(others.map(String::as_str))
        .map(|property_id| json!({ "property": property_id, "visible": true }))
        .collect()
}
