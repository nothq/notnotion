use super::optional_string_field;
use crate::live::board::{
    block_collection_pointer, BlockCollectionPointer, CollectionPointer, Value,
};

pub(super) struct PageCollections<'a> {
    pub(super) pointers: Vec<CollectionPointer<'a>>,
    pub(super) linked_view_ids: Vec<&'a str>,
}

pub(super) fn page_collections<'a>(
    value: &'a Value,
    block_id: &str,
    block_type: &str,
    block_space_id: &'a str,
) -> Result<Option<PageCollections<'a>>, String> {
    let mut collections = PageCollections {
        pointers: Vec::new(),
        linked_view_ids: Vec::new(),
    };
    if optional_string_field(value, block_id, "parent_table")? == Some("collection") {
        let Some(parent_id) = optional_string_field(value, block_id, "parent_id")?
            .filter(|parent_id| !parent_id.trim().is_empty())
        else {
            return Ok(None);
        };
        insert_pointer(
            &mut collections.pointers,
            CollectionPointer {
                id: parent_id,
                space_id: block_space_id,
            },
            block_id,
        )?;
    }
    if !matches!(block_type, "collection_view" | "collection_view_page") {
        return Ok(Some(collections));
    }
    match block_collection_pointer(value, block_id, block_space_id)? {
        BlockCollectionPointer::Present(pointer) => {
            insert_pointer(&mut collections.pointers, pointer, block_id)?;
        }
        BlockCollectionPointer::Linked => {
            collections.linked_view_ids = linked_view_ids(value, block_id)?;
        }
        BlockCollectionPointer::Incomplete => return Ok(None),
    }
    Ok(Some(collections))
}

fn linked_view_ids<'a>(value: &'a Value, block_id: &str) -> Result<Vec<&'a str>, String> {
    let Some(view_ids) = value.get("view_ids") else {
        return Ok(Vec::new());
    };
    view_ids
        .as_array()
        .ok_or_else(|| format!("Notion block {block_id} contains non-array field view_ids"))?
        .iter()
        .map(|view_id| {
            view_id
                .as_str()
                .filter(|view_id| !view_id.trim().is_empty())
                .ok_or_else(|| format!("Notion block {block_id} contains an invalid view id"))
        })
        .collect()
}

fn insert_pointer<'a>(
    pointers: &mut Vec<CollectionPointer<'a>>,
    pointer: CollectionPointer<'a>,
    block_id: &str,
) -> Result<(), String> {
    let Some(existing) = pointers.iter().find(|existing| existing.id == pointer.id) else {
        pointers.push(pointer);
        return Ok(());
    };
    if existing.space_id != pointer.space_id {
        return Err(format!(
            "Notion block {block_id} references collection {} in both spaces {} and {}",
            pointer.id, existing.space_id, pointer.space_id
        ));
    }
    Ok(())
}
