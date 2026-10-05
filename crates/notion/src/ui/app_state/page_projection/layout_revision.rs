use std::collections::HashMap;

use crate::ui::{CardPageBlock, CardPageBlockKind};

use super::{
    LoadedCardPageData, LoadedCardPageDocumentUnit, LoadedCardPageVisibleRow,
    PageDocumentUnitLayoutRevision, PageFlowCalloutLayoutRevision,
};

mod decorators;

use decorators::reuse_decorator_layout_revisions;

/// Layout revisions for each document unit and each visible row's callout.
type PageLayoutRevisions = (
    Vec<PageDocumentUnitLayoutRevision>,
    Vec<Option<PageFlowCalloutLayoutRevision>>,
);

pub(super) fn fresh_page_layout_revisions(
    page: &crate::ui::CardPage,
    visible_rows: &[LoadedCardPageVisibleRow],
    document_unit_count: usize,
) -> PageLayoutRevisions {
    let document_units = (0..document_unit_count)
        .map(|_| PageDocumentUnitLayoutRevision::fresh())
        .collect();
    let callouts = decorators::fresh_callout_layout_revisions(page, visible_rows);
    (document_units, callouts)
}

impl LoadedCardPageData {
    pub(crate) fn reuse_document_unit_layout_revisions(&mut self, previous: &Self) {
        if self.page.block_id != previous.page.block_id {
            return;
        }
        let previous_rows = visible_rows_by_block_id(previous);
        let equivalent_rows = equivalent_visible_rows(self, previous, &previous_rows);
        let previous_units = previous
            .document_units
            .iter()
            .enumerate()
            .map(|(index, unit)| (unit.key(), index))
            .collect::<HashMap<_, _>>();
        let boundaries = CalloutUnitBoundaries::new(self);
        let previous_boundaries = CalloutUnitBoundaries::new(previous);
        for (index, unit) in self.document_units.iter().enumerate() {
            let Some(&previous_index) = previous_units.get(unit.key()) else {
                continue;
            };
            if document_units_have_same_layout(
                UnitLayoutComparison {
                    data: self,
                    index,
                    boundaries: &boundaries,
                },
                UnitLayoutComparison {
                    data: previous,
                    index: previous_index,
                    boundaries: &previous_boundaries,
                },
                &equivalent_rows,
            ) {
                self.document_unit_layout_revisions[index] =
                    previous.document_unit_layout_revisions[previous_index].clone();
            }
        }
        reuse_decorator_layout_revisions(self, previous, &equivalent_rows);
    }
}

fn visible_rows_by_block_id(data: &LoadedCardPageData) -> HashMap<&str, usize> {
    data.visible_rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            (
                data.page.blocks[row.block_index].block_id.as_str(),
                row_index,
            )
        })
        .collect()
}

fn equivalent_visible_rows(
    data: &LoadedCardPageData,
    previous: &LoadedCardPageData,
    previous_rows: &HashMap<&str, usize>,
) -> Vec<Option<usize>> {
    let mut equivalent = vec![None; data.visible_rows.len()];
    for (row_index, row) in data.visible_rows.iter().enumerate() {
        let block = &data.page.blocks[row.block_index];
        let Some(&previous_row_index) = previous_rows.get(block.block_id.as_str()) else {
            continue;
        };
        if visible_rows_have_same_layout(data, row_index, previous, previous_row_index, &equivalent)
        {
            equivalent[row_index] = Some(previous_row_index);
        }
    }
    equivalent
}

fn visible_rows_have_same_layout(
    data: &LoadedCardPageData,
    row_index: usize,
    previous: &LoadedCardPageData,
    previous_row_index: usize,
    equivalent: &[Option<usize>],
) -> bool {
    let row = data.visible_rows[row_index];
    let previous_row = previous.visible_rows[previous_row_index];
    let block = &data.page.blocks[row.block_index];
    let previous_block = &previous.page.blocks[previous_row.block_index];
    blocks_have_same_render_authority(block, previous_block)
        && row.visual_depth == previous_row.visual_depth
        && row.joins_previous_list_sibling == previous_row.joins_previous_list_sibling
        && row.joins_next_list_sibling == previous_row.joins_next_list_sibling
        && data.numbered_indices[row.block_index]
            == previous.numbered_indices[previous_row.block_index]
        && data.inherited_text_colors[row_index]
            == previous.inherited_text_colors[previous_row_index]
        && data.effective_callout_colors[row_index]
            == previous.effective_callout_colors[previous_row_index]
        && data.empty_toggle_placeholder_mask[row_index]
            == previous.empty_toggle_placeholder_mask[previous_row_index]
        && linked_visible_rows_match(
            row.visible_parent_row_index,
            previous_row.visible_parent_row_index,
            equivalent,
        )
        && linked_visible_rows_match(
            row.callout_parent_row_index,
            previous_row.callout_parent_row_index,
            equivalent,
        )
        && referenced_blocks_match(
            data,
            row.column_block_index,
            previous,
            previous_row.column_block_index,
        )
        && first_callout_input_ids_match(data, row_index, previous, previous_row_index)
        && next_row_list_state(data, row_index) == next_row_list_state(previous, previous_row_index)
}

fn blocks_have_same_render_authority(left: &CardPageBlock, right: &CardPageBlock) -> bool {
    left.block_id == right.block_id
        && left.parent_block_id == right.parent_block_id
        && left.depth == right.depth
        && left.color == right.color
        && left.icon == right.icon
        && left.content == right.content
}

