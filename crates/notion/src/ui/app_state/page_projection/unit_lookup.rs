use std::collections::HashSet;

use crate::ui::{CardPage, CardPageBlockKind};

use super::{LoadedCardPageData, LoadedCardPageVisibleRow, PageDocumentUnitKey};

pub(super) fn build_empty_toggle_placeholder_mask(
    page: &CardPage,
    visible_rows: &[LoadedCardPageVisibleRow],
    expanded_toggle_ids: Option<&HashSet<String>>,
) -> Vec<bool> {
    visible_rows
        .iter()
        .map(|row| {
            let block = &page.blocks[row.block_index];
            block
                .editable_content()
                .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
                && expanded_toggle_ids.is_some_and(|ids| ids.contains(&block.block_id))
                && page
                    .blocks
                    .get(row.block_index + 1)
                    .is_none_or(|candidate| candidate.parent_block_id != block.block_id)
        })
        .collect()
}

impl LoadedCardPageData {
    pub(crate) fn document_unit_key_for_block(
        &self,
        block_id: &str,
    ) -> Option<PageDocumentUnitKey> {
        let block_index = *self.block_indices.get(block_id)?;
        let visible_row_index = self
            .visible_rows
            .binary_search_by_key(&block_index, |row| row.block_index)
            .ok()?;
        let unit_index = self.document_unit_ranges[visible_row_index]
            .clone()
            .next()?;
        Some(self.document_units[unit_index].key().clone())
    }

    pub(crate) fn document_unit_key_for_table_cell(
        &self,
        address: &crate::model::CardPageSimpleTableCellAddress,
    ) -> Option<PageDocumentUnitKey> {
        let location = self.simple_table_cell_location(address)?;
        Some(
            self.document_units[location.document_unit_index]
                .key()
                .clone(),
        )
    }

    pub(crate) fn block_shows_empty_toggle_placeholder(&self, block_id: &str) -> bool {
        let Some(block_index) = self.editable_block_indices.get(block_id).copied() else {
            return false;
        };
        self.visible_rows
            .binary_search_by_key(&block_index, |row| row.block_index)
            .ok()
            .is_some_and(|row_index| self.empty_toggle_placeholder_mask[row_index])
    }
}
