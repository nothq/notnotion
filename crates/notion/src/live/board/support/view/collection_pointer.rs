use serde_json::{Map, Value};

use super::super::record::{collection_view_entry, required_string};
use super::resolve_active_view_id;

#[derive(Clone, Copy)]
pub(crate) struct CollectionPointer<'a> {
    pub(crate) id: &'a str,
    pub(crate) space_id: &'a str,
}

pub(crate) enum BlockCollectionPointer<'a> {
    /// A linked database view names its collection on each view record, not
    /// on the block.
    Linked,
    Incomplete,
    Present(CollectionPointer<'a>),
}

enum FormatCollectionPointer<'a> {
    Absent,
    Incomplete,
    Present(CollectionPointer<'a>),
}

/// Reads the block-level collection pointer of a `collection_view` or
/// `collection_view_page` block. The legacy `collection_id` and
/// `format.collection_pointer` must agree when both are present.
pub(crate) fn block_collection_pointer<'a>(
    block: &'a Value,
    block_id: &str,
    block_space_id: &'a str,
) -> Result<BlockCollectionPointer<'a>, String> {
    let legacy_id = match block.get("collection_id") {
        None => None,
        Some(Value::String(id)) => Some(id.as_str()).filter(|id| !id.trim().is_empty()),
        Some(_) => {
            return Err(format!(
                "Notion block {block_id} contains non-string field collection_id"
            ));
        }
    };
    let format_pointer = format_collection_pointer(block, "collection view block", block_id)?;
    let pointer = match (legacy_id, format_pointer) {
        (Some(legacy_id), FormatCollectionPointer::Present(pointer)) => {
            if legacy_id != pointer.id || pointer.space_id != block_space_id {
                return Err(format!(
                    "Notion collection view block {block_id} contains conflicting legacy pointer {legacy_id} in space {block_space_id} and modern pointer {} in space {}",
                    pointer.id, pointer.space_id
                ));
            }
            pointer
        }
        (Some(id), FormatCollectionPointer::Absent) => CollectionPointer {
            id,
            space_id: block_space_id,
        },
        (None, FormatCollectionPointer::Present(pointer)) => pointer,
        (None, FormatCollectionPointer::Absent) => return Ok(BlockCollectionPointer::Linked),
        (_, FormatCollectionPointer::Incomplete) => {
            return Ok(BlockCollectionPointer::Incomplete);
        }
    };
    Ok(BlockCollectionPointer::Present(pointer))
}

pub(crate) fn view_collection_pointer<'a>(
    view: &'a Value,
    view_id: &str,
) -> Result<Option<CollectionPointer<'a>>, String> {
    match format_collection_pointer(view, "collection view", view_id)? {
        FormatCollectionPointer::Absent => Ok(None),
        FormatCollectionPointer::Incomplete => Err(format!(
            "Notion collection view {view_id} has an incomplete collection pointer"
        )),
        FormatCollectionPointer::Present(pointer) => Ok(Some(pointer)),
    }
}

/// Resolves the collection a database block shows through `view_id`, or through
/// its default view when `view_id` is `None`. A block-level pointer wins; a
/// linked view uses the pointer on the view record.
pub(crate) fn collection_view_collection_id<'a>(
    block: &'a Value,
    collection_views: Option<&'a Map<String, Value>>,
    view_id: Option<&str>,
) -> Result<&'a str, String> {
    let block_id = required_string(block, "id")?;
    match block_collection_pointer(block, block_id, required_string(block, "space_id")?)? {
        BlockCollectionPointer::Present(pointer) => return Ok(pointer.id),
        BlockCollectionPointer::Incomplete => {
            return Err(format!(
                "Notion collection view block {block_id} has an incomplete collection pointer"
            ));
        }
        BlockCollectionPointer::Linked => {}
    }
    let views = collection_views
        .ok_or_else(|| format!("missing collection views for linked Notion database {block_id}"))?;
    let view_id = resolve_active_view_id(block, views, view_id)?;
    view_collection_pointer(collection_view_entry(views, &view_id)?, &view_id)?
        .map(|pointer| pointer.id)
        .ok_or_else(|| {
            format!("linked Notion database {block_id} view {view_id} does not name a collection")
        })
}

fn format_collection_pointer<'a>(
    record: &'a Value,
    kind: &str,
    record_id: &str,
) -> Result<FormatCollectionPointer<'a>, String> {
    let Some(pointer) = record
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("collection_pointer"))
    else {
        return Ok(FormatCollectionPointer::Absent);
    };
    if pointer.is_null() {
        return Ok(FormatCollectionPointer::Absent);
    }
    let pointer = pointer
        .as_object()
        .ok_or_else(|| format!("Notion {kind} {record_id} has an invalid collection pointer"))?;
    match pointer.get("table") {
        Some(Value::String(table)) if table == "collection" => {}
        Some(Value::String(table)) => {
            return Err(format!(
                "Notion {kind} {record_id} points to {table}, not a collection"
            ));
        }
        Some(_) => {
            return Err(format!(
                "Notion {kind} {record_id} has a non-string collection pointer table"
            ));
        }
        None => return Ok(FormatCollectionPointer::Incomplete),
    }
    let Some(id) = pointer_string(pointer.get("id"), kind, record_id, "id")? else {
        return Ok(FormatCollectionPointer::Incomplete);
    };
    let Some(space_id) = pointer_string(pointer.get("spaceId"), kind, record_id, "spaceId")? else {
        return Ok(FormatCollectionPointer::Incomplete);
    };
    Ok(FormatCollectionPointer::Present(CollectionPointer {
        id,
        space_id,
    }))
}

fn pointer_string<'a>(
    value: Option<&'a Value>,
    kind: &str,
    record_id: &str,
    field: &str,
) -> Result<Option<&'a str>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.as_str().ok_or_else(|| {
        format!("Notion {kind} {record_id} has a non-string collection pointer {field}")
    })?;
    if value.trim().is_empty() {
        return Err(format!(
            "Notion {kind} {record_id} has an empty collection pointer {field}"
        ));
    }
    Ok(Some(value))
}
