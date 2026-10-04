use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::board_workspace::{PageMutationAction, PageMutationPlan};
use std::collections::HashSet;

use gpui::Context;

use super::super::support::{page_block_subtree_end, page_parent_subtree_end};
use super::super::{CardPage, CardPageBlock, SurfaceState};
use crate::model::{PageBlockPlacement, PageMutation, ReorderPageBlockSubtreesRequest};
use crate::ui::PageEditFocus;

struct PageBlockMovePlan {
    root_block_ids: Vec<String>,
    source_block_ids: HashSet<String>,
    moved_subtrees: Vec<Vec<CardPageBlock>>,
    destination_depth: usize,
}

impl PageBlockMovePlan {
    fn mutation(
        &self,
        target_parent_block_id: &str,
        before_block_id: Option<&str>,
    ) -> PageMutation {
        PageMutation::ReorderSubtrees(ReorderPageBlockSubtreesRequest {
            target_parent_block_id: target_parent_block_id.to_string(),
            block_ids: self.root_block_ids.clone(),
            placement: before_block_id.map_or(PageBlockPlacement::Append, |block_id| {
                PageBlockPlacement::Before(block_id.to_string())
            }),
        })
    }
}

struct PageBlockMoveRequest<'a> {
    dragged_root_block_ids: &'a [String],
    target_parent_block_id: &'a str,
    before_block_id: Option<&'a str>,
    focus: Option<PageEditFocus>,
}

impl SurfaceState {
    pub(crate) fn move_page_block_subtrees(
        &mut self,
        dragged_root_block_ids: &[String],
        target_parent_block_id: &str,
        before_block_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let focus = self.page_editor.current_page_edit_focus(cx);
        self.move_page_block_subtrees_with_focus(
            PageBlockMoveRequest {
                dragged_root_block_ids,
                target_parent_block_id,
                before_block_id,
                focus,
            },
            cx,
        );
    }

    fn move_page_block_subtrees_with_focus(
        &mut self,
        request: PageBlockMoveRequest<'_>,
        cx: &mut Context<Self>,
    ) {
        let PageBlockMoveRequest {
            dragged_root_block_ids,
            target_parent_block_id,
            before_block_id,
            focus,
        } = request;
        let Some(first_root_id) = dragged_root_block_ids.first() else {
            return;
        };
        let Some(mut page) = self.page_documents.page_containing_block(first_root_id) else {
            return;
        };
        if self.page_mutations.is_recovering(&page.block_id) {
            return;
        }
        let Some(plan) = page_block_move_plan(
            &page,
            dragged_root_block_ids,
            target_parent_block_id,
            before_block_id,
        ) else {
            return;
        };
        let mutation = plan.mutation(target_parent_block_id, before_block_id);
        let original_page = page.clone();
        self.page_editor.expand_page_toggle_destination(
            &self.page_documents,
            &page.block_id,
            target_parent_block_id,
        );
        apply_page_block_move(&mut page, plan, target_parent_block_id, before_block_id);
        if page.blocks == original_page.blocks {
            return;
        }
        let page_block_id = page.block_id.clone();
        self.page_editor
            .record_page_structural_edit(&original_page, focus);
        self.dispatch_page_mutation_action(
            PageMutationAction::ApplyPlan(PageMutationPlan::mutation(&page_block_id, mutation)),
            cx,
        );
        self.page_editor.page_slash_menu = None;
        self.page_editor.mention.clear_menu();
        self.page_editor.page_block_context_menu = None;
        self.page_editor.clear_page_document_selection(cx);
        self.dispatch_page_document_action(PageDocumentAction::replace_loaded(page), cx);
        cx.notify();
    }

    pub(crate) fn move_page_block_by_keyboard(
        &mut self,
        block_id: &str,
        delta: isize,
        focus_offset: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(page) = self.page_documents.page_containing_block(block_id) else {
            return;
        };
        let Some(index) = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)
        else {
            return;
        };
        let parent_block_id = page.blocks[index].parent_block_id.clone();
        let siblings = page
            .blocks
            .iter()
            .filter(|block| block.parent_block_id == parent_block_id)
            .map(|block| block.block_id.as_str())
            .collect::<Vec<_>>();
        let Some(sibling_index) = siblings.iter().position(|candidate| *candidate == block_id)
        else {
            return;
        };
        let before_block_id = keyboard_move_before_block_id(&siblings, sibling_index, delta);
        if delta < 0 && before_block_id.is_none()
            || delta >= 0 && sibling_index + 1 >= siblings.len()
        {
            return;
        }
        let dragged_root_block_ids = [block_id.to_string()];
        self.move_page_block_subtrees_with_focus(
            PageBlockMoveRequest {
                dragged_root_block_ids: &dragged_root_block_ids,
                target_parent_block_id: &parent_block_id,
                before_block_id,
                focus: Some(PageEditFocus::block(block_id, focus_offset)),
            },
            cx,
        );
    }
}

