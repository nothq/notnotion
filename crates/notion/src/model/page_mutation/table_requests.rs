use crate::model::{
    CardPage, CardPageSimpleTableCell, CardPageSimpleTableCellAddress,
    CardPageSimpleTableCellIndex, CardPageWritableSimpleTableCell,
};

#[derive(Clone, Debug)]
pub struct ReplacePageSimpleTableCellRequest {
    target: CardPageSimpleTableCellAddress,
    cell: CardPageWritableSimpleTableCell,
}

impl ReplacePageSimpleTableCellRequest {
    pub fn new(
        page: &CardPage,
        target: CardPageSimpleTableCellAddress,
        cell: CardPageSimpleTableCell,
    ) -> Result<Self, String> {
        let index = CardPageSimpleTableCellIndex::new(page)?;
        CardPageWritableSimpleTableCell::try_from(index.cell(page, &target)?.clone())?;
        Ok(Self {
            target,
            cell: cell.try_into()?,
        })
    }

    pub fn target(&self) -> &CardPageSimpleTableCellAddress {
        &self.target
    }

    pub fn cell(&self) -> &CardPageWritableSimpleTableCell {
        &self.cell
    }
}
