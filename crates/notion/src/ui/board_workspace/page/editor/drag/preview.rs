use std::collections::HashSet;

use super::super::support::{page_block_subtree_end, page_block_visible_row_spacing};
use super::super::{
    CardPageBlock, LoadedCardPageData, PageBlockDragPayload, PageBlockDragPreviewRow,
    PageBlockDragPreviewRows,
};
use crate::ui::{LoadedCardPageDocumentUnit, NotionBlockImageCache, PageTextSelection};

const PAGE_BLOCK_DRAG_PREVIEW_VISIBLE_ROWS: usize = 5;

struct PageBlockDragPreviewSourceRow {
    visible_row_index: usize,
    block_index: usize,
    visual_depth: usize,
}

struct PageBlockDragPreviewWindow {
    start: usize,
    visible_count: usize,
    overflow_count: usize,
    anchor_index: usize,
}

pub(in crate::ui::board_workspace::page::editor) struct PageBlockDragVisiblePreview<'a> {
    pub(in crate::ui::board_workspace::page::editor) rows_before_anchor:
        &'a [PageBlockDragPreviewRow],
    pub(in crate::ui::board_workspace::page::editor) rows_from_anchor:
        &'a [PageBlockDragPreviewRow],
    pub(in crate::ui::board_workspace::page::editor) overflow_count: usize,
}

impl PageBlockDragPayload {
    pub(in crate::ui::board_workspace::page::editor) fn visible_preview(
        &self,
    ) -> PageBlockDragVisiblePreview<'_> {
        let resolved = self.resolve();
        assert!(
            resolved.preview_anchor_index < resolved.preview_rows.rows.len(),
            "page block drag preview anchor must resolve to a preview row"
        );
        let anchor = resolved.preview_anchor_index;
        PageBlockDragVisiblePreview {
            rows_before_anchor: &resolved.preview_rows.rows[..anchor],
            rows_from_anchor: &resolved.preview_rows.rows[anchor..],
            overflow_count: resolved.preview_rows.overflow_count,
        }
    }
}

pub(super) fn page_block_drag_selected_ids(
    data: &LoadedCardPageData,
    block_selection: &[String],
    text_selection: Option<&PageTextSelection>,
    dragged_block_id: &str,
) -> Vec<String> {
    if block_selection
        .iter()
        .any(|block_id| block_id == dragged_block_id)
    {
        return block_selection.to_vec();
    }
    let Some(selection) = text_selection else {
        return Vec::new();
    };
    let Some(visible_selection) =
        super::super::selection::VisiblePageTextSelection::new(data, selection)
    else {
        return Vec::new();
    };
    let dragged = data
        .page
        .blocks
        .iter()
        .position(|block| block.block_id == dragged_block_id);
    if !dragged.is_some_and(|index| visible_selection.contains_visible_block_index(index)) {
        return Vec::new();
    }
    visible_selection
        .block_indices()
        .map(|index| &data.page.blocks[index])
        .filter(|block| {
            !block.is_layout_container()
                && !block.is_opaque_unavailable()
                && block.simple_table_row_content().is_none()
        })
        .map(|block| block.block_id.clone())
        .collect()
}

pub(super) fn page_block_drag_root_ids(
    blocks: &[CardPageBlock],
    selected_block_ids: &[String],
    dragged_block_id: &str,
) -> Vec<String> {
    if !selected_block_ids
        .iter()
        .any(|block_id| block_id == dragged_block_id)
    {
        return vec![dragged_block_id.to_string()];
    }
    let selected = selected_block_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut roots = Vec::new();
    let mut covered_until = 0;
    for (index, block) in blocks.iter().enumerate() {
        if index >= covered_until && selected.contains(block.block_id.as_str()) {
            roots.push(block.block_id.clone());
            covered_until = page_block_subtree_end(blocks, index);
        }
    }
    roots
}

pub(super) fn page_block_drag_preview_rows(
    data: &LoadedCardPageData,
    dragged_index: usize,
    root_block_ids: &[String],
    subtree_block_ids: &[String],
    block_images: &NotionBlockImageCache,
) -> (PageBlockDragPreviewRows, usize) {
    let root_block_ids = root_block_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let subtree_block_ids = subtree_block_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let base_depth = page_block_drag_preview_base_depth(data, &root_block_ids);
    let rows = page_block_drag_preview_source_rows(data, &subtree_block_ids);
    let window = PageBlockDragPreviewWindow::new(&rows, dragged_index);
    let visible_rows = rows
        .iter()
        .skip(window.start)
        .take(window.visible_count)
        .map(|row| page_block_drag_preview_row(data, row, base_depth, block_images))
        .collect();
    (
        PageBlockDragPreviewRows {
            rows: visible_rows,
            overflow_count: window.overflow_count,
        },
        window.anchor_index,
    )
}

