use super::super::{
    blocks::{
        create_block_content_operations, metadata_operations, CreateBlockInput, MutationIdentity,
    },
    text::{split_operations, title_insertion_point},
    wire::{InsertTextArgs, SaveOperation},
    BuiltPageMutation, CrdtClock,
};
use super::{planner::build_selection_merge_plan, SelectionMutation};
use crate::live::board::page_mutation::selection_structure::{
    ValidatedSelectionStructure, ValidatedSelectionStructureStep,
};
use crate::live::board::page_state::PageMutationState;
use crate::model::{
    BreakPageTextSelectionEffect, BreakPageTextSelectionRequest, NotionPageBlockKind,
    PageBlockPlacement, PageMutationEffect, PageTextCaretTarget, PageTextLineBreak,
};

pub(in crate::live::board::page_mutation) fn build_text_selection_line_break(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &BreakPageTextSelectionRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let mutation = SelectionMutation::from(request);
    let mut plan = build_selection_merge_plan(state, identity, mutation, clock)?;
    let survivor_id = plan.selection.first.id.clone();
    let terminal_id = plan.selection.last.id.clone();
    let (applied, user_action) = match request.line_break() {
        PageTextLineBreak::Enter {
            new_block_id,
            new_block_kind,
        } => (
            append_enter_operations(
                state,
                identity,
                EnterNewBlock {
                    id: new_block_id,
                    kind: new_block_kind,
                },
                &mut plan,
                clock,
            )?,
            "Text.handleEnter",
        ),
        PageTextLineBreak::ShiftEnter => (
            append_shift_enter_operation(request, &mut plan, clock)?,
            "Text.handleShiftEnter",
        ),
    };
    append_line_break_metadata(
        identity,
        LineBreakMetadataInput {
            structure: &plan.structure,
            survivor_id: &survivor_id,
            terminal_id: &terminal_id,
            created_block_id: applied.created_block_id.as_deref(),
        },
        &mut plan.operations,
    );
    let effect = line_break_effect(request.line_break(), plan.effects, applied.caret);

    Ok(BuiltPageMutation {
        operations: plan.operations,
        effect: PageMutationEffect::TextSelectionBroken(effect),
        user_action,
    })
}

struct AppliedLineBreak {
    caret: PageTextCaretTarget,
    created_block_id: Option<String>,
}

struct EnterNewBlock<'a> {
    id: &'a str,
    kind: &'a NotionPageBlockKind,
}

struct LineBreakMetadataInput<'a> {
    structure: &'a ValidatedSelectionStructure,
    survivor_id: &'a str,
    terminal_id: &'a str,
    created_block_id: Option<&'a str>,
}

fn append_enter_operations(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    new_block: EnterNewBlock<'_>,
    plan: &mut super::planner::SelectionMergePlan<'_>,
    clock: &mut CrdtClock,
) -> Result<AppliedLineBreak, String> {
    let EnterNewBlock {
        id: new_block_id,
        kind: new_block_kind,
    } = new_block;
    let survivor_id = plan.selection.first.id.clone();
    let placement = PageBlockPlacement::After(survivor_id);
    plan.operations.extend(create_block_content_operations(
        state,
        identity,
        CreateBlockInput {
            block_id: new_block_id,
            parent_block_id: &plan.selection.first.parent_id,
            kind: new_block_kind,
            text: "",
            placement: &placement,
        },
        clock,
    )?);
    plan.operations.extend(split_operations(
        plan.survivor_pointer.clone(),
        new_block_id,
        plan.selection.last.title()?,
        plan.selection.last_offset_utf16,
        clock,
    )?);
    Ok(AppliedLineBreak {
        caret: PageTextCaretTarget {
            block_id: new_block_id.to_string(),
            offset_utf8: 0,
        },
        created_block_id: Some(new_block_id.to_string()),
    })
}

fn append_shift_enter_operation(
    request: &BreakPageTextSelectionRequest,
    plan: &mut super::planner::SelectionMergePlan<'_>,
    clock: &mut CrdtClock,
) -> Result<AppliedLineBreak, String> {
    let insertion = title_insertion_point(
        plan.selection.last.title()?,
        plan.selection.last_offset_utf16,
    )?;
    plan.operations.push(SaveOperation::insert_text(
        plan.survivor_pointer.clone(),
        InsertTextArgs {
            operation_type: "insertText",
            text_instance_id: insertion.text_instance_id,
            search_label: insertion.search_label,
            id: clock.reserve_after(&insertion.origin_id, 1)?,
            origin_id: insertion.origin_id,
            content: "\n".to_string(),
            prev_items: insertion.prev_items,
        },
    ));
    Ok(AppliedLineBreak {
        caret: PageTextCaretTarget {
            block_id: plan.selection.first.id.clone(),
            offset_utf8: request.start().offset_utf8 + 1,
        },
        created_block_id: None,
    })
}

fn line_break_effect(
    line_break: &PageTextLineBreak,
    effects: Vec<PageMutationEffect>,
    caret: PageTextCaretTarget,
) -> BreakPageTextSelectionEffect {
    match line_break {
        PageTextLineBreak::Enter { .. } => BreakPageTextSelectionEffect::Enter { effects, caret },
        PageTextLineBreak::ShiftEnter => {
            BreakPageTextSelectionEffect::ShiftEnter { effects, caret }
        }
    }
}

fn append_line_break_metadata(
    identity: &MutationIdentity<'_>,
    input: LineBreakMetadataInput<'_>,
    operations: &mut Vec<SaveOperation>,
) {
    let LineBreakMetadataInput {
        structure,
        survivor_id,
        terminal_id,
        created_block_id,
    } = input;
    let mut block_ids = Vec::new();
    block_ids.push(survivor_id);
    block_ids.push(identity.page_block_id);
    block_ids.push(terminal_id);
    for step in structure.steps() {
        match step {
            ValidatedSelectionStructureStep::Reorder(plan) => {
                for (block_id, parent_id) in plan.block_source_parents() {
                    block_ids.push(block_id);
                    block_ids.push(parent_id);
                }
                block_ids.push(plan.target_parent_id());
            }
            ValidatedSelectionStructureStep::Delete(plan) => {
                block_ids.push(plan.block_id());
                block_ids.push(plan.parent_id());
            }
        }
    }
    if let Some(block_id) = created_block_id {
        block_ids.push(block_id);
    }
    operations.extend(metadata_operations(identity, block_ids));
}
