use std::collections::HashSet;

use crate::model::{
    CardPage, PageBlockPlacement, PageBlockStructuralMutation, PageMutation,
    ReorderPageBlockSubtreesRequest,
};

use super::super::super::super::super::rich_text::PageWriteOperation;

pub(super) struct PageOrderEffect {
    before: Vec<PageOrderConstraint>,
    after: Vec<PageOrderConstraint>,
}

#[derive(Clone)]
pub(super) struct PageOrderConstraint {
    parent_id: String,
    ordered_block_ids: Vec<String>,
}

impl PageOrderEffect {
    pub(super) fn matches(&self, page: &CardPage, after: bool) -> bool {
        let constraints = if after { &self.after } else { &self.before };
        constraints
            .iter()
            .all(|constraint| constraint.matches(page))
    }
}

impl PageOrderConstraint {
    pub(super) fn matches(&self, page: &CardPage) -> bool {
        let expected = self
            .ordered_block_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        page.blocks
            .iter()
            .filter(|block| {
                block.parent_block_id == self.parent_id
                    && expected.contains(block.block_id.as_str())
            })
            .map(|block| block.block_id.as_str())
            .eq(self.ordered_block_ids.iter().map(String::as_str))
    }
}

pub(super) fn fixed_write_order(
    operation: &PageWriteOperation,
    after: &CardPage,
) -> Option<PageOrderConstraint> {
    let PageWriteOperation::Mutation(mutation) = operation else {
        return None;
    };
    match mutation {
        PageMutation::CreateBlock(request) => placement_constraint(
            &request.block_id,
            &request.parent_block_id,
            &request.placement,
        ),
        PageMutation::CreateCodeBlock(request) => placement_constraint(
            request.block_id(),
            request.parent_block_id(),
            request.placement(),
        ),
        PageMutation::SplitBlock(request) => {
            block_pair(after, &request.block_id, &request.new_block_id)
        }
        PageMutation::BreakTextSelection(request) => {
            let crate::model::PageTextLineBreak::Enter { new_block_id, .. } = request.line_break()
            else {
                return None;
            };
            block_pair(after, &request.start().block_id, new_block_id)
        }
        PageMutation::PasteTextSelection(request) => {
            block_pair(after, &request.end().block_id, request.new_block_id())
        }
        _ => None,
    }
}

pub(super) fn mutation_order_effects(
    mutation: &PageMutation,
    before: &CardPage,
    after: &CardPage,
) -> Vec<PageOrderEffect> {
    let replacement = |source_id, destination_id| {
        replacement_order_effect(before, after, source_id, destination_id)
            .into_iter()
            .collect()
    };
    match mutation {
        PageMutation::DuplicateAlias(request) => {
            block_pair(after, request.source_block_id(), request.new_block_id())
                .map(|after| PageOrderEffect {
                    before: Vec::new(),
                    after: vec![after],
                })
                .into_iter()
                .collect()
        }
        PageMutation::ConvertBlockToDivider(request) => block_pair(
            after,
            request.source_block_id(),
            request.continuation_block_id(),
        )
        .map(|after| PageOrderEffect {
            before: Vec::new(),
            after: vec![after],
        })
        .into_iter()
        .collect(),
        PageMutation::ReplaceBlockWithCode(request) => {
            replacement(request.source_block_id(), request.code_block_id())
        }
        PageMutation::RestoreBlockFromCode(request) => {
            replacement(request.code_block_id(), request.source_block_id())
        }
        PageMutation::ReorderSubtrees(request) => vec![reorder_effect(request, before, after)],
        PageMutation::ReplaceTextSelection(request) => {
            nested_order_effects(request.structural_mutations(), before, after)
        }
        PageMutation::BreakTextSelection(request) => {
            nested_order_effects(request.structural_mutations(), before, after)
        }
        _ => Vec::new(),
    }
}

