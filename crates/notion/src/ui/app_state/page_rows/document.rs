use std::{collections::HashMap, ops::Range, sync::Arc};

use gpui::SharedString;

use crate::model::{
    CardPageSimpleTableCellAddress, CardPageSimpleTableCellRoundTrip, CardPageTextAnnotationSpan,
    PageTextAnnotation,
};
use crate::ui::{CardPage, CardPageBlock, CardPageBlockKind};

use super::super::{
    LoadedCardPageDocumentUnit, LoadedCardPageSimpleTableAnnotationRun,
    LoadedCardPageSimpleTableCell, LoadedCardPageSimpleTableCellAccess,
    LoadedCardPageSimpleTableCellLocation, LoadedCardPageSimpleTableRowPosition,
    PageDocumentUnitKey,
};
use super::CanonicalProjection;

pub(super) struct DocumentProjection {
    pub(super) text_input_block_mask: Vec<bool>,
    pub(super) document_units: Vec<LoadedCardPageDocumentUnit>,
    pub(super) document_unit_ranges: Vec<Range<usize>>,
    pub(super) simple_table_cell_locations:
        HashMap<CardPageSimpleTableCellAddress, LoadedCardPageSimpleTableCellLocation>,
}

pub(super) fn build_document_projection(
    page: &CardPage,
    canonical: &CanonicalProjection,
) -> DocumentProjection {
    let text_input_block_mask = build_text_input_block_mask(page, canonical);
    let block_indices = page
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.block_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut document_units = Vec::new();
    let mut simple_table_cell_locations = HashMap::new();
    let mut document_unit_ranges = Vec::with_capacity(canonical.visible_rows.len());
    for (visible_row_index, row) in canonical.visible_rows.iter().enumerate() {
        let start = document_units.len();
        let block = &page.blocks[row.block_index];
        if let Some(table) = block.simple_table_content() {
            append_table_document_units(
                page,
                &block_indices,
                VisibleTable {
                    block_id: &block.block_id,
                    content: table,
                    visible_row_index,
                },
                &mut document_units,
                &mut simple_table_cell_locations,
            );
        } else if !block.is_editable() || text_input_block_mask[row.block_index] {
            document_units.push(LoadedCardPageDocumentUnit::Block {
                key: PageDocumentUnitKey::Block {
                    block_id: Arc::from(block.block_id.as_str()),
                },
                owner_visible_row_index: visible_row_index,
            });
        }
        document_unit_ranges.push(start..document_units.len());
    }
    DocumentProjection {
        text_input_block_mask,
        document_units,
        document_unit_ranges,
        simple_table_cell_locations,
    }
}

/// A table block and the visible row that shows it.
#[derive(Clone, Copy)]
struct VisibleTable<'a> {
    block_id: &'a str,
    content: &'a crate::model::CardPageSimpleTableBlock,
    visible_row_index: usize,
}

/// A table row's block, its position among the table's rows, and its document
/// unit.
#[derive(Clone, Copy)]
struct TableRowUnit {
    block_index: usize,
    ordinal: usize,
    document_unit_index: usize,
}

fn append_table_document_units(
    page: &CardPage,
    block_indices: &HashMap<&str, usize>,
    visible_table: VisibleTable<'_>,
    document_units: &mut Vec<LoadedCardPageDocumentUnit>,
    cell_locations: &mut HashMap<
        CardPageSimpleTableCellAddress,
        LoadedCardPageSimpleTableCellLocation,
    >,
) {
    let VisibleTable {
        block_id: table_block_id,
        content: table,
        visible_row_index,
    } = visible_table;
    let row_block_indices = table
        .row_block_ids()
        .iter()
        .filter_map(|row_block_id| {
            let block_index = *block_indices
                .get(row_block_id.as_str())
                .expect("validated Notion table child must resolve by its content ID");
            (!page.blocks[block_index].is_opaque_unavailable()).then_some(block_index)
        })
        .collect::<Vec<_>>();
    for (ordinal, row_block_index) in row_block_indices.iter().copied().enumerate() {
        let document_unit_index = document_units.len();
        let cells = project_table_row_cells(
            page,
            visible_table,
            TableRowUnit {
                block_index: row_block_index,
                ordinal,
                document_unit_index,
            },
            cell_locations,
        );
        document_units.push(LoadedCardPageDocumentUnit::SimpleTableRow {
            key: PageDocumentUnitKey::SimpleTableRow {
                table_block_id: Arc::from(table_block_id),
                row_block_id: Arc::from(page.blocks[row_block_index].block_id.as_str()),
            },
            owner_visible_row_index: visible_row_index,
            row_block_index,
            ordinal,
            position: table_row_position(ordinal, row_block_indices.len()),
            cells,
        });
    }
}

