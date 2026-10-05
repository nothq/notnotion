use gpui::{px, Bounds, Pixels, Point};

use super::super::{
    LoadedCardPageData, PageBlockDragLayout, PageBlockDragLayouts, PageBlockDragScrollTarget,
    PAGE_BLOCK_GUTTER_WIDTH,
};

struct PageBlockDragMeasuredRow {
    document_unit_index: usize,
    owner_visible_row_index: usize,
    bounds: Bounds<Pixels>,
    block_id: String,
}

pub(super) fn merge_page_block_drag_rows(
    layout: &mut PageBlockDragLayout,
    data: &LoadedCardPageData,
    document_unit_indices: Vec<usize>,
    row_bounds: Vec<Bounds<Pixels>>,
    generation: u64,
) {
    assert_eq!(
        document_unit_indices.len(),
        row_bounds.len(),
        "Notion page section must report one bound per document unit"
    );
    if layout.generation != generation {
        layout.generation = generation;
        layout.document_unit_indices.clear();
        layout.owner_visible_row_indices.clear();
        layout.row_bounds.clear();
        layout.owner_block_ids.clear();
    }
    let mut rows = page_block_drag_measured_rows(layout);
    for (document_unit_index, bounds) in document_unit_indices.into_iter().zip(row_bounds) {
        let measured_row = PageBlockDragMeasuredRow::new(data, document_unit_index, bounds);
        match rows
            .iter_mut()
            .find(|row| row.document_unit_index == document_unit_index)
        {
            Some(row) => *row = measured_row,
            None => rows.push(measured_row),
        }
    }
    rows.sort_by_key(|row| row.document_unit_index);
    replace_page_block_drag_layout_rows(layout, rows);
}

fn page_block_drag_measured_rows(layout: &PageBlockDragLayout) -> Vec<PageBlockDragMeasuredRow> {
    layout
        .document_unit_indices
        .iter()
        .copied()
        .zip(layout.owner_visible_row_indices.iter().copied())
        .zip(layout.row_bounds.iter().copied())
        .zip(layout.owner_block_ids.iter().cloned())
        .map(
            |(((document_unit_index, owner_visible_row_index), bounds), block_id)| {
                PageBlockDragMeasuredRow {
                    document_unit_index,
                    owner_visible_row_index,
                    bounds,
                    block_id,
                }
            },
        )
        .collect()
}

fn replace_page_block_drag_layout_rows(
    layout: &mut PageBlockDragLayout,
    rows: Vec<PageBlockDragMeasuredRow>,
) {
    layout.document_unit_indices = rows.iter().map(|row| row.document_unit_index).collect();
    layout.owner_visible_row_indices = rows.iter().map(|row| row.owner_visible_row_index).collect();
    layout.row_bounds = rows.iter().map(|row| row.bounds).collect();
    layout.owner_block_ids = rows.into_iter().map(|row| row.block_id).collect();
}

impl PageBlockDragMeasuredRow {
    fn new(data: &LoadedCardPageData, document_unit_index: usize, bounds: Bounds<Pixels>) -> Self {
        let owner_visible_row_index =
            data.document_units[document_unit_index].owner_visible_row_index();
        let row = &data.visible_rows[owner_visible_row_index];
        Self {
            document_unit_index,
            owner_visible_row_index,
            bounds,
            block_id: data.page.blocks[row.block_index].block_id.clone(),
        }
    }
}

impl PageBlockDragLayouts {
    pub(super) fn get(&self, target: PageBlockDragScrollTarget) -> &PageBlockDragLayout {
        match target {
            PageBlockDragScrollTarget::Standalone => &self.standalone,
            PageBlockDragScrollTarget::SelectedPage => &self.selected_page,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn get_mut(
        &mut self,
        target: PageBlockDragScrollTarget,
    ) -> &mut PageBlockDragLayout {
        match target {
            PageBlockDragScrollTarget::Standalone => &mut self.standalone,
            PageBlockDragScrollTarget::SelectedPage => &mut self.selected_page,
        }
    }

    pub(super) fn hovered_block_id(
        &self,
        target: PageBlockDragScrollTarget,
        position: Point<Pixels>,
    ) -> Option<String> {
        self.get(target)
            .hovered_block_id(position)
            .map(str::to_string)
    }
}

impl PageBlockDragLayout {
    fn hovered_block_id(&self, position: Point<Pixels>) -> Option<&str> {
        self.row_bounds
            .iter()
            .zip(&self.owner_block_ids)
            .rfind(|(bounds, _)| {
                position.x >= bounds.left() - px(PAGE_BLOCK_GUTTER_WIDTH)
                    && position.x <= bounds.right()
                    && position.y >= bounds.top()
                    && position.y <= bounds.bottom()
            })
            .map(|(_, block_id)| block_id.as_str())
    }
}
