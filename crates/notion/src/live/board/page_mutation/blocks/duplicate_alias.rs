use super::super::{
    text::empty_title_tree,
    validated_new_block_id,
    wire::{
        AliasCopyFormatArgs, DuplicatedAliasRecordArgs, NewBlockArgs, NewBlockCrdtData,
        RecordPointer, SaveOperation,
    },
    BuiltPageMutation,
};
use super::{editable_block, metadata_operation, require_direct_child, MutationIdentity};
use crate::live::board::page_state::{NotionBlockRecord, PageMutationState};
use crate::model::{DuplicatePageAliasRequest, NotionPageBlockKind, PageMutationEffect};

pub(in crate::live::board::page_mutation) fn build_duplicate_alias(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &DuplicatePageAliasRequest,
) -> Result<BuiltPageMutation, String> {
    let source = editable_block(state, request.source_block_id())?;
    if source.id == identity.page_block_id
        || source.kind != NotionPageBlockKind::Alias
        || source.parent_table != "block"
        || source.parent_id != identity.page_block_id
    {
        return Err(format!(
            "block {} is not a root-level Notion page alias",
            request.source_block_id()
        ));
    }
    if !source.content_ids.is_empty() {
        return Err(format!(
            "Notion alias block {} unexpectedly contains child blocks",
            source.id
        ));
    }
    let root = editable_block(state, identity.page_block_id)?;
    require_direct_child(root, &source.id)?;
    let target = source.alias_target.as_ref().ok_or_else(|| {
        format!(
            "Notion alias block {} does not contain a parsed target pointer",
            source.id
        )
    })?;
    let new_block_id = validated_new_block_id(state, request.new_block_id())?;

    let pointers = DuplicateAliasPointers {
        root: RecordPointer::block(identity.page_block_id, identity.space_id),
        source: RecordPointer::block(&source.id, identity.space_id),
        new: RecordPointer::block(new_block_id, identity.space_id),
        target: RecordPointer::block(&target.id, &target.space_id),
    };
    let operations = duplicate_alias_operations(source, identity, new_block_id, pointers);
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlockCreated {
            block_id: new_block_id.to_string(),
        },
        user_action: "selectableBlockActions.duplicateBlocksSynchronous",
    })
}

struct DuplicateAliasPointers {
    root: RecordPointer,
    source: RecordPointer,
    new: RecordPointer,
    target: RecordPointer,
}

fn duplicate_alias_operations(
    source: &NotionBlockRecord,
    identity: &MutationIdentity<'_>,
    new_block_id: &str,
    pointers: DuplicateAliasPointers,
) -> Vec<SaveOperation> {
    let DuplicateAliasPointers {
        root: root_pointer,
        source: source_pointer,
        new: new_pointer,
        target: target_pointer,
    } = pointers;
    vec![
        SaveOperation::set_block(
            new_pointer.clone(),
            NewBlockArgs {
                id: new_block_id.to_string(),
                block_type: "copy_indicator".to_string(),
                space_id: identity.space_id.to_string(),
                created_time: identity.now,
                created_by_table: "notion_user",
                created_by_id: identity.user_id.to_string(),
                last_edited_time: identity.now,
                crdt_data: NewBlockCrdtData {
                    title: empty_title_tree(new_block_id),
                },
                crdt_format_version: 1,
            },
        ),
        SaveOperation::insert_children_after(
            root_pointer.clone(),
            vec![new_block_id.to_string()],
            Some(source.id.clone()),
            vec![root_pointer.clone(), new_pointer.clone()],
        ),
        SaveOperation::update_duplicated_alias(
            new_pointer.clone(),
            DuplicatedAliasRecordArgs {
                id: new_block_id.to_string(),
                version: source.version,
                block_type: "alias",
                last_edited_time: identity.now,
                last_edited_by_table: "notion_user",
                last_edited_by_id: identity.user_id.to_string(),
                space_id: identity.space_id.to_string(),
                copied_from: source.id.clone(),
            },
        ),
        SaveOperation::update_alias_copy_format(
            new_pointer.clone(),
            AliasCopyFormatArgs {
                alias_pointer: target_pointer,
                block_color: source.block_color.clone(),
                copied_from_pointer: source_pointer,
            },
        ),
        SaveOperation::update_empty_block_format(new_pointer.clone()),
        metadata_operation(identity, new_block_id),
    ]
}
