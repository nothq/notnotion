use super::{
    convert_block_operations, create_block_operations, delete_block_operations, editable_block,
    lift_children_before_source, merge_operations, metadata_operation, move_children_operations,
    reorder_operations, replacement_operations, require_mergeable_block, require_splittable_block,
    split_operations, validate_block_conversion, validated_new_block_id, BuiltPageMutation,
    CrdtClock, CreateBlockInput, MergePageBlockChildrenEffect, MergePageBlocksEffect,
    MergePageBlocksRequest, MutationIdentity, NotionBlockRecord, PageBlockPlacement,
    PageMutationEffect, PageMutationState, RecordPointer, ReorderPageBlockSubtreesRequest,
    ReplacePageBlockTextAndConvertRequest, SplitPageBlockEffect, SplitPageBlockRequest,
};

pub(super) fn build_replace_text_and_convert(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageBlockTextAndConvertRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let conversion = validate_block_conversion(state, identity, &request.block_id, &request.kind)?;
    let mut operations = replacement_operations(
        RecordPointer::block(&conversion.block.id, identity.space_id),
        conversion.block.title()?,
        &request.text,
        clock,
    )?;
    operations.extend(convert_block_operations(identity, &conversion));
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "transformActions.transformBlock",
    })
}

pub(super) fn build_split(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &SplitPageBlockRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let source = editable_block(state, &request.block_id)?;
    if source.id == identity.page_block_id || source.parent_table != "block" {
        return Err(format!(
            "block {} cannot be split as page content",
            source.id
        ));
    }
    require_splittable_block(source)?;
    let new_block_id = validated_new_block_id(state, &request.new_block_id)?;
    let placement = PageBlockPlacement::After(source.id.clone());
    let mut operations = create_block_operations(
        state,
        identity,
        CreateBlockInput {
            block_id: new_block_id,
            parent_block_id: &source.parent_id,
            kind: &request.new_block_kind,
            text: "",
            placement: &placement,
        },
        clock,
    )?;
    operations.extend(move_children_operations(
        state,
        identity,
        &source.id,
        new_block_id,
    )?);
    operations.extend(split_operations(
        RecordPointer::block(&source.id, identity.space_id),
        new_block_id,
        source.title()?,
        request.split_offset_utf16,
        clock,
    )?);
    operations.push(metadata_operation(identity, &source.id));
    operations.extend(
        source
            .content_ids
            .iter()
            .filter(|child_id| !state.is_opaque_unavailable(child_id))
            .map(|child_id| metadata_operation(identity, child_id)),
    );
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlockSplit(SplitPageBlockEffect {
            source_block_id: source.id.clone(),
            new_block_id: new_block_id.to_string(),
            parent_block_id: source.parent_id.clone(),
            moved_child_block_ids: source.content_ids.clone(),
        }),
        user_action: "Text.handleEnter",
    })
}

pub(super) fn build_merge(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &MergePageBlocksRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let source = editable_block(state, &request.source_block_id)?;
    let target = editable_block(state, &request.target_block_id)?;
    require_mergeable_block(source)?;
    require_mergeable_block(target)?;
    validate_merge_siblings(state, identity.page_block_id, source, target)?;
    let mut operations = merge_operations(
        RecordPointer::block(&source.id, identity.space_id),
        source.title()?,
        &target.id,
        target.title()?,
        clock,
    )?;
    if source.kind != crate::model::NotionPageBlockKind::Toggle {
        operations.extend(lift_children_before_source(state, identity, source)?);
    }
    operations.extend(delete_block_operations(state, identity, &source.id)?);
    operations.push(metadata_operation(identity, &target.id));
    operations.extend(
        source
            .content_ids
            .iter()
            .filter(|child_id| !state.is_opaque_unavailable(child_id))
            .map(|child_id| metadata_operation(identity, child_id)),
    );
    let source_children = if source.kind == crate::model::NotionPageBlockKind::Toggle {
        MergePageBlockChildrenEffect::RemainUnderRemovedSource {
            child_block_ids: source.content_ids.clone(),
        }
    } else {
        MergePageBlockChildrenEffect::LiftedToSourceParent {
            child_block_ids: source.content_ids.clone(),
        }
    };
    Ok(BuiltPageMutation {
        operations,
        effect: PageMutationEffect::BlocksMerged(MergePageBlocksEffect {
            source_block_id: source.id.clone(),
            target_block_id: target.id.clone(),
            source_parent_block_id: source.parent_id.clone(),
            source_children,
        }),
        user_action: "textBackspaceActions.textBackspaceAtBeginning",
    })
}

pub(super) fn build_reorder(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReorderPageBlockSubtreesRequest,
) -> Result<BuiltPageMutation, String> {
    Ok(BuiltPageMutation {
        operations: reorder_operations(
            state,
            identity,
            &request.target_parent_block_id,
            &request.block_ids,
            &request.placement,
        )?,
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "ListItemBlock.handleDrop",
    })
}

pub(super) fn validate_merge_siblings(
    state: &PageMutationState,
    page_block_id: &str,
    source: &NotionBlockRecord,
    target: &NotionBlockRecord,
) -> Result<(), String> {
    if source.id == page_block_id || target.id == page_block_id {
        return Err("the page root cannot participate in a content-block merge".to_string());
    }
    if source.parent_table != "block"
        || target.parent_table != "block"
        || source.parent_id != target.parent_id
    {
        return Err("merged blocks must have the same block parent".to_string());
    }
    let parent = state.block(&source.parent_id)?;
    let source_index = parent
        .content_ids
        .iter()
        .position(|id| id == &source.id)
        .ok_or_else(|| format!("parent {} does not contain source block", parent.id))?;
    if source_index == 0 || parent.content_ids[source_index - 1] != target.id {
        return Err("merge target must immediately precede the source block".to_string());
    }
    Ok(())
}
