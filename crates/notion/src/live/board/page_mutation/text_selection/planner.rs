use std::collections::HashSet;

use super::super::selection_structure::{
    ValidatedSelectionDeletion, ValidatedSelectionStructure, ValidatedSelectionStructureStep,
};
use super::super::{
    blocks::{
        batch_validated_reorder_content_operations, metadata_operations, require_mergeable_block,
        MutationIdentity,
    },
    text::{text_offset_utf8_to_utf16, TextInsertionPoint},
    wire::{InsertTextArgs, RecordPointer, SaveOperation},
    BuiltPageMutation, CrdtClock,
};
use super::{belongs_to_deleted_subtree, validate_composition, SelectionMutation};
use crate::live::board::page_state::{NotionBlockRecord, PageMutationState};
use crate::model::{
    PageMutationEffect, PageTextSelectionAction, ReplacePageTextSelectionEffect,
    ReplacePageTextSelectionRequest,
};

mod merge;

use merge::build_text_merge_operations;

pub(super) struct ResolvedSelection<'a> {
    pub(super) first: &'a NotionBlockRecord,
    pub(super) intermediate: Vec<&'a NotionBlockRecord>,
    pub(super) last: &'a NotionBlockRecord,
    pub(super) first_offset_utf16: usize,
    pub(super) last_offset_utf16: usize,
}

pub(super) struct SelectionMergePlan<'a> {
    pub(super) selection: ResolvedSelection<'a>,
    pub(super) survivor_pointer: RecordPointer,
    pub(super) survivor_insertion: TextInsertionPoint,
    pub(super) operations: Vec<SaveOperation>,
    pub(super) effects: Vec<PageMutationEffect>,
    pub(super) structure: ValidatedSelectionStructure,
}

pub(in crate::live::board::page_mutation) fn build_text_selection_replacement(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ReplacePageTextSelectionRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let mutation = SelectionMutation::from(request);
    let mut plan = build_selection_merge_plan(state, identity, mutation, clock)?;
    if !request.replacement().is_empty() {
        plan.operations.push(SaveOperation::insert_text(
            plan.survivor_pointer,
            InsertTextArgs {
                operation_type: "insertText",
                text_instance_id: plan.survivor_insertion.text_instance_id,
                search_label: plan.survivor_insertion.search_label,
                id: clock.reserve_after(
                    &plan.survivor_insertion.origin_id,
                    request.replacement().encode_utf16().count(),
                )?,
                origin_id: plan.survivor_insertion.origin_id,
                content: request.replacement().to_string(),
                prev_items: plan.survivor_insertion.prev_items,
            },
        ));
    }
    append_metadata_operations(identity, mutation, &plan.structure, &mut plan.operations);

    Ok(BuiltPageMutation {
        operations: plan.operations,
        effect: PageMutationEffect::TextSelectionReplaced(ReplacePageTextSelectionEffect {
            effects: plan.effects,
        }),
        user_action: match request.action() {
            PageTextSelectionAction::TextMutation => "Text.handleMutation",
            PageTextSelectionAction::Paste => "paste",
            PageTextSelectionAction::Cut => "clipboardActions.cutWithMaybeConfirmation",
        },
    })
}

pub(super) fn build_selection_merge_plan<'a>(
    state: &'a PageMutationState,
    identity: &MutationIdentity<'_>,
    mutation: SelectionMutation<'_>,
    clock: &mut CrdtClock,
) -> Result<SelectionMergePlan<'a>, String> {
    let structure = validate_composition(state, mutation)?;
    let selection = resolve_selection(state, mutation)?;
    validate_removed_selection(state, mutation, &selection)?;
    let text_merge = build_text_merge_operations(&selection, identity, clock)?;
    let mut operations = text_merge.operations;
    let effects = append_structural_operations(identity, &structure, &mut operations);
    Ok(SelectionMergePlan {
        selection,
        survivor_pointer: text_merge.survivor_pointer,
        survivor_insertion: text_merge.survivor_insertion,
        operations,
        effects,
        structure,
    })
}

fn resolve_selection<'a>(
    state: &'a PageMutationState,
    mutation: SelectionMutation<'_>,
) -> Result<ResolvedSelection<'a>, String> {
    let ordered = ordered_text_blocks(state)?;
    let selected_ids = mutation
        .selected_block_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let selected = ordered
        .iter()
        .copied()
        .filter(|block| selected_ids.contains(block.id.as_str()))
        .collect::<Vec<_>>();
    if selected.len() != mutation.selected_block_ids.len() {
        let missing = mutation
            .selected_block_ids
            .iter()
            .find(|block_id| {
                !selected
                    .iter()
                    .any(|block| block.id.as_str() == block_id.as_str())
            })
            .expect("a selected block count mismatch must contain a missing block");
        return Err(format!("selected block {missing} is not editable"));
    }
    if selected
        .iter()
        .zip(mutation.selected_block_ids)
        .any(|(block, requested_id)| block.id.as_str() != requested_id.as_str())
    {
        return Err("selected blocks are not in document order".to_string());
    }
    for block in &selected {
        require_mergeable_block(block)?;
        block.title()?;
    }
    let first = selected[0];
    let last = selected[selected.len() - 1];
    Ok(ResolvedSelection {
        first,
        intermediate: selected[1..selected.len() - 1].to_vec(),
        last,
        first_offset_utf16: text_offset_utf8_to_utf16(first.title()?, mutation.start.offset_utf8)?,
        last_offset_utf16: text_offset_utf8_to_utf16(last.title()?, mutation.end.offset_utf8)?,
    })
}

