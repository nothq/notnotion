use super::super::page_state::{NotionBlockRecord, PageMutationState};
use super::{
    text::{empty_title_tree, initial_insert_operation},
    wire::{
        BlockTypeArgs, EditedMetadataArgs, EmptyTitleProperties, NewBlockCrdtData, ParentArgs,
        PastedBlockArgs, RecordPointer, SaveOperation,
    },
    CrdtClock,
};
use crate::model::{NotionPageBlockKind, PageBlockPlacement};
use std::collections::HashSet;

mod code;
mod create;
mod duplicate_alias;
mod kind;
mod reorder;

pub(super) use code::{create_code_block_operations, CreateCodeBlockInput};
pub(super) use create::{
    create_block_content_operations, create_block_operations, CreateBlockInput,
};
pub(super) use duplicate_alias::build_duplicate_alias;
use kind::{convertible_block_type, require_text_navigation_kind};
pub(super) use reorder::{batch_validated_reorder_content_operations, reorder_operations};

pub(super) struct MutationIdentity<'a> {
    pub(super) space_id: &'a str,
    pub(super) user_id: &'a str,
    pub(super) page_block_id: &'a str,
    pub(super) now: u64,
}

pub(super) struct PasteTextBlockInput<'a> {
    pub(super) block_id: &'a str,
    pub(super) after_block_id: &'a str,
    pub(super) text: &'a str,
}

pub(super) struct ValidatedBlockConversion<'a> {
    pub(super) block: &'a NotionBlockRecord,
    block_type: String,
}

pub(super) fn paste_text_block_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    input: PasteTextBlockInput<'_>,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let PasteTextBlockInput {
        block_id,
        after_block_id,
        text,
    } = input;
    let after = editable_block(state, after_block_id)?;
    if after.parent_table != "block" {
        return Err(format!(
            "paste anchor block {after_block_id} is not page content"
        ));
    }
    let parent = editable_parent(state, &after.parent_id)?;
    require_direct_child(parent, after_block_id)?;

    let tree = empty_title_tree(block_id);
    let block_pointer = RecordPointer::block(block_id, identity.space_id);
    let mut operations = vec![SaveOperation::set_pasted_block(
        block_pointer.clone(),
        PastedBlockArgs {
            id: block_id.to_string(),
            block_type: "text",
            properties: EmptyTitleProperties { title: Vec::new() },
            parent_id: parent.id.clone(),
            parent_table: "block",
            alive: true,
            space_id: identity.space_id.to_string(),
            created_time: identity.now,
            created_by_table: "notion_user",
            created_by_id: identity.user_id.to_string(),
            last_edited_time: identity.now,
            last_edited_by_table: "notion_user",
            last_edited_by_id: identity.user_id.to_string(),
            crdt_data: NewBlockCrdtData {
                title: tree.clone(),
            },
            crdt_format_version: 1,
        },
    )];
    operations.extend(initial_insert_operation(block_pointer, &tree, text, clock)?);
    operations.push(SaveOperation::list_after(
        RecordPointer::block(&parent.id, identity.space_id),
        block_id.to_string(),
        Some(after_block_id.to_string()),
    ));
    operations.extend(metadata_operations(
        identity,
        [block_id, parent.id.as_str(), identity.page_block_id],
    ));
    Ok(operations)
}

pub(super) fn validate_block_conversion<'a>(
    state: &'a PageMutationState,
    identity: &MutationIdentity<'_>,
    block_id: &str,
    kind: &NotionPageBlockKind,
) -> Result<ValidatedBlockConversion<'a>, String> {
    let block = editable_block(state, block_id)?;
    if block_id == identity.page_block_id {
        return Err("the page root cannot be converted as a content block".to_string());
    }
    require_text_navigation_kind(&block.kind, "converting from")?;
    let block_type = convertible_block_type(kind)?.to_string();
    Ok(ValidatedBlockConversion { block, block_type })
}