fn project_table_row_cells(
    page: &CardPage,
    visible_table: VisibleTable<'_>,
    row: TableRowUnit,
    cell_locations: &mut HashMap<
        CardPageSimpleTableCellAddress,
        LoadedCardPageSimpleTableCellLocation,
    >,
) -> Arc<[LoadedCardPageSimpleTableCell]> {
    let VisibleTable {
        block_id: table_block_id,
        content: table,
        ..
    } = visible_table;
    let TableRowUnit {
        block_index: row_block_index,
        ordinal: row_ordinal,
        document_unit_index,
    } = row;
    let row = page.blocks[row_block_index]
        .simple_table_row_content()
        .expect("validated Notion table child must contain table-row content");
    row.cells()
        .iter()
        .zip(table.columns())
        .enumerate()
        .map(|(column_index, (cell, column))| {
            let address = CardPageSimpleTableCellAddress::new(
                table_block_id.to_owned(),
                page.blocks[row_block_index].block_id.clone(),
                column.id().clone(),
            )
            .expect("validated table projection must form typed cell addresses");
            cell_locations.insert(
                address.clone(),
                LoadedCardPageSimpleTableCellLocation {
                    document_unit_index,
                    row_ordinal,
                    column_index,
                },
            );
            LoadedCardPageSimpleTableCell {
                address,
                access: match cell.round_trip() {
                    CardPageSimpleTableCellRoundTrip::Writable => {
                        LoadedCardPageSimpleTableCellAccess::Writable
                    }
                    CardPageSimpleTableCellRoundTrip::ReadOnly(_) => {
                        LoadedCardPageSimpleTableCellAccess::ReadOnly
                    }
                },
                text: SharedString::from(cell.text().to_owned()),
                annotation_runs: simple_table_annotation_runs(cell.annotations()),
            }
        })
        .collect::<Vec<_>>()
        .into()
}

fn simple_table_annotation_runs(
    spans: &[CardPageTextAnnotationSpan],
) -> Arc<[LoadedCardPageSimpleTableAnnotationRun]> {
    let mut boundaries = spans
        .iter()
        .flat_map(|span| [span.start_utf8, span.end_utf8])
        .collect::<Vec<_>>();
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
        .windows(2)
        .filter_map(|boundary| {
            let range = boundary[0]..boundary[1];
            let mut run = LoadedCardPageSimpleTableAnnotationRun {
                range: range.clone(),
                bold: false,
                italic: false,
                underline: false,
                strikethrough: false,
                code: false,
                link: false,
                text_color: None,
                background_color: None,
            };
            let mut annotated = false;
            for annotation in spans
                .iter()
                .filter(|span| span.start_utf8 <= range.start && span.end_utf8 >= range.end)
                .map(|span| &span.annotation)
            {
                annotated = true;
                match annotation {
                    PageTextAnnotation::Bold => run.bold = true,
                    PageTextAnnotation::Italic => run.italic = true,
                    PageTextAnnotation::Underline => run.underline = true,
                    PageTextAnnotation::Strike => run.strikethrough = true,
                    PageTextAnnotation::Code => run.code = true,
                    PageTextAnnotation::Link(_) => run.link = true,
                    PageTextAnnotation::TextColor(color) => run.text_color = Some(*color),
                    PageTextAnnotation::BackgroundColor(color) => {
                        run.background_color = Some(*color)
                    }
                    // Simple-table cells with mention tokens stay read-only and
                    // render the flattened text, so the run needs no styling.
                    PageTextAnnotation::Mention(_) => {}
                }
            }
            annotated.then_some(run)
        })
        .collect::<Vec<_>>()
        .into()
}

const fn table_row_position(
    ordinal: usize,
    row_count: usize,
) -> LoadedCardPageSimpleTableRowPosition {
    match (ordinal == 0, ordinal + 1 == row_count) {
        (true, true) => LoadedCardPageSimpleTableRowPosition::Only,
        (true, false) => LoadedCardPageSimpleTableRowPosition::First,
        (false, true) => LoadedCardPageSimpleTableRowPosition::Last,
        (false, false) => LoadedCardPageSimpleTableRowPosition::Middle,
    }
}

fn build_text_input_block_mask(page: &CardPage, canonical: &CanonicalProjection) -> Vec<bool> {
    let mut mask = page
        .blocks
        .iter()
        .map(CardPageBlock::is_editable)
        .collect::<Vec<_>>();
    for (visible_row_index, children) in canonical.callout_child_rows.iter().enumerate() {
        if children.is_empty() {
            continue;
        }
        let block_index = canonical.visible_rows[visible_row_index].block_index;
        if page.blocks[block_index]
            .editable_content()
            .is_some_and(|editable| {
                editable.kind == CardPageBlockKind::Callout && editable.text.is_empty()
            })
        {
            mask[block_index] = false;
        }
    }
    mask
}
