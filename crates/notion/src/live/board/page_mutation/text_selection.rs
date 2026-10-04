use std::collections::{HashMap, HashSet};

use super::selection_structure::ValidatedSelectionStructure;
use crate::live::board::page_state::PageMutationState;
use crate::model::{
    BreakPageTextSelectionRequest, PageBlockPlacement, PageBlockStructuralMutation,
    PageTextSelectionEndpoint, ReplacePageTextSelectionRequest,
};

mod line_break;
mod paste;
mod planner;

pub(super) use line_break::build_text_selection_line_break;
pub(super) use paste::build_multiline_text_selection_paste;
pub(super) use planner::build_text_selection_replacement;

#[derive(Clone, Copy)]
pub(super) struct SelectionMutation<'a> {
    pub(super) start: &'a PageTextSelectionEndpoint,
    pub(super) end: &'a PageTextSelectionEndpoint,
    pub(super) selected_block_ids: &'a [String],
    pub(super) structural_mutations: &'a [PageBlockStructuralMutation],
}

impl<'a> From<&'a ReplacePageTextSelectionRequest> for SelectionMutation<'a> {
    fn from(request: &'a ReplacePageTextSelectionRequest) -> Self {
        Self {
            start: request.start(),
            end: request.end(),
            selected_block_ids: request.selected_block_ids(),
            structural_mutations: request.structural_mutations(),
        }
    }
}

impl<'a> From<&'a BreakPageTextSelectionRequest> for SelectionMutation<'a> {
    fn from(request: &'a BreakPageTextSelectionRequest) -> Self {
        Self {
            start: request.start(),
            end: request.end(),
            selected_block_ids: request.selected_block_ids(),
            structural_mutations: request.structural_mutations(),
        }
    }
}

fn validate_composition(
    state: &PageMutationState,
    selection: SelectionMutation<'_>,
) -> Result<ValidatedSelectionStructure, String> {
    let composition = collect_composition_changes(selection.structural_mutations)?;

    reject_intersection(
        &composition.moved_ids,
        &composition.deleted_ids,
        "move and delete",
    )?;
    if composition.moved_ids.contains(&selection.start.block_id)
        || composition.deleted_ids.contains(&selection.start.block_id)
    {
        return Err(
            "the first text-selection endpoint must remain stationary and alive".to_string(),
        );
    }
    validate_stable_anchors(
        &composition.reorder_requests,
        &composition.moved_ids,
        &composition.deleted_ids,
    )?;
    validate_disjoint_deletions(state, &composition.deleted_ids)?;
    let structure = ValidatedSelectionStructure::new(state, selection.structural_mutations)?;
    validate_final_hierarchy(
        state,
        structure.final_parent_ids(),
        &composition.deleted_ids,
        &composition.moved_ids,
        &selection.start.block_id,
    )?;
    Ok(structure)
}

struct CompositionChanges<'a> {
    moved_ids: HashSet<String>,
    deleted_ids: HashSet<String>,
    reorder_requests: Vec<&'a crate::model::ReorderPageBlockSubtreesRequest>,
}

fn collect_composition_changes(
    mutations: &[PageBlockStructuralMutation],
) -> Result<CompositionChanges<'_>, String> {
    let mut moved_ids = HashSet::new();
    let mut deleted_ids = HashSet::new();
    let mut reorder_requests = Vec::new();

    for mutation in mutations {
        match mutation {
            PageBlockStructuralMutation::Reorder(request) => {
                register_moved_blocks(&mut moved_ids, &request.block_ids)?;
                reorder_requests.push(request);
            }
            PageBlockStructuralMutation::Delete(request) => {
                register_deleted_block(&mut deleted_ids, &request.block_id)?;
            }
        }
    }

    Ok(CompositionChanges {
        moved_ids,
        deleted_ids,
        reorder_requests,
    })
}

fn register_moved_blocks(
    moved_ids: &mut HashSet<String>,
    block_ids: &[String],
) -> Result<(), String> {
    for block_id in block_ids {
        if !moved_ids.insert(block_id.clone()) {
            return Err(format!(
                "text-selection replacement moves block {block_id} more than once"
            ));
        }
    }
    Ok(())
}

fn register_deleted_block(deleted_ids: &mut HashSet<String>, block_id: &str) -> Result<(), String> {
    if deleted_ids.insert(block_id.to_string()) {
        return Ok(());
    }
    Err(format!(
        "text-selection replacement deletes block {block_id} more than once"
    ))
}

fn reject_intersection(
    left: &HashSet<String>,
    right: &HashSet<String>,
    operation: &str,
) -> Result<(), String> {
    let Some(block_id) = left.intersection(right).next() else {
        return Ok(());
    };
    Err(format!(
        "text-selection replacement cannot {operation} block {block_id}"
    ))
}