fn linked_visible_rows_match(
    row: Option<usize>,
    previous_row: Option<usize>,
    equivalent: &[Option<usize>],
) -> bool {
    match (row, previous_row) {
        (None, None) => true,
        (Some(row), Some(previous_row)) => equivalent.get(row) == Some(&Some(previous_row)),
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn referenced_blocks_match(
    data: &LoadedCardPageData,
    block_index: Option<usize>,
    previous: &LoadedCardPageData,
    previous_block_index: Option<usize>,
) -> bool {
    match (block_index, previous_block_index) {
        (None, None) => true,
        (Some(index), Some(previous_index)) => {
            data.page.blocks[index].block_id == previous.page.blocks[previous_index].block_id
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn first_callout_input_ids_match(
    data: &LoadedCardPageData,
    row_index: usize,
    previous: &LoadedCardPageData,
    previous_row_index: usize,
) -> bool {
    referenced_row_block_id(data, data.callout_first_text_input_row_indices[row_index])
        == referenced_row_block_id(
            previous,
            previous.callout_first_text_input_row_indices[previous_row_index],
        )
}

fn referenced_row_block_id(data: &LoadedCardPageData, row_index: Option<usize>) -> Option<&str> {
    row_index.map(|index| {
        let row = data.visible_rows[index];
        data.page.blocks[row.block_index].block_id.as_str()
    })
}

fn next_row_list_state(data: &LoadedCardPageData, row_index: usize) -> bool {
    data.visible_rows
        .get(row_index + 1)
        .and_then(|row| data.page.blocks[row.block_index].editable_content())
        .is_some_and(|editable| {
            matches!(
                editable.kind,
                CardPageBlockKind::BulletedList
                    | CardPageBlockKind::NumberedList
                    | CardPageBlockKind::ToDoList
                    | CardPageBlockKind::ToggleList
            )
        })
}

struct CalloutUnitBoundaries {
    starts: Vec<Vec<usize>>,
    ends: Vec<Vec<usize>>,
}

impl CalloutUnitBoundaries {
    fn new(data: &LoadedCardPageData) -> Self {
        let mut starts = vec![Vec::new(); data.document_units.len()];
        let mut ends = vec![Vec::new(); data.document_units.len()];
        for (row_index, row) in data.visible_rows.iter().enumerate() {
            let block = &data.page.blocks[row.block_index];
            if !block
                .editable_content()
                .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
            {
                continue;
            }
            let range = &data.callout_render_unit_ranges[row_index];
            if range.is_empty() {
                continue;
            }
            starts[range.start].push(row_index);
            ends[range.end - 1].push(row_index);
        }
        Self { starts, ends }
    }
}

struct UnitLayoutComparison<'a> {
    data: &'a LoadedCardPageData,
    index: usize,
    boundaries: &'a CalloutUnitBoundaries,
}

fn document_units_have_same_layout(
    unit: UnitLayoutComparison<'_>,
    previous: UnitLayoutComparison<'_>,
    equivalent_rows: &[Option<usize>],
) -> bool {
    let owner = unit.data.document_units[unit.index].owner_visible_row_index();
    let previous_owner = previous.data.document_units[previous.index].owner_visible_row_index();
    equivalent_rows.get(owner) == Some(&Some(previous_owner))
        && document_unit_payloads_match(unit.data, unit.index, previous.data, previous.index)
        && boundary_rows_match(
            unit.data,
            &unit.boundaries.starts[unit.index],
            previous.data,
            &previous.boundaries.starts[previous.index],
            equivalent_rows,
        )
        && boundary_rows_match(
            unit.data,
            &unit.boundaries.ends[unit.index],
            previous.data,
            &previous.boundaries.ends[previous.index],
            equivalent_rows,
        )
}

fn document_unit_payloads_match(
    data: &LoadedCardPageData,
    index: usize,
    previous: &LoadedCardPageData,
    previous_index: usize,
) -> bool {
    match (
        &data.document_units[index],
        &previous.document_units[previous_index],
    ) {
        (LoadedCardPageDocumentUnit::Block { .. }, LoadedCardPageDocumentUnit::Block { .. }) => {
            true
        }
        (
            LoadedCardPageDocumentUnit::SimpleTableRow {
                row_block_index,
                ordinal,
                position,
                cells,
                ..
            },
            LoadedCardPageDocumentUnit::SimpleTableRow {
                row_block_index: previous_row_block_index,
                ordinal: previous_ordinal,
                position: previous_position,
                cells: previous_cells,
                ..
            },
        ) => {
            ordinal == previous_ordinal
                && position == previous_position
                && cells == previous_cells
                && blocks_have_same_render_authority(
                    &data.page.blocks[*row_block_index],
                    &previous.page.blocks[*previous_row_block_index],
                )
        }
        (
            LoadedCardPageDocumentUnit::Block { .. },
            LoadedCardPageDocumentUnit::SimpleTableRow { .. },
        )
        | (
            LoadedCardPageDocumentUnit::SimpleTableRow { .. },
            LoadedCardPageDocumentUnit::Block { .. },
        ) => false,
    }
}

fn boundary_rows_match(
    data: &LoadedCardPageData,
    rows: &[usize],
    previous: &LoadedCardPageData,
    previous_rows: &[usize],
    equivalent_rows: &[Option<usize>],
) -> bool {
    rows.len() == previous_rows.len()
        && rows.iter().zip(previous_rows).all(|(row, previous_row)| {
            equivalent_rows.get(*row) == Some(&Some(*previous_row))
                && referenced_row_block_id(data, Some(*row))
                    == referenced_row_block_id(previous, Some(*previous_row))
        })
}
