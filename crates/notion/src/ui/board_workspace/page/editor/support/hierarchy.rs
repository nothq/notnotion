use std::collections::HashSet;
use std::sync::Arc;

use super::super::{CardPage, CardPageBlock, CardPageBlockKind, LoadedCardPage};

pub(in crate::ui::board_workspace::page::editor) fn page_block_subtree_end(
    blocks: &[CardPageBlock],
    index: usize,
) -> usize {
    let depth = blocks[index].depth;
    blocks[index + 1..]
        .iter()
        .position(|block| block.depth <= depth)
        .map_or(blocks.len(), |offset| index + 1 + offset)
}

pub(in crate::ui::board_workspace::page::editor) fn page_parent_subtree_end(
    blocks: &[CardPageBlock],
    parent_block_id: &str,
) -> usize {
    blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.parent_block_id == parent_block_id)
        .map(|(index, _)| page_block_subtree_end(blocks, index))
        .max()
        .or_else(|| {
            blocks
                .iter()
                .position(|block| block.block_id == parent_block_id)
                .map(|parent_index| parent_index + 1)
        })
        .unwrap_or(blocks.len())
}

pub(in crate::ui::board_workspace::page::editor) fn reparent_direct_children(
    blocks: &mut [CardPageBlock],
    source_index: usize,
    target_block_id: &str,
) {
    let source_id = blocks[source_index].block_id.clone();
    let subtree_end = page_block_subtree_end(blocks, source_index);
    for child in &mut blocks[source_index + 1..subtree_end] {
        if child.parent_block_id == source_id {
            child.parent_block_id = target_block_id.to_string();
        }
    }
}

pub(in crate::ui::board_workspace::page::editor) fn remove_merged_page_block(
    blocks: &mut Vec<CardPageBlock>,
    source_index: usize,
) {
    let source = blocks[source_index].clone();
    let subtree_end = page_block_subtree_end(blocks, source_index);
    if source
        .editable_content()
        .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
    {
        blocks.drain(source_index..subtree_end);
        return;
    }
    for descendant in &mut blocks[source_index + 1..subtree_end] {
        descendant.depth = descendant.depth.saturating_sub(1);
        if descendant.parent_block_id == source.block_id {
            descendant
                .parent_block_id
                .clone_from(&source.parent_block_id);
        }
    }
    blocks.remove(source_index);
}

pub(in crate::ui::board_workspace::page::editor) fn indent_page_block_in_page(
    page: &mut CardPage,
    index: usize,
) {
    let parent_block_id = page.blocks[index].parent_block_id.clone();
    let Some(previous_sibling_index) = (0..index)
        .rev()
        .find(|candidate| page.blocks[*candidate].parent_block_id == parent_block_id)
    else {
        return;
    };
    if !page.blocks[previous_sibling_index].can_accept_children() {
        return;
    }
    let new_parent_block_id = page.blocks[previous_sibling_index].block_id.clone();
    let subtree_end = page_block_subtree_end(&page.blocks, index);
    for block in &mut page.blocks[index..subtree_end] {
        block.depth += 1;
    }
    page.blocks[index].parent_block_id = new_parent_block_id;
}

pub(in crate::ui::board_workspace::page::editor) fn replace_loaded_card_page(
    loaded: &mut LoadedCardPage,
    page: CardPage,
    authority: Arc<CardPage>,
    expanded_toggle_ids: Option<&HashSet<String>>,
) -> bool {
    let list_state = loaded.list_state.clone();
    let list_allocation = loaded.list_allocation.clone();
    let mut replacement = LoadedCardPage::with_authority(page, authority, expanded_toggle_ids);
    Arc::make_mut(&mut replacement.data).reuse_document_unit_layout_revisions(&loaded.data);
    let visible_projection_changed = !loaded.data.has_same_visible_projection(&replacement.data);
    *loaded = replacement;
    loaded.list_state = list_state;
    loaded.list_allocation = list_allocation;
    if !loaded.data.has_column_structure() {
        loaded.list_state.remeasure();
    }
    visible_projection_changed
}