fn replacement_order_effect(
    before: &CardPage,
    after: &CardPage,
    source_id: &str,
    destination_id: &str,
) -> Option<PageOrderEffect> {
    let source = before
        .blocks
        .iter()
        .find(|block| block.block_id == source_id)?;
    let siblings = before
        .blocks
        .iter()
        .filter(|block| block.parent_block_id == source.parent_block_id)
        .map(|block| block.block_id.as_str())
        .collect::<Vec<_>>();
    let index = siblings
        .iter()
        .position(|block_id| *block_id == source_id)?;
    let neighbors = [
        index.checked_sub(1),
        (index + 1 < siblings.len()).then_some(index + 1),
    ]
    .into_iter()
    .flatten()
    .map(|index| siblings[index].to_string())
    .collect::<Vec<_>>();
    if neighbors.is_empty() {
        return None;
    }
    Some(PageOrderEffect {
        before: replacement_constraints(before, source_id, &neighbors),
        after: replacement_constraints(after, destination_id, &neighbors),
    })
}

fn replacement_constraints(
    page: &CardPage,
    replacement_id: &str,
    neighbors: &[String],
) -> Vec<PageOrderConstraint> {
    let mut relevant = neighbors.to_vec();
    relevant.push(replacement_id.to_string());
    constraints_in_page_order(page, &relevant)
}

fn nested_order_effects(
    mutations: &[PageBlockStructuralMutation],
    before: &CardPage,
    after: &CardPage,
) -> Vec<PageOrderEffect> {
    mutations
        .iter()
        .filter_map(|mutation| match mutation {
            PageBlockStructuralMutation::Reorder(request) => {
                Some(reorder_effect(request, before, after))
            }
            PageBlockStructuralMutation::Delete(_) => None,
        })
        .collect()
}

fn reorder_effect(
    request: &ReorderPageBlockSubtreesRequest,
    before: &CardPage,
    after: &CardPage,
) -> PageOrderEffect {
    let mut relevant = request.block_ids.clone();
    if let PageBlockPlacement::Before(anchor) | PageBlockPlacement::After(anchor) =
        &request.placement
    {
        relevant.push(anchor.clone());
    }
    PageOrderEffect {
        before: constraints_in_page_order(before, &relevant),
        after: constraints_in_page_order(after, &relevant),
    }
}

fn constraints_in_page_order(page: &CardPage, relevant: &[String]) -> Vec<PageOrderConstraint> {
    let relevant = relevant.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut parents = Vec::<String>::new();
    for block in &page.blocks {
        if relevant.contains(block.block_id.as_str()) && !parents.contains(&block.parent_block_id) {
            parents.push(block.parent_block_id.clone());
        }
    }
    parents
        .into_iter()
        .map(|parent_id| PageOrderConstraint {
            ordered_block_ids: page
                .blocks
                .iter()
                .filter(|block| {
                    block.parent_block_id == parent_id && relevant.contains(block.block_id.as_str())
                })
                .map(|block| block.block_id.clone())
                .collect(),
            parent_id,
        })
        .collect()
}

fn placement_constraint(
    block_id: &str,
    parent_id: &str,
    placement: &PageBlockPlacement,
) -> Option<PageOrderConstraint> {
    let ordered_block_ids = match placement {
        PageBlockPlacement::Before(anchor) => vec![block_id.to_string(), anchor.clone()],
        PageBlockPlacement::After(anchor) => vec![anchor.clone(), block_id.to_string()],
        PageBlockPlacement::Append => return None,
    };
    Some(PageOrderConstraint {
        parent_id: parent_id.to_string(),
        ordered_block_ids,
    })
}

fn block_pair(page: &CardPage, first: &str, second: &str) -> Option<PageOrderConstraint> {
    let parent_id = page
        .blocks
        .iter()
        .find(|block| block.block_id == second)?
        .parent_block_id
        .clone();
    Some(PageOrderConstraint {
        parent_id,
        ordered_block_ids: vec![first.to_string(), second.to_string()],
    })
}
