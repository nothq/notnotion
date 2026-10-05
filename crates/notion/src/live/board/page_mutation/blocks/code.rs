use std::collections::HashSet;

use super::{
    editable_parent, metadata_operations, placement_operation, validate_placement, MutationIdentity,
};
use crate::live::board::page_state::PageMutationState;
use crate::model::{CardPageCodeSettings, PageBlockPlacement};

use super::super::{
    text::empty_title_tree,
    wire::{
        BlockPropertyPrimitiveArgs, BlockPropertyPrimitiveOperation, BlockPropertyValueArgs,
        CodeLanguagePropertyArgs, CodeWrapArgs, EmptyBlockFormatArgs, NewBlockCrdtData,
        NewCodeBlockArgs, ParentArgs, RecordPointer, SaveOperation,
    },
};

#[derive(Clone, Copy)]
pub(in crate::live::board::page_mutation) struct CreateCodeBlockInput<'a> {
    pub(in crate::live::board::page_mutation) block_id: &'a str,
    pub(in crate::live::board::page_mutation) parent_block_id: &'a str,
    pub(in crate::live::board::page_mutation) placement: &'a PageBlockPlacement,
    pub(in crate::live::board::page_mutation) settings: &'a CardPageCodeSettings,
}

pub(in crate::live::board::page_mutation) fn create_code_block_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    input: CreateCodeBlockInput<'_>,
) -> Result<Vec<SaveOperation>, String> {
    let parent = editable_parent(state, input.parent_block_id)?;
    validate_placement(parent, input.placement, &HashSet::new())?;
    let pointer = RecordPointer::block(input.block_id, identity.space_id);
    let parent_pointer = RecordPointer::block(input.parent_block_id, identity.space_id);
    let mut operations = code_record_operations(identity, input, pointer.clone());
    operations.push(SaveOperation::update_parent(
        pointer,
        ParentArgs {
            parent_id: input.parent_block_id.to_string(),
            parent_table: "block",
            alive: true,
        },
    ));
    operations.push(placement_operation(
        parent_pointer,
        input.block_id,
        input.placement,
    ));
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

fn code_record_operations(
    identity: &MutationIdentity<'_>,
    input: CreateCodeBlockInput<'_>,
    pointer: RecordPointer,
) -> Vec<SaveOperation> {
    let tree = empty_title_tree(input.block_id);
    vec![
        SaveOperation::set_code_block(
            pointer.clone(),
            NewCodeBlockArgs {
                id: input.block_id.to_string(),
                block_type: "code",
                format: EmptyBlockFormatArgs {},
                space_id: identity.space_id.to_string(),
                created_time: identity.now,
                created_by_table: "notion_user",
                created_by_id: identity.user_id.to_string(),
                last_edited_time: identity.now,
                crdt_data: NewBlockCrdtData { title: tree },
                crdt_format_version: 1,
            },
        ),
        SaveOperation::update_block_property_value(
            pointer.clone(),
            BlockPropertyValueArgs {
                primitive_op: BlockPropertyPrimitiveOperation {
                    command: "update",
                    args: BlockPropertyPrimitiveArgs::CodeLanguage(CodeLanguagePropertyArgs {
                        language: [[input.settings.language().as_str().to_string()]],
                    }),
                },
            },
            vec![pointer.clone()],
        ),
        SaveOperation::update_code_wrap(
            pointer,
            CodeWrapArgs {
                code_wrap: input.settings.wrap().is_enabled(),
            },
        ),
    ]
}