fn page_block_move_plan(
    page: &CardPage,
    requested_root_ids: &[String],
    target_parent_block_id: &str,
    before_block_id: Option<&str>,
) -> Option<PageBlockMovePlan> {
    let root_block_ids = ordered_page_block_roots(&page.blocks, requested_root_ids)?;
    let reorder = ReorderPageBlockSubtreesRequest {
        target_parent_block_id: target_parent_block_id.to_string(),
        block_ids: root_block_ids.clone(),
        placement: before_block_id.map_or(PageBlockPlacement::Append, |block_id| {
            PageBlockPlacement::Before(block_id.to_string())
        }),
    };
    reorder.validate_column_sources(page).ok()?;
    let moved_subtrees = page_block_subtrees(&page.blocks, &root_block_ids)?;
    let source_block_ids = moved_subtrees
        .iter()
        .flatten()
        .map(|block| block.block_id.clone())
        .collect::<HashSet<_>>();
    if source_block_ids.contains(target_parent_block_id)
        || before_block_id.is_some_and(|block_id| source_block_ids.contains(block_id))
    {
        return None;
    }
    let destination_depth = if target_parent_block_id == page.block_id {
        0
    } else {
        let target_parent = page
            .blocks
            .iter()
            .find(|block| block.block_id == target_parent_block_id)?;
        if !target_parent.accepts_content_children() {
            return None;
        }
        target_parent.depth + 1
    };
    if before_block_id.is_some_and(|before_block_id| {
        !page.blocks.iter().any(|block| {
            block.block_id == before_block_id && block.parent_block_id == target_parent_block_id
        })
    }) {
        return None;
    }
    Some(PageBlockMovePlan {
        root_block_ids,
        source_block_ids,
        moved_subtrees,
        destination_depth,
    })
}

fn ordered_page_block_roots(
    blocks: &[CardPageBlock],
    requested_root_ids: &[String],
) -> Option<Vec<String>> {
    let requested = requested_root_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if requested.len() != requested_root_ids.len() {
        return None;
    }
    let mut roots = Vec::new();
    let mut covered_until = 0;
    for (index, block) in blocks.iter().enumerate() {
        if requested.contains(block.block_id.as_str()) {
            if index < covered_until {
                return None;
            }
            roots.push(block.block_id.clone());
            covered_until = page_block_subtree_end(blocks, index);
        }
    }
    (roots.len() == requested.len()).then_some(roots)
}

fn page_block_subtrees(
    blocks: &[CardPageBlock],
    root_block_ids: &[String],
) -> Option<Vec<Vec<CardPageBlock>>> {
    root_block_ids
        .iter()
        .map(|root_block_id| {
            let root_index = blocks
                .iter()
                .position(|block| block.block_id == *root_block_id)?;
            let subtree_end = page_block_subtree_end(blocks, root_index);
            Some(blocks[root_index..subtree_end].to_vec())
        })
        .collect()
}

fn apply_page_block_move(
    page: &mut CardPage,
    mut plan: PageBlockMovePlan,
    target_parent_block_id: &str,
    before_block_id: Option<&str>,
) {
    page.blocks
        .retain(|block| !plan.source_block_ids.contains(&block.block_id));
    let insertion_index = before_block_id.map_or_else(
        || page_parent_subtree_end(&page.blocks, target_parent_block_id),
        |block_id| {
            page.blocks
                .iter()
                .position(|block| block.block_id == block_id)
                .expect("validated destination block must remain after removing moved subtrees")
        },
    );
    let mut moved = Vec::new();
    for mut subtree in plan.moved_subtrees.drain(..) {
        let depth_delta = plan.destination_depth as isize - subtree[0].depth as isize;
        for block in &mut subtree {
            block.depth = block.depth.saturating_add_signed(depth_delta);
        }
        subtree[0].parent_block_id = target_parent_block_id.to_string();
        moved.append(&mut subtree);
    }
    let insertion_index = insertion_index.min(page.blocks.len());
    page.blocks.splice(insertion_index..insertion_index, moved);
}

fn keyboard_move_before_block_id<'a>(
    siblings: &[&'a str],
    sibling_index: usize,
    delta: isize,
) -> Option<&'a str> {
    if delta < 0 {
        sibling_index
            .checked_sub(1)
            .and_then(|index| siblings.get(index))
            .copied()
    } else {
        siblings.get(sibling_index + 2).copied()
    }
}
