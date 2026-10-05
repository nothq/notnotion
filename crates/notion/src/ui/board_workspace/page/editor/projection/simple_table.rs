use crate::model::{CardPageSimpleTableCellIndex, CardPageWritableSimpleTableCell};
use crate::ui::surface::PageEditorState;
use crate::ui::{CardPage, LoadedCardPageData, LoadedCardPageSimpleTableCellAccess};

impl PageEditorState {
    pub(super) fn prune_projection_simple_table_cell(
        &self,
        data: &LoadedCardPageData,
        authority: Option<&CardPage>,
    ) {
        let retained = {
            let active = self.tables.editor().borrow();
            active.as_ref().is_none_or(|active| {
                active.page_id == data.page.block_id
                    && (active.composition_active
                        || active.composition_dirty
                        || writable_projection_cell(data, authority, &active.address))
            })
        };
        if !retained {
            self.tables.editor().clear();
        }
    }
}

fn writable_projection_cell(
    data: &LoadedCardPageData,
    authority: Option<&CardPage>,
    address: &crate::model::CardPageSimpleTableCellAddress,
) -> bool {
    data.projected_simple_table_cell(address)
        .is_some_and(|cell| cell.access == LoadedCardPageSimpleTableCellAccess::Writable)
        && writable_model_cell(&data.page, address)
        && authority.is_some_and(|authority| writable_model_cell(authority, address))
}

fn writable_model_cell(
    page: &crate::model::CardPage,
    address: &crate::model::CardPageSimpleTableCellAddress,
) -> bool {
    CardPageSimpleTableCellIndex::new(page)
        .and_then(|index| index.cell(page, address).cloned())
        .and_then(CardPageWritableSimpleTableCell::try_from)
        .is_ok()
}