pub(super) fn convert_block_operations(
    identity: &MutationIdentity<'_>,
    conversion: &ValidatedBlockConversion<'_>,
) -> Vec<SaveOperation> {
    vec![
        SaveOperation::update_type(
            RecordPointer::block(&conversion.block.id, identity.space_id),
            BlockTypeArgs {
                block_type: conversion.block_type.clone(),
            },
        ),
        metadata_operation(identity, &conversion.block.id),
        metadata_operation(identity, identity.page_block_id),
    ]
}

pub(super) fn delete_block_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    block_id: &str,
) -> Result<Vec<SaveOperation>, String> {
    if block_id == identity.page_block_id {
        return Err("the page root cannot be deleted as a content block".to_string());
    }
    let block = editable_block(state, block_id)?;
    let parent = editable_parent(state, &block.parent_id)?;
    require_direct_child(parent, block_id)?;
    let pointer = RecordPointer::block(block_id, identity.space_id);
    let mut operations = vec![
        SaveOperation::set_alive(pointer, false),
        SaveOperation::list_remove(
            RecordPointer::block(&block.parent_id, identity.space_id),
            block_id.to_string(),
        ),
    ];
    operations.extend(metadata_operations(
        identity,
        [block_id, block.parent_id.as_str(), identity.page_block_id],
    ));
    Ok(operations)
}

pub(super) fn move_children_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    source_block_id: &str,
    target_block_id: &str,
) -> Result<Vec<SaveOperation>, String> {
    let source = editable_parent(state, source_block_id)?;
    reject_opaque_children(state, source)?;
    let mut operations = Vec::new();
    let mut after = None;
    for child_id in &source.content_ids {
        operations.extend(move_child(
            identity,
            child_id,
            source_block_id,
            target_block_id,
            after.clone(),
        ));
        after = Some(child_id.clone());
    }
    Ok(operations)
}

pub(super) fn lift_children_before_source(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    source: &NotionBlockRecord,
) -> Result<Vec<SaveOperation>, String> {
    editable_parent(state, &source.parent_id)?;
    reject_opaque_children(state, source)?;
    let mut operations = Vec::new();
    for child_id in &source.content_ids {
        operations.extend(move_child_before(
            identity,
            child_id,
            &source.id,
            &source.parent_id,
            &source.id,
        ));
    }
    Ok(operations)
}

fn reject_opaque_children(
    state: &PageMutationState,
    source: &NotionBlockRecord,
) -> Result<(), String> {
    let Some(block_id) = source
        .content_ids
        .iter()
        .find(|block_id| state.is_opaque_unavailable(block_id))
    else {
        return Ok(());
    };
    Err(format!(
        "block {} contains opaque unavailable child {block_id} and cannot reparent its children",
        source.id
    ))
}

fn move_child(
    identity: &MutationIdentity<'_>,
    child_id: &str,
    source_parent_id: &str,
    target_parent_id: &str,
    after: Option<String>,
) -> Vec<SaveOperation> {
    let child_pointer = RecordPointer::block(child_id, identity.space_id);
    let target_pointer = RecordPointer::block(target_parent_id, identity.space_id);
    vec![
        SaveOperation::set_alive(child_pointer.clone(), false),
        SaveOperation::list_remove(
            RecordPointer::block(source_parent_id, identity.space_id),
            child_id.to_string(),
        ),
        SaveOperation::update_parent(
            child_pointer,
            ParentArgs {
                parent_id: target_parent_id.to_string(),
                parent_table: "block",
                alive: true,
            },
        ),
        SaveOperation::list_after(target_pointer, child_id.to_string(), after),
    ]
}

