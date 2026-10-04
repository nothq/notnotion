use super::super::super::support::page_block_subtree_end;
use super::super::super::{
    LoadedCardPageData, PageBlockDragWash, PAGE_BLOCK_DROP_LINE_HEIGHT, PAGE_BLOCK_INDENT,
};
use super::{
    PageBlockDragDestination, PageBlockDragGeometry, PageBlockDragViewport,
    PAGE_BLOCK_DROP_WASH_INSET, PAGE_CALLOUT_BODY_INDENT_EXTRA, PAGE_CALLOUT_BODY_RIGHT_INSET,
};

pub(super) fn page_block_before_target(
    data: &LoadedCardPageData,
    geometry: &PageBlockDragGeometry,
    row_index: usize,
) -> Option<PageBlockDragDestination> {
    let row = &data.visible_rows[row_index];
    let block = &data.page.blocks[row.block_index];
    let nested_offset = if row.visual_depth == 0 {
        0.0
    } else {
        PAGE_BLOCK_DROP_LINE_HEIGHT
    };
    Some(PageBlockDragDestination {
        parent_block_id: block.parent_block_id.clone(),
        before_block_id: Some(block.block_id.clone()),
        depth: row.visual_depth,
        indicator_top: geometry.row(data, row_index)?.visual_top - nested_offset,
    })
}

pub(super) fn page_block_after_target(
    data: &LoadedCardPageData,
    geometry: &PageBlockDragGeometry,
    row_index: usize,
) -> Option<PageBlockDragDestination> {
    let blocks = &data.page.blocks;
    let row = &data.visible_rows[row_index];
    let block_index = row.block_index;
    let block = &blocks[block_index];
    let subtree_end = page_block_subtree_end(blocks, block_index);
    let next_sibling = blocks.get(subtree_end).filter(|candidate| {
        candidate.parent_block_id == block.parent_block_id && candidate.depth == block.depth
    });
    let indicator_top = geometry.subtree_span(data, row_index)?.visual_bottom;
    Some(PageBlockDragDestination {
        parent_block_id: block.parent_block_id.clone(),
        before_block_id: next_sibling.map(|sibling| sibling.block_id.clone()),
        depth: row.visual_depth,
        indicator_top,
    })
}

pub(super) fn page_block_nested_target(
    data: &LoadedCardPageData,
    geometry: &PageBlockDragGeometry,
    parent_row_index: usize,
) -> Option<PageBlockDragDestination> {
    let blocks = &data.page.blocks;
    let parent_row = &data.visible_rows[parent_row_index];
    let parent_index = parent_row.block_index;
    let parent = &blocks[parent_index];
    let destination_depth = parent_row.visual_depth + 1;
    let first_child_index = blocks[parent_index + 1..]
        .iter()
        .position(|block| block.parent_block_id == parent.block_id)
        .map(|offset| parent_index + 1 + offset);
    let first_visible_child_row = data.visible_rows[parent_row_index + 1..]
        .iter()
        .position(|row| row.block_index < page_block_subtree_end(blocks, parent_index))
        .map(|offset| parent_row_index + 1 + offset);
    let indicator_top = match first_visible_child_row {
        Some(row_index) => geometry.row(data, row_index)?.visual_top - PAGE_BLOCK_DROP_LINE_HEIGHT,
        None => geometry.row(data, parent_row_index)?.visual_bottom,
    };
    Some(PageBlockDragDestination {
        parent_block_id: parent.block_id.clone(),
        before_block_id: first_child_index.map(|index| blocks[index].block_id.clone()),
        depth: destination_depth,
        indicator_top,
    })
}

pub(super) fn page_block_drop_wash(
    data: &LoadedCardPageData,
    geometry: &PageBlockDragGeometry,
    destination: &PageBlockDragDestination,
    row_width: f32,
    viewport: PageBlockDragViewport,
) -> Option<PageBlockDragWash> {
    let target_parent_block_id = destination.parent_block_id.as_str();
    let destination_depth = destination.depth;
    if target_parent_block_id == data.page.block_id {
        return None;
    }
    let parent_index = data
        .editable_block_indices
        .get(target_parent_block_id)
        .copied()?;
    let parent_row_index = data
        .visible_rows
        .binary_search_by_key(&parent_index, |row| row.block_index)
        .ok()?;
    let parent_span = geometry.subtree_span(data, parent_row_index)?;
    let wash_top = (parent_span.visual_top + PAGE_BLOCK_DROP_WASH_INSET).max(viewport.top);
    let subtree_bottom =
        parent_span.visual_bottom.min(viewport.bottom) - PAGE_BLOCK_DROP_WASH_INSET;
    if subtree_bottom <= wash_top {
        return None;
    }
    let parent_callout_layers = data.callout_layer_depth(parent_row_index);
    let parent_is_callout = data.effective_callout_colors[parent_row_index].is_some();
    let ancestor_callout_layers = parent_callout_layers - usize::from(parent_is_callout);
    let parent_left = destination_depth.saturating_sub(1) as f32 * PAGE_BLOCK_INDENT
        + ancestor_callout_layers as f32 * PAGE_CALLOUT_BODY_INDENT_EXTRA;
    let parent_right = ancestor_callout_layers as f32 * PAGE_CALLOUT_BODY_RIGHT_INSET;
    Some(PageBlockDragWash {
        top: wash_top - viewport.top,
        left: viewport.page_left + parent_left + PAGE_BLOCK_DROP_WASH_INSET,
        width: (row_width - parent_left - parent_right - PAGE_BLOCK_DROP_WASH_INSET * 2.0).max(0.0),
        height: (subtree_bottom - wash_top).max(0.0),
    })
}
