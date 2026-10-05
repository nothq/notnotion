use super::super::selection_structure::{
    validate_source_order, validated_insertion_after, SourceParentGroup, ValidatedSelectionReorder,
};
use super::{editable_block, editable_parent, metadata_operations, MutationIdentity};
use crate::live::board::page_state::{NotionBlockRecord, PageMutationState};
use crate::model::PageBlockPlacement;
use std::collections::HashSet;

use super::super::wire::{RecordPointer, SaveOperation};

type SourceParentGroups = Vec<SourceParentGroup>;

pub(in crate::live::board::page_mutation) fn reorder_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    target_parent_id: &str,
    block_ids: &[String],
    placement: &PageBlockPlacement,
) -> Result<Vec<SaveOperation>, String> {
    let source_parents = source_parent_groups(state, block_ids)?
        .into_iter()
        .map(|(parent_id, _)| parent_id)
        .collect::<Vec<_>>();
    let mut operations =
        reorder_content_operations(state, identity, target_parent_id, block_ids, placement)?;
    let metadata_ids = block_ids
        .iter()
        .map(String::as_str)
        .chain([identity.page_block_id])
        .chain(source_parents.iter().map(String::as_str))
        .chain([target_parent_id]);
    operations.extend(metadata_operations(identity, metadata_ids));
    Ok(operations)
}

pub(in crate::live::board::page_mutation) fn reorder_content_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    target_parent_id: &str,
    block_ids: &[String],
    placement: &PageBlockPlacement,
) -> Result<Vec<SaveOperation>, String> {
    let moved = validate_moved_roots(state, identity.page_block_id, block_ids)?;
    let source_parents = source_parent_groups(state, block_ids)?;
    let target_parent = editable_parent(state, target_parent_id)?;
    let insertion_after = validated_insertion_after(&target_parent.content_ids, &moved, placement)?;
    for (parent_id, child_ids) in &source_parents {
        let source_parent = editable_parent(state, parent_id)?;
        validate_source_order(&source_parent.content_ids, child_ids, &moved)?;
        validate_column_source_retains_content(source_parent, target_parent_id, &moved)?;
    }
    validate_target_outside_subtrees(state, target_parent_id, block_ids)?;
    Ok(assemble_reorder_content_operations(
        identity,
        target_parent_id,
        block_ids,
        &source_parents,
        insertion_after,
    ))
}

pub(in crate::live::board::page_mutation) fn batch_validated_reorder_content_operations(
    identity: &MutationIdentity<'_>,
    plan: &ValidatedSelectionReorder,
) -> Vec<SaveOperation> {
    assemble_reorder_content_operations(
        identity,
        plan.target_parent_id(),
        plan.block_ids(),
        plan.source_parent_groups(),
        plan.insertion_after().map(str::to_string),
    )
}

fn assemble_reorder_content_operations(
    identity: &MutationIdentity<'_>,
    target_parent_id: &str,
    block_ids: &[String],
    source_parents: &[SourceParentGroup],
    insertion_after: Option<String>,
) -> Vec<SaveOperation> {
    let target_pointer = RecordPointer::block(target_parent_id, identity.space_id);
    let mut additional_updated_pointers = vec![target_pointer.clone()];
    additional_updated_pointers.extend(
        block_ids
            .iter()
            .map(|block_id| RecordPointer::block(block_id, identity.space_id)),
    );
    for (parent_id, _) in source_parents {
        if parent_id != target_parent_id {
            additional_updated_pointers.push(RecordPointer::block(parent_id, identity.space_id));
        }
    }
    let mut operations = Vec::new();
    for (parent_id, child_ids) in source_parents {
        let source_pointer = RecordPointer::block(parent_id, identity.space_id);
        for block_id in child_ids {
            operations.push(SaveOperation::set_alive(
                RecordPointer::block(block_id, identity.space_id),
                false,
            ));
            operations.push(SaveOperation::list_remove(
                source_pointer.clone(),
                block_id.clone(),
            ));
        }
    }
    operations.push(SaveOperation::insert_children_after(
        target_pointer,
        block_ids.to_vec(),
        insertion_after,
        additional_updated_pointers,
    ));
    operations
}

fn validate_column_source_retains_content(
    source_parent: &NotionBlockRecord,
    target_parent_id: &str,
    moved: &HashSet<String>,
) -> Result<(), String> {
    if source_parent.id == target_parent_id
        || !matches!(
            &source_parent.kind,
            crate::model::NotionPageBlockKind::Other(kind) if kind == "column"
        )
        || source_parent
            .content_ids
            .iter()
            .any(|block_id| !moved.contains(block_id))
    {
        return Ok(());
    }
    Err(format!(
        "subtree move would leave column {} without authoritative content",
        source_parent.id
    ))
}

fn validate_moved_roots(
    state: &PageMutationState,
    page_block_id: &str,
    block_ids: &[String],
) -> Result<HashSet<String>, String> {
    let first_id = block_ids
        .first()
        .ok_or_else(|| "a subtree move requires at least one block".to_string())?;
    let first = editable_block(state, first_id)?;
    if first.id == page_block_id || first.parent_table != "block" {
        return Err(format!("block {first_id} cannot be moved as page content"));
    }
    let moved = block_ids.iter().cloned().collect::<HashSet<_>>();
    if moved.len() != block_ids.len() {
        return Err("a subtree move contains duplicate block ids".to_string());
    }
    for block_id in block_ids.iter().skip(1) {
        let block = editable_block(state, block_id)?;
        if block.id == page_block_id || block.parent_table != "block" {
            return Err(format!("block {block_id} cannot be moved as page content"));
        }
    }
    Ok(moved)
}

fn source_parent_groups(
    state: &PageMutationState,
    block_ids: &[String],
) -> Result<SourceParentGroups, String> {
    let mut groups = SourceParentGroups::new();
    for block_id in block_ids {
        let parent_id = &state.block(block_id)?.parent_id;
        if let Some((_, children)) = groups.iter_mut().find(|(id, _)| id == parent_id) {
            children.push(block_id.clone());
        } else {
            groups.push((parent_id.clone(), vec![block_id.clone()]));
        }
    }
    if groups.is_empty() {
        return Err("a subtree move requires at least one block".to_string());
    }
    Ok(groups)
}

fn validate_target_outside_subtrees(
    state: &PageMutationState,
    target_parent_id: &str,
    block_ids: &[String],
) -> Result<(), String> {
    let mut visited = HashSet::new();
    let mut pending = block_ids.to_vec();
    while let Some(block_id) = pending.pop() {
        if block_id == target_parent_id {
            return Err(format!(
                "target parent {target_parent_id} is inside a moved subtree"
            ));
        }
        if !visited.insert(block_id.clone()) {
            continue;
        }
        if state.is_opaque_unavailable(&block_id) {
            continue;
        }
        let block = state.block(&block_id)?;
        pending.extend(block.content_ids.iter().cloned());
    }
    Ok(())
}
