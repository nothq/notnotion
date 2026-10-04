use std::collections::HashMap;

use super::{
    CardPageSimpleTableCellAddress, CardPageSimpleTableColumnId, CardPageWritableSimpleTableCell,
};
use crate::model::CardPage;

struct IndexedSimpleTable {
    table_block_index: usize,
    row_ordinals: HashMap<String, usize>,
    column_indices: HashMap<CardPageSimpleTableColumnId, usize>,
}

pub(crate) struct CardPageSimpleTableCellIndex {
    block_indices: HashMap<String, usize>,
    tables: HashMap<String, IndexedSimpleTable>,
}

#[derive(Clone, Copy)]
pub(crate) struct ResolvedCardPageSimpleTableCell {
    row_block_index: usize,
    column_index: usize,
}

impl CardPageSimpleTableCellIndex {
    pub(crate) fn new(page: &CardPage) -> Result<Self, String> {
        let mut block_indices = HashMap::with_capacity(page.blocks.len());
        let mut tables = HashMap::new();
        for (block_index, block) in page.blocks.iter().enumerate() {
            if block_indices
                .insert(block.block_id.clone(), block_index)
                .is_some()
            {
                return Err(format!(
                    "page {} contains duplicate block {}",
                    page.block_id, block.block_id
                ));
            }
            let Some(table) = block.simple_table_content() else {
                continue;
            };
            tables.insert(
                block.block_id.clone(),
                IndexedSimpleTable {
                    table_block_index: block_index,
                    row_ordinals: table
                        .row_block_ids()
                        .iter()
                        .enumerate()
                        .map(|(index, id)| (id.clone(), index))
                        .collect(),
                    column_indices: table
                        .columns()
                        .iter()
                        .enumerate()
                        .map(|(index, column)| (column.id().clone(), index))
                        .collect(),
                },
            );
        }
        Ok(Self {
            block_indices,
            tables,
        })
    }

    pub(crate) fn resolve(
        &self,
        page: &CardPage,
        address: &CardPageSimpleTableCellAddress,
    ) -> Result<ResolvedCardPageSimpleTableCell, String> {
        let indexed = self.tables.get(address.table_block_id()).ok_or_else(|| {
            format!(
                "block {} is not a loaded Notion table",
                address.table_block_id()
            )
        })?;
        let row_ordinal = indexed
            .row_ordinals
            .get(address.row_block_id())
            .copied()
            .ok_or_else(|| table_address_error(address, "does not declare the requested row"))?;
        let column_index = indexed
            .column_indices
            .get(address.column_id())
            .copied()
            .ok_or_else(|| table_address_error(address, "does not declare the requested column"))?;
        let row_block_index = self
            .block_indices
            .get(address.row_block_id())
            .copied()
            .ok_or_else(|| table_address_error(address, "contains an unavailable requested row"))?;
        let resolved = ResolvedCardPageSimpleTableCell {
            row_block_index,
            column_index,
        };
        validate_resolved_row(page, address, indexed, row_ordinal, resolved)?;
        Ok(resolved)
    }

    pub(crate) fn replace(
        &self,
        page: &mut CardPage,
        address: &CardPageSimpleTableCellAddress,
        cell: CardPageWritableSimpleTableCell,
    ) -> Result<(), String> {
        let resolved = self.resolve(page, address)?;
        let row = page
            .blocks
            .get_mut(resolved.row_block_index)
            .and_then(|block| block.simple_table_row_content_mut())
            .ok_or_else(|| table_address_error(address, "does not resolve to a mutable row"))?;
        CardPageWritableSimpleTableCell::try_from(row.cells()[resolved.column_index].clone())?;
        row.replace_cell(resolved.column_index, cell.into_cell())
    }

    pub(crate) fn cell<'a>(
        &self,
        page: &'a CardPage,
        address: &CardPageSimpleTableCellAddress,
    ) -> Result<&'a super::CardPageSimpleTableCell, String> {
        let resolved = self.resolve(page, address)?;
        Ok(&page.blocks[resolved.row_block_index]
            .simple_table_row_content()
            .expect("resolved Notion table row must contain row content")
            .cells()[resolved.column_index])
    }
}

fn validate_resolved_row(
    page: &CardPage,
    address: &CardPageSimpleTableCellAddress,
    indexed: &IndexedSimpleTable,
    row_ordinal: usize,
    resolved: ResolvedCardPageSimpleTableCell,
) -> Result<(), String> {
    let ResolvedCardPageSimpleTableCell {
        row_block_index,
        column_index,
    } = resolved;
    let table_block = &page.blocks[indexed.table_block_index];
    let table = table_block
        .simple_table_content()
        .ok_or_else(|| table_address_error(address, "no longer resolves to a table root"))?;
    if table.row_block_ids().get(row_ordinal).map(String::as_str) != Some(address.row_block_id()) {
        return Err(table_address_error(address, "has conflicting row order"));
    }
    if table.columns().get(column_index).map(|column| column.id()) != Some(address.column_id()) {
        return Err(table_address_error(address, "has conflicting column order"));
    }
    let row_block = &page.blocks[row_block_index];
    let expected_depth = table_block
        .depth
        .checked_add(1)
        .ok_or_else(|| table_address_error(address, "has an invalid root depth"))?;
    if row_block.parent_block_id != table_block.block_id || row_block.depth != expected_depth {
        return Err(table_address_error(
            address,
            "does not own the row directly",
        ));
    }
    let row = row_block
        .simple_table_row_content()
        .ok_or_else(|| table_address_error(address, "row content is opaque or unavailable"))?;
    if row.cells().len() != table.columns().len() {
        return Err(table_address_error(address, "row and column counts differ"));
    }
    Ok(())
}

fn table_address_error(address: &CardPageSimpleTableCellAddress, detail: &str) -> String {
    format!(
        "Notion table {} {detail} for row {} column {}",
        address.table_block_id(),
        address.row_block_id(),
        address.column_id().as_str()
    )
}
