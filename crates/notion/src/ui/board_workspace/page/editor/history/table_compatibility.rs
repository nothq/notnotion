use std::collections::HashMap;

use crate::model::{
    CardPage, CardPageBlock, CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex,
    CardPageWritableSimpleTableCell,
};
use crate::ui::PageEditScope;

pub(super) fn validate_simple_table_history_compatibility(
    current: &CardPage,
    target: &CardPage,
    scope: &PageEditScope,
) -> Result<(), String> {
    if current.block_id != target.block_id {
        return Err("history snapshot belongs to a different page".to_string());
    }
    SimpleTableHistoryCompatibility::new(current, target, scope)?.validate()
}

struct SimpleTableHistoryPage<'a> {
    page: &'a CardPage,
    blocks: HashMap<&'a str, &'a CardPageBlock>,
    tables: HashMap<&'a str, &'a CardPageBlock>,
    cells: CardPageSimpleTableCellIndex,
}

impl<'a> SimpleTableHistoryPage<'a> {
    fn new(page: &'a CardPage) -> Result<Self, String> {
        Ok(Self {
            page,
            blocks: block_index(page)?,
            tables: table_index(page)?,
            cells: CardPageSimpleTableCellIndex::new(page)?,
        })
    }
}

struct SimpleTableHistoryCompatibility<'page, 'scope> {
    current: SimpleTableHistoryPage<'page>,
    target: SimpleTableHistoryPage<'page>,
    scope: &'scope PageEditScope,
}

impl<'page, 'scope> SimpleTableHistoryCompatibility<'page, 'scope> {
    fn new(
        current: &'page CardPage,
        target: &'page CardPage,
        scope: &'scope PageEditScope,
    ) -> Result<Self, String> {
        Ok(Self {
            current: SimpleTableHistoryPage::new(current)?,
            target: SimpleTableHistoryPage::new(target)?,
            scope,
        })
    }

    fn validate(&self) -> Result<(), String> {
        if self.current.tables.len() != self.target.tables.len() {
            return Err("simple-table roots changed since the history snapshot".to_string());
        }
        for (table_id, current_root) in &self.current.tables {
            let target_root = self
                .target
                .tables
                .get(table_id)
                .ok_or_else(|| format!("simple table {table_id} is absent from history"))?;
            validate_table_root(current_root, target_root)?;
            self.validate_table_rows(current_root)?;
        }
        Ok(())
    }

    fn validate_table_rows(&self, current_root: &CardPageBlock) -> Result<(), String> {
        let table = current_root
            .simple_table_content()
            .expect("indexed table root must retain table content");
        for row_id in table.row_block_ids() {
            let current_row = self
                .current
                .blocks
                .get(row_id.as_str())
                .ok_or_else(|| format!("simple table row {row_id} is unavailable"))?;
            let target_row = self
                .target
                .blocks
                .get(row_id.as_str())
                .ok_or_else(|| format!("history simple table row {row_id} is unavailable"))?;
            validate_row_identity(current_row, target_row)?;
            self.validate_row_cells(current_root, row_id)?;
        }
        Ok(())
    }

    fn validate_row_cells(&self, current_root: &CardPageBlock, row_id: &str) -> Result<(), String> {
        let table = current_root
            .simple_table_content()
            .expect("indexed table root must retain table content");
        for column in table.columns() {
            let address = CardPageSimpleTableCellAddress::new(
                current_root.block_id.clone(),
                row_id.to_string(),
                column.id().clone(),
            )?;
            validate_cell(
                self.current.cells.cell(self.current.page, &address)?,
                self.target.cells.cell(self.target.page, &address)?,
                &address,
                self.scope,
            )?;
        }
        Ok(())
    }
}

fn block_index(page: &CardPage) -> Result<HashMap<&str, &CardPageBlock>, String> {
    let mut blocks = HashMap::with_capacity(page.blocks.len());
    for block in &page.blocks {
        if blocks.insert(block.block_id.as_str(), block).is_some() {
            return Err(format!("page contains duplicate block {}", block.block_id));
        }
    }
    Ok(blocks)
}

fn table_index(page: &CardPage) -> Result<HashMap<&str, &CardPageBlock>, String> {
    let mut tables = HashMap::new();
    for block in page
        .blocks
        .iter()
        .filter(|block| block.simple_table_content().is_some())
    {
        if tables.insert(block.block_id.as_str(), block).is_some() {
            return Err(format!("page contains duplicate table {}", block.block_id));
        }
    }
    Ok(tables)
}

fn validate_table_root(current: &CardPageBlock, target: &CardPageBlock) -> Result<(), String> {
    let identity_matches = current.parent_block_id == target.parent_block_id
        && current.depth == target.depth
        && current.color == target.color
        && current.icon == target.icon
        && current.simple_table_content() == target.simple_table_content();
    if !identity_matches {
        return Err(format!(
            "simple table {} changed schema or identity",
            current.block_id
        ));
    }
    Ok(())
}

fn validate_row_identity(current: &CardPageBlock, target: &CardPageBlock) -> Result<(), String> {
    if current.parent_block_id != target.parent_block_id
        || current.depth != target.depth
        || current.color != target.color
        || current.icon != target.icon
    {
        return Err(format!(
            "simple table row {} changed identity",
            current.block_id
        ));
    }
    Ok(())
}

fn validate_cell(
    current: &crate::model::CardPageSimpleTableCell,
    target: &crate::model::CardPageSimpleTableCell,
    address: &CardPageSimpleTableCellAddress,
    scope: &PageEditScope,
) -> Result<(), String> {
    match scope {
        PageEditScope::WholePage if current == target => return Ok(()),
        PageEditScope::WholePage => {
            return Err(format!(
                "simple table cell {}/{}/{} changed outside whole-page history",
                address.table_block_id(),
                address.row_block_id(),
                address.column_id().as_str()
            ));
        }
        PageEditScope::SimpleTableCell(expected) if expected != address => return Ok(()),
        PageEditScope::SimpleTableCell(_) => {}
        PageEditScope::ColumnRatios(_) => return Ok(()),
    }
    CardPageWritableSimpleTableCell::try_from(current.clone()).map_err(|_| {
        format!(
            "simple table cell {}/{}/{} became read-only",
            address.table_block_id(),
            address.row_block_id(),
            address.column_id().as_str()
        )
    })?;
    CardPageWritableSimpleTableCell::try_from(target.clone()).map_err(|_| {
        format!(
            "history simple table cell {}/{}/{} is read-only and differs",
            address.table_block_id(),
            address.row_block_id(),
            address.column_id().as_str()
        )
    })?;
    Ok(())
}

pub(super) fn rebase_simple_table_cell_history_target(
    current: &CardPage,
    target: &CardPage,
    address: &CardPageSimpleTableCellAddress,
) -> Result<CardPage, String> {
    let target_index = CardPageSimpleTableCellIndex::new(target)?;
    let cell =
        CardPageWritableSimpleTableCell::try_from(target_index.cell(target, address)?.clone())?;
    let current_index = CardPageSimpleTableCellIndex::new(current)?;
    CardPageWritableSimpleTableCell::try_from(current_index.cell(current, address)?.clone())?;
    let mut rebased = current.clone();
    current_index.replace(&mut rebased, address, cell)?;
    Ok(rebased)
}
