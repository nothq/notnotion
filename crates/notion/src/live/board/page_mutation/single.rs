use super::{
    blocks::{
        convert_block_operations, create_block_content_operations, create_block_operations,
        delete_block_operations, editable_block, metadata_operation, require_splittable_block,
        validate_block_conversion, CreateBlockInput, MutationIdentity,
    },
    text::replacement_operations,
    validated_new_block_id,
    wire::{BlockTypeArgs, SaveOperation},
    with_text_metadata, BuiltPageMutation, CrdtClock, PageMutationState, RecordPointer,
};
use crate::live::board::page_state::NotionBlockRecord;
use crate::model::{
    ConvertPageBlockRequest, ConvertPageBlockToDividerRequest, CreatePageBlockRequest,
    NotionPageBlockKind, PageMutationEffect, ReplacePageBlockTextRequest,
    ReplacePageBlockWithDividerRequest, RestorePageBlockFromDividerRequest,
};

mod code;

pub(super) use code::{
    build_create_code, build_replace_block_with_code, build_restore_block_from_code,
};

pub(super) fn build_create(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &CreatePageBlockRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let block_id = validated_new_block_id(state, &request.block_id)?;
    Ok(BuiltPageMutation {
        operations: create_block_operations(
            state,
            identity,
            CreateBlockInput {
                block_id,
                parent_block_id: &request.parent_block_id,
                kind: &request.kind,
                text: &request.text,
                placement: &request.placement,
            },
            clock,
        )?,
        effect: PageMutationEffect::BlockCreated {
            block_id: block_id.to_string(),
        },
        user_action: "textEnterActions.insertBlock",
    })
}

pub(super) fn build_replace_block_with_divider(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageBlockWithDividerRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let source = editable_block(state, request.source_block_id())?;
    if source.id == identity.page_block_id || source.parent_table != "block" {
        return Err(format!(
            "block {} cannot be replaced as page content",
            source.id
        ));
    }
    require_splittable_block(source)?;
    if !source.content_ids.is_empty() {
        return Err(format!(
            "block {} cannot be replaced by a divider while it contains child blocks",
            source.id
        ));
    }
    let divider_block_id = validated_new_block_id(state, request.divider_block_id())?;
    let placement = crate::model::PageBlockPlacement::After(source.id.clone());
    let mut operations = create_block_operations(
        state,
        identity,
        CreateBlockInput {
            block_id: divider_block_id,
            parent_block_id: &source.parent_id,
            kind: &crate::model::NotionPageBlockKind::Divider,
            text: "",
            placement: &placement,
        },
        clock,
    )?;
    operations.extend(delete_block_operations(state, identity, &source.id)?);
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlockCreated {
            block_id: divider_block_id.to_string(),
        },
        user_action: "actionRegistry.createInsertAction",
    })
}

pub(super) fn build_convert_block_to_divider(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ConvertPageBlockToDividerRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let source = editable_block(state, request.source_block_id())?;
    if source.id == identity.page_block_id
        || source.parent_table != "block"
        || source.kind != NotionPageBlockKind::Text
    {
        return Err(format!(
            "block {} cannot be converted from text to a divider",
            source.id
        ));
    }
    if !source.content_ids.is_empty() {
        return Err(format!(
            "block {} cannot become a divider while it contains child blocks",
            source.id
        ));
    }
    let continuation_block_id = validated_new_block_id(state, request.continuation_block_id())?;
    let source_pointer = RecordPointer::block(&source.id, identity.space_id);
    let mut operations = vec![SaveOperation::update_type(
        source_pointer.clone(),
        BlockTypeArgs {
            block_type: NotionPageBlockKind::Divider.api_type().to_string(),
        },
    )];
    operations.extend(replacement_operations(
        source_pointer,
        source.title()?,
        "",
        clock,
    )?);
    let placement = crate::model::PageBlockPlacement::After(source.id.clone());
    operations.extend(create_block_content_operations(
        state,
        identity,
        CreateBlockInput {
            block_id: continuation_block_id,
            parent_block_id: &source.parent_id,
            kind: &NotionPageBlockKind::Text,
            text: "",
            placement: &placement,
        },
        clock,
    )?);
    operations.push(metadata_operation(identity, &source.id));
    operations.push(metadata_operation(identity, continuation_block_id));
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlockCreated {
            block_id: continuation_block_id.to_string(),
        },
        user_action: "textFilterActions.runAllFiltersAfterTransaction",
    })
}

pub(super) fn build_restore_block_from_divider(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &RestorePageBlockFromDividerRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let restored = RestorableDivider::parse(state, identity, request)?;
    let divider_pointer = RecordPointer::block(&restored.divider.id, identity.space_id);
    let mut operations = vec![SaveOperation::update_type(
        divider_pointer.clone(),
        BlockTypeArgs {
            block_type: NotionPageBlockKind::Text.api_type().to_string(),
        },
    )];
    operations.extend(replacement_operations(
        divider_pointer,
        restored.divider.title()?,
        request.text(),
        clock,
    )?);
    operations.extend(delete_block_operations(
        state,
        identity,
        &restored.continuation.id,
    )?);
    operations.push(metadata_operation(identity, &restored.divider.id));
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "keyboardInputShortcut.undo",
    })
}

struct RestorableDivider<'a> {
    divider: &'a NotionBlockRecord,
    continuation: &'a NotionBlockRecord,
}

impl<'a> RestorableDivider<'a> {
    fn parse(
        state: &'a PageMutationState,
        identity: &MutationIdentity<'_>,
        request: &RestorePageBlockFromDividerRequest,
    ) -> Result<Self, String> {
        let divider = editable_block(state, request.divider_block_id())?;
        if divider.id == identity.page_block_id
            || divider.parent_table != "block"
            || divider.kind != NotionPageBlockKind::Divider
            || !divider.content_ids.is_empty()
        {
            return Err(format!(
                "block {} is not a restorable leaf divider",
                divider.id
            ));
        }
        let continuation = editable_block(state, request.continuation_block_id())?;
        if continuation.parent_id != divider.parent_id
            || continuation.parent_table != "block"
            || continuation.kind != NotionPageBlockKind::Text
            || !continuation.content_ids.is_empty()
        {
            return Err(format!(
                "block {} is not the divider's empty text continuation",
                continuation.id
            ));
        }
        let parent = editable_block(state, &divider.parent_id)?;
        let divider_index = parent
            .content_ids
            .iter()
            .position(|block_id| block_id == &divider.id)
            .ok_or_else(|| format!("block {} is not a child of {}", divider.id, parent.id))?;
        if parent.content_ids.get(divider_index + 1) != Some(&continuation.id) {
            return Err(format!(
                "block {} does not immediately follow divider {}",
                continuation.id, divider.id
            ));
        }
        Ok(Self {
            divider,
            continuation,
        })
    }
}

pub(super) fn build_replace_text(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageBlockTextRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, &request.block_id)?;
    let operations = replacement_operations(
        RecordPointer::block(&block.id, identity.space_id),
        block.title()?,
        &request.text,
        clock,
    )?;
    Ok(BuiltPageMutation {
        operations: with_text_metadata(operations, identity, &block.id),
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "Text.handleMutation",
    })
}

pub(super) fn build_convert(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ConvertPageBlockRequest,
) -> Result<BuiltPageMutation, String> {
    let conversion = validate_block_conversion(state, identity, &request.block_id, &request.kind)?;
    Ok(BuiltPageMutation {
        operations: convert_block_operations(identity, &conversion),
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "transformActions.transformBlock",
    })
}