fn page_block_drag_preview_base_depth(
    data: &LoadedCardPageData,
    root_block_ids: &HashSet<&str>,
) -> usize {
    data.visible_rows
        .iter()
        .filter(|row| root_block_ids.contains(data.page.blocks[row.block_index].block_id.as_str()))
        .map(|row| row.visual_depth)
        .min()
        .expect("page block drag roots must resolve")
}

fn page_block_drag_preview_source_rows(
    data: &LoadedCardPageData,
    subtree_block_ids: &HashSet<&str>,
) -> Vec<PageBlockDragPreviewSourceRow> {
    data.visible_rows
        .iter()
        .enumerate()
        .filter(|row| {
            subtree_block_ids.contains(data.page.blocks[row.1.block_index].block_id.as_str())
        })
        .map(|(visible_row_index, row)| PageBlockDragPreviewSourceRow {
            visible_row_index,
            block_index: row.block_index,
            visual_depth: row.visual_depth,
        })
        .collect()
}

impl PageBlockDragPreviewWindow {
    fn new(rows: &[PageBlockDragPreviewSourceRow], dragged_index: usize) -> Self {
        let full_anchor_index = rows
            .iter()
            .position(|row| row.block_index == dragged_index)
            .expect("dragged page block must appear in its preview");
        let visible_count = rows.len().min(PAGE_BLOCK_DRAG_PREVIEW_VISIBLE_ROWS);
        let start = full_anchor_index
            .saturating_sub(visible_count / 2)
            .min(rows.len() - visible_count);
        Self {
            start,
            visible_count,
            overflow_count: rows.len() - visible_count,
            anchor_index: full_anchor_index - start,
        }
    }
}

fn page_block_drag_preview_row(
    data: &LoadedCardPageData,
    source: &PageBlockDragPreviewSourceRow,
    base_depth: usize,
    block_images: &NotionBlockImageCache,
) -> PageBlockDragPreviewRow {
    let PageBlockDragPreviewSourceRow {
        visible_row_index,
        block_index: index,
        visual_depth,
    } = *source;
    let block = &data.page.blocks[index];
    let cached_block_image = block
        .resource_content()
        .and_then(|resource| resource.image().source().fetch_key())
        .and_then(|key| block_images.cached(key));
    let simple_table_rows = data.document_unit_ranges[visible_row_index]
        .clone()
        .take(2)
        .filter_map(
            |document_unit_index| match &data.document_units[document_unit_index] {
                LoadedCardPageDocumentUnit::SimpleTableRow { cells, .. } => Some(cells.clone()),
                LoadedCardPageDocumentUnit::Block { .. } => None,
            },
        )
        .collect::<Vec<_>>()
        .into();
    PageBlockDragPreviewRow {
        content: block.content.clone(),
        color: block.color,
        numbered_index: data.numbered_indices[index],
        depth: visual_depth.saturating_sub(base_depth),
        row_spacing: block
            .editable_content()
            .map(|_| page_block_visible_row_spacing(data, visible_row_index)),
        format: data.page.format,
        cached_block_image,
        simple_table_rows,
    }
}

pub(super) fn page_block_subtree_ids(
    blocks: &[CardPageBlock],
    root_block_ids: &[String],
) -> Option<Vec<String>> {
    let root_ids = root_block_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut subtree_block_ids = Vec::new();
    let mut found_roots = 0;
    let mut index = 0;
    while index < blocks.len() {
        if root_ids.contains(blocks[index].block_id.as_str()) {
            found_roots += 1;
            let subtree_end = page_block_subtree_end(blocks, index);
            subtree_block_ids.extend(
                blocks[index..subtree_end]
                    .iter()
                    .map(|block| block.block_id.clone()),
            );
            index = subtree_end;
        } else {
            index += 1;
        }
    }
    (found_roots == root_ids.len()).then_some(subtree_block_ids)
}
