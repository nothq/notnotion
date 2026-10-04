use std::collections::HashSet;

use super::{editable_parent, metadata_operations, validate_placement, MutationIdentity};
use crate::live::board::page_state::PageMutationState;
use crate::model::{NotionPageBlockKind, PageBlockPlacement};

use super::super::{
    text::{empty_title_tree, initial_insert_operation},
    wire::{NewBlockArgs, NewBlockCrdtData, ParentArgs, RecordPointer, SaveOperation},
    CrdtClock,
};
use super::kind::creatable_block_type;

#[derive(Clone, Copy)]
pub(in crate::live::board::page_mutation) struct CreateBlockInput<'a> {
    pub(in crate::live::board::page_mutation) block_id: &'a str,
    pub(in crate::live::board::page_mutation) parent_block_id: &'a str,
    pub(in crate::live::board::page_mutation) kind: &'a NotionPageBlockKind,
    pub(in crate::live::board::page_mutation) text: &'a str,
    pub(in crate::live::board::page_mutation) placement: &'a PageBlockPlacement,
}

pub(in crate::live::board::page_mutation) fn create_block_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    input: CreateBlockInput<'_>,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let mut operations = create_block_content_operations(state, identity, input, clock)?;
    operations.extend(metadata_operations(
        identity,
        [
            input.block_id,
            input.parent_block_id,
            identity.page_block_id,
        ],
    ));
    Ok(operations)
}

pub(in crate::live::board::page_mutation) fn create_block_content_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    input: CreateBlockInput<'_>,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let parent = editable_parent(state, input.parent_block_id)?;
    validate_placement(parent, input.placement, &HashSet::new())?;
    let block_type = creatable_block_type(input.kind)?;
    if matches!(input.kind, NotionPageBlockKind::Divider) && !input.text.is_empty() {
        return Err("a divider block cannot contain text".to_string());
    }
    let tree = empty_title_tree(input.block_id);
    let child_pointer = RecordPointer::block(input.block_id, identity.space_id);
    let parent_pointer = RecordPointer::block(input.parent_block_id, identity.space_id);
    let mut operations = vec![
        SaveOperation::set_block(
            child_pointer.clone(),
            NewBlockArgs {
                id: input.block_id.to_string(),
                block_type: block_type.to_string(),
                space_id: identity.space_id.to_string(),
                created_time: identity.now,
                created_by_table: "notion_user",
                created_by_id: identity.user_id.to_string(),
                last_edited_time: identity.now,
                crdt_data: NewBlockCrdtData {
                    title: tree.clone(),
                },
                crdt_format_version: 1,
            },
        ),
        SaveOperation::update_parent(
            child_pointer.clone(),
            ParentArgs {
                parent_id: input.parent_block_id.to_string(),
                parent_table: "block",
                alive: true,
            },
        ),
        super::placement_operation(parent_pointer, input.block_id, input.placement),
    ];
    operations.extend(initial_insert_operation(
        child_pointer,
        &tree,
        input.text,
        clock,
    )?);
    Ok(operations)
}