fn move_child_before(
    identity: &MutationIdentity<'_>,
    child_id: &str,
    source_parent_id: &str,
    target_parent_id: &str,
    before: &str,
) -> Vec<SaveOperation> {
    let child_pointer = RecordPointer::block(child_id, identity.space_id);
    vec![
        SaveOperation::set_alive(child_pointer.clone(), false),
        SaveOperation::list_remove(
            RecordPointer::block(source_parent_id, identity.space_id),
            child_id.to_string(),
        ),
        SaveOperation::update_parent(
            child_pointer,
            ParentArgs {
                parent_id: target_parent_id.to_string(),
                parent_table: "block",
                alive: true,
            },
        ),
        SaveOperation::list_before(
            RecordPointer::block(target_parent_id, identity.space_id),
            child_id.to_string(),
            before.to_string(),
        ),
    ]
}

fn placement_operation(
    parent_pointer: RecordPointer,
    block_id: &str,
    placement: &PageBlockPlacement,
) -> SaveOperation {
    match placement {
        PageBlockPlacement::Append => {
            SaveOperation::list_after(parent_pointer, block_id.to_string(), None)
        }
        PageBlockPlacement::Before(before) => {
            SaveOperation::list_before(parent_pointer, block_id.to_string(), before.clone())
        }
        PageBlockPlacement::After(after) => {
            SaveOperation::list_after(parent_pointer, block_id.to_string(), Some(after.clone()))
        }
    }
}

fn validate_placement(
    parent: &NotionBlockRecord,
    placement: &PageBlockPlacement,
    moved: &HashSet<String>,
) -> Result<(), String> {
    let anchor = match placement {
        PageBlockPlacement::Append => return Ok(()),
        PageBlockPlacement::Before(anchor) | PageBlockPlacement::After(anchor) => anchor,
    };
    if moved.contains(anchor) {
        return Err("a subtree reorder anchor cannot be one of the moved blocks".to_string());
    }
    require_direct_child(parent, anchor)
}

fn require_direct_child(parent: &NotionBlockRecord, child_id: &str) -> Result<(), String> {
    if parent.content_ids.iter().any(|id| id == child_id) {
        return Ok(());
    }
    Err(format!("block {child_id} is not a child of {}", parent.id))
}

fn editable_parent<'a>(
    state: &'a PageMutationState,
    block_id: &str,
) -> Result<&'a NotionBlockRecord, String> {
    let block = editable_block(state, block_id)?;
    if block.space_id != state.space_id {
        return Err(format!("block {block_id} belongs to another space"));
    }
    Ok(block)
}

pub(super) fn editable_block<'a>(
    state: &'a PageMutationState,
    block_id: &str,
) -> Result<&'a NotionBlockRecord, String> {
    let block = state.block(block_id)?;
    if !block.alive {
        return Err(format!("block {block_id} is not alive"));
    }
    if block.version == 0 {
        return Err(format!("block {block_id} has an invalid zero version"));
    }
    Ok(block)
}

pub(super) fn require_mergeable_block(block: &NotionBlockRecord) -> Result<(), String> {
    require_text_navigation_kind(&block.kind, "merging")
}

pub(super) fn require_splittable_block(block: &NotionBlockRecord) -> Result<(), String> {
    require_text_navigation_kind(&block.kind, "splitting")
}

pub(super) fn metadata_operations<'a>(
    identity: &MutationIdentity<'_>,
    block_ids: impl IntoIterator<Item = &'a str>,
) -> Vec<SaveOperation> {
    let mut seen = HashSet::new();
    block_ids
        .into_iter()
        .filter(|block_id| seen.insert(*block_id))
        .map(|block_id| metadata_operation(identity, block_id))
        .collect()
}

pub(super) fn metadata_operation(identity: &MutationIdentity<'_>, block_id: &str) -> SaveOperation {
    SaveOperation::update_metadata(
        RecordPointer::block(block_id, identity.space_id),
        EditedMetadataArgs {
            last_edited_time: identity.now,
            last_edited_by_table: "notion_user",
            last_edited_by_id: identity.user_id.to_string(),
        },
    )
}
