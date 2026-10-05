use std::collections::HashSet;

use super::{PageTextReplacementMutationInput, ReparentedRoot};
use crate::model::{
    ColumnSafePageBlockMutationSequence, DeletePageBlockRequest, PageBlockPlacement,
    PageBlockStructuralMutation, PageMutation, PageTextSelectionEdit,
    ReorderPageBlockSubtreesRequest, ReplacePageTextSelectionRequest,
};
use crate::ui::CardPage;

pub(super) fn page_text_replacement_mutation(
    input: PageTextReplacementMutationInput<'_>,
) -> Result<PageMutation, String> {
    let mutations =
        page_text_selection_structural_mutations(input.page, input.reparented, input.removed)?;
    Ok(PageMutation::ReplaceTextSelection(
        ReplacePageTextSelectionRequest::new(
            input.start,
            input.end,
            input.selected_block_ids,
            PageTextSelectionEdit {
                replacement: input.replacement,
                action: input.action,
            },
            mutations,
        )?,
    ))
}

pub(in super::super) fn page_text_selection_structural_mutations(
    page: &CardPage,
    reparented: &[ReparentedRoot],
    removed: &HashSet<String>,
) -> Result<Vec<PageBlockStructuralMutation>, String> {
    let mut reorder_requests = Vec::new();
    let moved_ids = reparented
        .iter()
        .map(|moved| moved.block_id.as_str())
        .collect::<HashSet<_>>();
    for moved in reparented {
        let moved_index = page
            .blocks
            .iter()
            .position(|block| block.block_id == moved.block_id)
            .ok_or_else(|| format!("retained block {} is missing", moved.block_id))?;
        let anchor = page.blocks[moved_index + 1..].iter().find(|block| {
            block.parent_block_id == moved.target_parent_block_id
                && !removed.contains(&block.block_id)
                && !moved_ids.contains(block.block_id.as_str())
        });
        let request = ReorderPageBlockSubtreesRequest {
            target_parent_block_id: moved.target_parent_block_id.clone(),
            block_ids: vec![moved.block_id.clone()],
            placement: anchor.map_or(PageBlockPlacement::Append, |block| {
                PageBlockPlacement::Before(block.block_id.clone())
            }),
        };
        reorder_requests.push(request);
    }
    let mut mutations = reorder_requests
        .into_iter()
        .map(PageBlockStructuralMutation::Reorder)
        .collect::<Vec<_>>();
    mutations.extend(
        page.blocks
            .iter()
            .filter(|block| {
                removed.contains(&block.block_id) && !removed.contains(&block.parent_block_id)
            })
            .map(|block| {
                PageBlockStructuralMutation::Delete(DeletePageBlockRequest {
                    block_id: block.block_id.clone(),
                })
            }),
    );
    Ok(ColumnSafePageBlockMutationSequence::new(page, mutations)?.into_mutations())
}