pub(super) fn ordered_text_blocks(
    state: &PageMutationState,
) -> Result<Vec<&NotionBlockRecord>, String> {
    fn append<'a>(
        state: &'a PageMutationState,
        parent_id: &str,
        seen: &mut HashSet<String>,
        ordered: &mut Vec<&'a NotionBlockRecord>,
    ) -> Result<(), String> {
        for block_id in &state.block(parent_id)?.content_ids {
            if !seen.insert(block_id.clone()) {
                return Err(format!(
                    "page hierarchy contains block {block_id} more than once"
                ));
            }
            if state.is_opaque_unavailable(block_id) {
                continue;
            }
            let block = state.block(block_id)?;
            if block.alive && block.title.is_some() {
                ordered.push(block);
            }
            append(state, block_id, seen, ordered)?;
        }
        Ok(())
    }

    let mut ordered = Vec::new();
    append(
        state,
        &state.page_block_id,
        &mut HashSet::new(),
        &mut ordered,
    )?;
    Ok(ordered)
}

fn validate_removed_selection(
    state: &PageMutationState,
    mutation: SelectionMutation<'_>,
    selection: &ResolvedSelection<'_>,
) -> Result<(), String> {
    let deleted = mutation
        .structural_mutations
        .iter()
        .filter_map(|mutation| match mutation {
            crate::model::PageBlockStructuralMutation::Delete(request) => {
                Some(request.block_id.clone())
            }
            crate::model::PageBlockStructuralMutation::Reorder(_) => None,
        })
        .collect::<HashSet<_>>();
    for block in selection
        .intermediate
        .iter()
        .copied()
        .chain(std::iter::once(selection.last))
    {
        if !belongs_to_deleted_subtree(state, &block.id, &deleted)? {
            return Err(format!(
                "selected block {} is not removed by the structural mutations",
                block.id
            ));
        }
    }
    if belongs_to_deleted_subtree(state, &selection.first.id, &deleted)? {
        return Err("the first text-selection endpoint cannot be removed".to_string());
    }
    Ok(())
}

fn append_structural_operations(
    identity: &MutationIdentity<'_>,
    structure: &ValidatedSelectionStructure,
    operations: &mut Vec<SaveOperation>,
) -> Vec<PageMutationEffect> {
    let mut effects = Vec::with_capacity(structure.steps().len());
    for step in structure.steps() {
        match step {
            ValidatedSelectionStructureStep::Reorder(plan) => {
                operations.extend(batch_validated_reorder_content_operations(identity, plan));
            }
            ValidatedSelectionStructureStep::Delete(plan) => {
                operations.extend(retire_block_operations(identity, plan));
            }
        }
        effects.push(PageMutationEffect::NoAdditionalContext);
    }
    effects
}

fn retire_block_operations(
    identity: &MutationIdentity<'_>,
    plan: &ValidatedSelectionDeletion,
) -> Vec<SaveOperation> {
    vec![
        SaveOperation::set_alive(
            RecordPointer::block(plan.block_id(), identity.space_id),
            false,
        ),
        SaveOperation::list_remove(
            RecordPointer::block(plan.parent_id(), identity.space_id),
            plan.block_id().to_string(),
        ),
    ]
}

fn append_metadata_operations(
    identity: &MutationIdentity<'_>,
    mutation: SelectionMutation<'_>,
    structure: &ValidatedSelectionStructure,
    operations: &mut Vec<SaveOperation>,
) {
    let deferred = [
        identity.page_block_id,
        mutation.start.block_id.as_str(),
        mutation.end.block_id.as_str(),
    ];
    let mut ids = Vec::new();
    for step in structure.steps() {
        match step {
            ValidatedSelectionStructureStep::Reorder(plan) => {
                for (block_id, parent_id) in plan.block_source_parents() {
                    push_structural_metadata_id(&mut ids, &deferred, block_id);
                    push_structural_metadata_id(&mut ids, &deferred, parent_id);
                }
                push_structural_metadata_id(&mut ids, &deferred, plan.target_parent_id());
            }
            ValidatedSelectionStructureStep::Delete(plan) => {
                push_structural_metadata_id(&mut ids, &deferred, plan.block_id());
                push_structural_metadata_id(&mut ids, &deferred, plan.parent_id());
            }
        }
    }
    ids.extend(deferred);
    operations.extend(metadata_operations(identity, ids));
}

fn push_structural_metadata_id<'a>(ids: &mut Vec<&'a str>, deferred: &[&str], block_id: &'a str) {
    if !deferred.contains(&block_id) {
        ids.push(block_id);
    }
}
