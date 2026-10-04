use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::LoadedCardPageSimpleTableCellAccess;

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace::page::editor) enum PageSimpleTableCellNavigation {
    Left,
    Right,
    Up,
    Down,
    Previous,
    Next,
}

pub(in crate::ui::board_workspace::page::editor) fn simple_table_navigation_target(
    data: &crate::ui::LoadedCardPageData,
    address: &CardPageSimpleTableCellAddress,
    navigation: PageSimpleTableCellNavigation,
) -> Option<CardPageSimpleTableCellAddress> {
    let location = data.simple_table_cell_location(address)?;
    let current = simple_table_row_cells(data, address, location.document_unit_index)?;
    let (unit_index, column_index) = match navigation {
        PageSimpleTableCellNavigation::Left | PageSimpleTableCellNavigation::Previous
            if location.column_index > 0 =>
        {
            (location.document_unit_index, location.column_index - 1)
        }
        PageSimpleTableCellNavigation::Right | PageSimpleTableCellNavigation::Next
            if location.column_index + 1 < current.len() =>
        {
            (location.document_unit_index, location.column_index + 1)
        }
        PageSimpleTableCellNavigation::Left | PageSimpleTableCellNavigation::Previous => (
            location.document_unit_index.checked_sub(1)?,
            current.len().checked_sub(1)?,
        ),
        PageSimpleTableCellNavigation::Right | PageSimpleTableCellNavigation::Next => {
            (location.document_unit_index.checked_add(1)?, 0)
        }
        PageSimpleTableCellNavigation::Up => (
            location.document_unit_index.checked_sub(1)?,
            location.column_index,
        ),
        PageSimpleTableCellNavigation::Down => (
            location.document_unit_index.checked_add(1)?,
            location.column_index,
        ),
    };
    let target = simple_table_row_cells(data, address, unit_index)?
        .get(column_index)?
        .address
        .clone();
    data.projected_simple_table_cell(&target)
        .filter(|cell| cell.access == LoadedCardPageSimpleTableCellAccess::Writable)
        .map(|_| target)
}

fn simple_table_row_cells<'a>(
    data: &'a crate::ui::LoadedCardPageData,
    address: &CardPageSimpleTableCellAddress,
    document_unit_index: usize,
) -> Option<&'a [crate::ui::LoadedCardPageSimpleTableCell]> {
    let crate::ui::LoadedCardPageDocumentUnit::SimpleTableRow { cells, .. } =
        data.document_units.get(document_unit_index)?
    else {
        return None;
    };
    cells
        .first()
        .is_some_and(|cell| cell.address.table_block_id() == address.table_block_id())
        .then_some(cells)
}
