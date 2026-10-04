use crate::live::board::{
    record_is_deleted, record_value_state, CollectionPointer, Map, RecordValueState, Value,
};

mod authoritative;
mod collections;
mod fields;
mod references;
mod shape;

use collections::{page_collections, PageCollections};
use fields::{
    known_object_field, nonempty_pointer_field, optional_bool_field, optional_string_field,
    optional_u64_field,
};
use references::{property_references, PropertyReferenceScope};
use shape::{block_requires_known_content, block_requires_known_properties};

pub(super) use authoritative::{
    apply_authoritative_page_block_omissions, missing_authoritative_page_block,
    normalize_authoritative_page_block_entry, unavailable_page_block_kind,
    AuthoritativePageBlockState, AuthoritativePageBlockUnavailable,
};

#[derive(Clone, Copy)]
pub(super) enum PageBlockRequirement {
    Root,
    ContentChild,
    ReferenceTarget,
}

impl PageBlockRequirement {
    fn requires_content(self, block_type: &str) -> bool {
        !matches!(self, Self::ReferenceTarget) && block_requires_known_content(block_type)
    }

    fn property_reference_scope(self) -> PropertyReferenceScope {
        match self {
            Self::Root | Self::ContentChild => PropertyReferenceScope::All,
            Self::ReferenceTarget => PropertyReferenceScope::Title,
        }
    }
}

pub(super) enum PageBlockRecordState<'a> {
    NoValue,
    Unavailable,
    Incomplete,
    Complete(Box<CompletePageBlock<'a>>),
}

pub(super) struct CompletePageBlock<'a> {
    value: &'a Value,
    block_type: &'a str,
    space_id: &'a str,
    parent_id: &'a str,
    parent_table: &'a str,
    content_ids: Vec<&'a str>,
    reference: Option<PageBlockPointer<'a>>,
    collections: PageCollections<'a>,
    property_block_ids: Vec<&'a str>,
    property_user_ids: Vec<&'a str>,
}

#[derive(Clone, Copy)]
pub(super) struct PageBlockPointer<'a> {
    pub(super) id: &'a str,
    pub(super) space_id: &'a str,
}

struct PageBlockBase<'a> {
    block_type: &'a str,
    space_id: &'a str,
    parent_id: &'a str,
    parent_table: &'a str,
}

enum PageBlockReference<'a> {
    None,
    Block(PageBlockPointer<'a>),
}

impl<'a> CompletePageBlock<'a> {
    pub(super) fn block_type(&self) -> &'a str {
        self.block_type
    }

    pub(super) fn space_id(&self) -> &'a str {
        self.space_id
    }

    pub(super) fn parent_id(&self) -> &'a str {
        self.parent_id
    }

    pub(super) fn parent_table(&self) -> &'a str {
        self.parent_table
    }

    pub(super) fn content_ids(&self) -> &[&'a str] {
        &self.content_ids
    }

    pub(super) fn reference(&self) -> Option<PageBlockPointer<'a>> {
        self.reference
    }

    pub(super) fn collection_pointers(&self) -> &[CollectionPointer<'a>] {
        &self.collections.pointers
    }

    pub(super) fn linked_view_ids(&self) -> &[&'a str] {
        &self.collections.linked_view_ids
    }

    pub(super) fn property_block_ids(&self) -> &[&'a str] {
        &self.property_block_ids
    }

    pub(super) fn property_user_ids(&self) -> &[&'a str] {
        &self.property_user_ids
    }

    pub(super) fn last_editor_id(&self) -> Option<&'a str> {
        (self
            .value
            .get("last_edited_by_table")
            .and_then(Value::as_str)
            == Some("notion_user"))
        .then(|| self.value.get("last_edited_by_id").and_then(Value::as_str))
        .flatten()
    }
}

pub(super) fn page_block_record_state<'a>(
    blocks: &'a Map<String, Value>,
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<PageBlockRecordState<'a>, String> {
    let value = match record_value_state(blocks.get(block_id)) {
        RecordValueState::Incomplete => return Ok(PageBlockRecordState::NoValue),
        RecordValueState::Unavailable => return Ok(PageBlockRecordState::Unavailable),
        RecordValueState::Present(value) if record_is_deleted(value) => {
            return Ok(PageBlockRecordState::Unavailable);
        }
        RecordValueState::Present(value) => value,
    };
    parse_page_block_value(value, block_id, requirement)
}

fn parse_page_block_value<'a>(
    value: &'a Value,
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<PageBlockRecordState<'a>, String> {
    let Some(fields) = parse_page_block_fields(value, block_id, requirement)? else {
        return Ok(PageBlockRecordState::Incomplete);
    };
    Ok(PageBlockRecordState::Complete(Box::new(
        CompletePageBlock {
            value,
            block_type: fields.base.block_type,
            space_id: fields.base.space_id,
            parent_id: fields.base.parent_id,
            parent_table: fields.base.parent_table,
            content_ids: fields.content_ids,
            reference: match fields.reference {
                PageBlockReference::None => None,
                PageBlockReference::Block(pointer) => Some(pointer),
            },
            collections: fields.collections,
            property_block_ids: fields.property_block_ids,
            property_user_ids: fields.property_user_ids,
        },
    )))
}

