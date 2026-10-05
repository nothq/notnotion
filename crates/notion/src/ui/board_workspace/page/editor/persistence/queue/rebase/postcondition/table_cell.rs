use crate::model::{
    CardPage, CardPageSimpleTableCell, CardPageSimpleTableCellAddress,
    CardPageSimpleTableCellIndex, CardPageSimpleTableColumnId, CardPageWritableSimpleTableCell,
    ReplacePageSimpleTableCellRequest,
};
use crate::ui::board_workspace::PageProjectedText;

use super::super::PageWriteReplayError;
use super::FailedPageWriteDisposition;

pub(super) struct SimpleTableCellPostcondition {
    address: CardPageSimpleTableCellAddress,
    identity: SimpleTableCellIdentity,
    before: PageProjectedText,
    after: PageProjectedText,
}

#[derive(Eq, PartialEq)]
struct SimpleTableCellIdentity {
    column_ids: Box<[CardPageSimpleTableColumnId]>,
}

struct ResolvedSimpleTableCell<'a> {
    identity: SimpleTableCellIdentity,
    cell: &'a CardPageSimpleTableCell,
}

impl SimpleTableCellPostcondition {
    pub(super) fn new(
        request: &ReplacePageSimpleTableCellRequest,
        before: &CardPage,
        after: &CardPage,
    ) -> Result<Self, PageWriteReplayError> {
        let address = request.target().clone();
        let before = resolved_cell(before, &address)
            .map_err(|detail| invalid_postcondition("before", detail))?;
        let after = resolved_cell(after, &address)
            .map_err(|detail| invalid_postcondition("after", detail))?;
        if before.identity != after.identity {
            return Err(invalid_postcondition(
                "after",
                "simple-table cell mutation changed its table schema",
            ));
        }
        Ok(Self {
            before: projected_cell(address.clone(), before.cell, "before")?,
            after: projected_cell(address.clone(), after.cell, "after")?,
            address,
            identity: before.identity,
        })
    }

    pub(super) fn classify(&self, authority: &CardPage) -> FailedPageWriteDisposition {
        let Ok(authority) = resolved_cell(authority, &self.address) else {
            return FailedPageWriteDisposition::Conflict;
        };
        if authority.identity != self.identity {
            return FailedPageWriteDisposition::Conflict;
        }
        if self.after.matches_simple_table_cell(authority.cell) {
            FailedPageWriteDisposition::Applied
        } else if self.before.matches_simple_table_cell(authority.cell) {
            FailedPageWriteDisposition::NotApplied
        } else {
            FailedPageWriteDisposition::Conflict
        }
    }
}

fn resolved_cell<'a>(
    page: &'a CardPage,
    address: &CardPageSimpleTableCellAddress,
) -> Result<ResolvedSimpleTableCell<'a>, String> {
    let index = CardPageSimpleTableCellIndex::new(page)?;
    let cell = index.cell(page, address)?;
    let table = page
        .blocks
        .iter()
        .find(|block| block.block_id == address.table_block_id())
        .and_then(|block| block.simple_table_content())
        .ok_or_else(|| "resolved simple-table root became unavailable".to_string())?;
    Ok(ResolvedSimpleTableCell {
        identity: SimpleTableCellIdentity {
            column_ids: table
                .columns()
                .iter()
                .map(|column| column.id().clone())
                .collect(),
        },
        cell,
    })
}

fn projected_cell(
    address: CardPageSimpleTableCellAddress,
    cell: &CardPageSimpleTableCell,
    side: &'static str,
) -> Result<PageProjectedText, PageWriteReplayError> {
    let writable = CardPageWritableSimpleTableCell::try_from(cell.clone())
        .map_err(|detail| invalid_postcondition(side, detail))?;
    PageProjectedText::simple_table_cell(address, &writable)
        .ok_or_else(|| invalid_postcondition(side, "simple-table cell annotations are invalid"))
}

fn invalid_postcondition(side: &'static str, detail: impl Into<String>) -> PageWriteReplayError {
    PageWriteReplayError::invalid_request(
        "replace simple-table cell postcondition",
        format!("{side} page: {}", detail.into()),
    )
}