fn validate_stable_anchors(
    reorders: &[&crate::model::ReorderPageBlockSubtreesRequest],
    moved_ids: &HashSet<String>,
    deleted_ids: &HashSet<String>,
) -> Result<(), String> {
    for request in reorders {
        let anchor = match &request.placement {
            PageBlockPlacement::Append => continue,
            PageBlockPlacement::Before(anchor) | PageBlockPlacement::After(anchor) => anchor,
        };
        if moved_ids.contains(anchor) || deleted_ids.contains(anchor) {
            return Err(format!(
                "text-selection replacement reorder anchor {anchor} must remain stationary and alive"
            ));
        }
    }
    Ok(())
}

fn validate_disjoint_deletions(
    state: &PageMutationState,
    deleted_ids: &HashSet<String>,
) -> Result<(), String> {
    for block_id in deleted_ids {
        if block_id == &state.page_block_id {
            return Err(
                "the page root cannot be deleted by a text-selection replacement".to_string(),
            );
        }
        let mut current_id = block_id.as_str();
        let mut visited = HashSet::new();
        loop {
            let block = state.block(current_id)?;
            if block.parent_table != "block" || block.parent_id == state.page_block_id {
                break;
            }
            let parent_id = block.parent_id.as_str();
            if !visited.insert(parent_id.to_string()) {
                return Err(format!(
                    "text-selection replacement found a hierarchy cycle at block {parent_id}"
                ));
            }
            if deleted_ids.contains(parent_id) {
                return Err(format!(
                    "text-selection replacement cannot separately delete nested blocks {parent_id} and {block_id}"
                ));
            }
            current_id = parent_id;
        }
    }
    Ok(())
}

fn validate_final_hierarchy(
    state: &PageMutationState,
    final_parents: &HashMap<String, String>,
    deleted_ids: &HashSet<String>,
    moved_ids: &HashSet<String>,
    stationary_start_id: &str,
) -> Result<(), String> {
    for block_id in moved_ids {
        require_page_root_destination(state, final_parents, deleted_ids, block_id, "moved block")?;
    }
    require_page_root_destination(
        state,
        final_parents,
        deleted_ids,
        stationary_start_id,
        "stationary first text-selection endpoint",
    )?;
    for (block_id, _) in state.blocks.iter().filter(|(_, block)| block.alive) {
        if !reaches_final_destination(
            state,
            final_parents,
            deleted_ids,
            block_id,
            FinalHierarchyDestination::PageRootOrDeletedSubtree,
        )? {
            return Err(format!(
                "text-selection replacement leaves retained block {block_id} outside the page hierarchy"
            ));
        }
    }
    Ok(())
}

fn require_page_root_destination(
    state: &PageMutationState,
    final_parents: &HashMap<String, String>,
    deleted_ids: &HashSet<String>,
    block_id: &str,
    subject: &str,
) -> Result<(), String> {
    if reaches_final_destination(
        state,
        final_parents,
        deleted_ids,
        block_id,
        FinalHierarchyDestination::PageRoot,
    )? {
        return Ok(());
    }
    Err(format!(
        "text-selection replacement leaves {subject} {block_id} outside the retained page hierarchy"
    ))
}

pub(super) fn belongs_to_deleted_subtree(
    state: &PageMutationState,
    block_id: &str,
    deleted_ids: &HashSet<String>,
) -> Result<bool, String> {
    let mut current = state.block(block_id)?;
    let mut visited = HashSet::new();
    loop {
        if deleted_ids.contains(&current.id) {
            return Ok(true);
        }
        if current.id == state.page_block_id || current.parent_table != "block" {
            return Ok(false);
        }
        if !visited.insert(current.id.as_str()) {
            return Err(format!(
                "page hierarchy contains a cycle at block {}",
                current.id
            ));
        }
        current = state.block(&current.parent_id)?;
    }
}

#[derive(Clone, Copy)]
enum FinalHierarchyDestination {
    PageRoot,
    PageRootOrDeletedSubtree,
}

fn reaches_final_destination(
    state: &PageMutationState,
    final_parents: &HashMap<String, String>,
    deleted_ids: &HashSet<String>,
    block_id: &str,
    destination: FinalHierarchyDestination,
) -> Result<bool, String> {
    let mut current_id = block_id;
    let mut visited = HashSet::new();
    loop {
        if current_id == state.page_block_id {
            return Ok(true);
        }
        if deleted_ids.contains(current_id) {
            return Ok(matches!(
                destination,
                FinalHierarchyDestination::PageRootOrDeletedSubtree
            ));
        }
        if !visited.insert(current_id.to_string()) {
            return Err(format!(
                "text-selection replacement creates a hierarchy cycle at block {current_id}"
            ));
        }
        let block = state.block(current_id)?;
        if !block.alive || block.parent_table != "block" {
            return Ok(false);
        }
        current_id = final_parents
            .get(current_id)
            .ok_or_else(|| format!("block {current_id} is missing a final parent"))?;
    }
}