struct PageBlockFields<'a> {
    base: PageBlockBase<'a>,
    content_ids: Vec<&'a str>,
    reference: PageBlockReference<'a>,
    collections: PageCollections<'a>,
    property_block_ids: Vec<&'a str>,
    property_user_ids: Vec<&'a str>,
}

fn parse_page_block_fields<'a>(
    value: &'a Value,
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<Option<PageBlockFields<'a>>, String> {
    let Some(base) = parse_page_block_base(value, block_id)? else {
        return Ok(None);
    };
    if matches!(requirement, PageBlockRequirement::Root)
        && optional_u64_field(value, block_id, "last_edited_time")?.is_none()
    {
        return Ok(None);
    }
    if !last_editor_fields_are_complete(value) {
        return Ok(None);
    }
    if !known_object_field(value, block_id, "format")?
        || (block_requires_known_properties(base.block_type)
            && !known_object_field(value, block_id, "properties")?)
    {
        return Ok(None);
    }
    let Some(content_ids) =
        parse_page_block_content(value, block_id, base.block_type, requirement)?
    else {
        return Ok(None);
    };
    let Some(reference) = parse_page_block_reference(value, block_id, base.block_type)? else {
        return Ok(None);
    };
    let Some(collections) = page_collections(value, block_id, base.block_type, base.space_id)?
    else {
        return Ok(None);
    };
    let property_references =
        property_references(value, block_id, requirement.property_reference_scope())?;
    Ok(Some(PageBlockFields {
        base,
        content_ids,
        reference,
        collections,
        property_block_ids: property_references.block_ids,
        property_user_ids: property_references.user_ids,
    }))
}

fn parse_page_block_base<'a>(
    value: &'a Value,
    block_id: &str,
) -> Result<Option<PageBlockBase<'a>>, String> {
    let Some(record_id) = optional_string_field(value, block_id, "id")? else {
        return Ok(None);
    };
    if record_id != block_id {
        return Err(format!(
            "Notion record key {block_id} contains block {record_id}"
        ));
    }
    let Some(_) = optional_u64_field(value, block_id, "version")? else {
        return Ok(None);
    };
    let Some(block_type) = optional_string_field(value, block_id, "type")? else {
        return Ok(None);
    };
    let Some(space_id) = optional_string_field(value, block_id, "space_id")? else {
        return Ok(None);
    };
    let Some(parent_id) = optional_string_field(value, block_id, "parent_id")? else {
        return Ok(None);
    };
    let Some(parent_table) = optional_string_field(value, block_id, "parent_table")? else {
        return Ok(None);
    };
    if block_type.trim().is_empty()
        || space_id.trim().is_empty()
        || parent_id.trim().is_empty()
        || parent_table.trim().is_empty()
        || optional_bool_field(value, block_id, "alive")?.is_none()
    {
        return Ok(None);
    }
    Ok(Some(PageBlockBase {
        block_type,
        space_id,
        parent_id,
        parent_table,
    }))
}

type PageBlockContentIds<'a> = Vec<&'a str>;

fn parse_page_block_content<'a>(
    value: &'a Value,
    block_id: &str,
    block_type: &str,
    requirement: PageBlockRequirement,
) -> Result<Option<PageBlockContentIds<'a>>, String> {
    let Some(content) = value.get("content").filter(|content| !content.is_null()) else {
        return Ok((!requirement.requires_content(block_type)).then(Vec::new));
    };
    let content = content
        .as_array()
        .ok_or_else(|| format!("Notion block {block_id} contains non-array content"))?;
    content
        .iter()
        .map(|child_id| {
            child_id
                .as_str()
                .filter(|child_id| !child_id.trim().is_empty())
                .ok_or_else(|| format!("Notion block {block_id} contains an invalid content id"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn parse_page_block_reference<'a>(
    value: &'a Value,
    block_id: &str,
    block_type: &str,
) -> Result<Option<PageBlockReference<'a>>, String> {
    let pointer_key = match block_type {
        "alias" => "alias_pointer",
        "link_to_page" => "page_pointer",
        "transclusion_reference" => "transclusion_reference_pointer",
        _ => return Ok(Some(PageBlockReference::None)),
    };
    let Some(pointer) = value
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get(pointer_key))
    else {
        return Ok(None);
    };
    let pointer = pointer
        .as_object()
        .ok_or_else(|| format!("Notion {block_type} block {block_id} has an invalid pointer"))?;
    if pointer.get("table").and_then(Value::as_str) != Some("block") {
        return Err(format!(
            "Notion {block_type} block {block_id} points to a non-block record"
        ));
    }
    let Some(id) = nonempty_pointer_field(pointer, "id") else {
        return Ok(None);
    };
    let Some(space_id) = nonempty_pointer_field(pointer, "spaceId") else {
        return Ok(None);
    };
    Ok(Some(PageBlockReference::Block(PageBlockPointer {
        id,
        space_id,
    })))
}

fn last_editor_fields_are_complete(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    last_editor_fields_are_complete_in(value)
}

fn last_editor_fields_are_complete_in(value: &Map<String, Value>) -> bool {
    let table = value.get("last_edited_by_table");
    let id = value.get("last_edited_by_id");
    match (table, id) {
        (None, None) => true,
        (Some(Value::String(table)), Some(Value::String(id))) => {
            !table.trim().is_empty() && !id.trim().is_empty()
        }
        _ => false,
    }
}
