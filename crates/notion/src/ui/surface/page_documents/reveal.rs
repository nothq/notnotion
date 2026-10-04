use std::sync::Arc;

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::{CardPeekState, LoadedCardPageData};

use super::PageDocuments;

impl PageDocuments {
    pub(in crate::ui) fn active_page_data(&self) -> Option<Arc<LoadedCardPageData>> {
        match self.selected_page.as_ref() {
            Some(CardPeekState::Loaded(page)) => Some(page.data.clone()),
            Some(CardPeekState::Loading { .. } | CardPeekState::Error { .. }) => None,
            None => self.standalone.as_ref().map(|page| page.data.clone()),
        }
    }

    pub(in crate::ui) fn reveal_page_block_list_item(&self, block_id: &str) {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.has_column_structure()
                && page.data.document_unit_key_for_block(block_id).is_some()
            {
                return;
            }
            if let Some(item_index) = page_document_list_item_index(&page.data, block_id) {
                page.list_state.scroll_to_reveal_item(item_index);
                return;
            }
        }
        if let Some(page) = self.standalone.as_ref() {
            if page.data.has_column_structure()
                && page.data.document_unit_key_for_block(block_id).is_some()
            {
                return;
            }
            if let Some(item_index) = page_document_list_item_index(&page.data, block_id) {
                page.list_state.scroll_to_reveal_item(item_index);
            }
        }
    }

    pub(in crate::ui) fn reveal_page_simple_table_cell(
        &self,
        data: &LoadedCardPageData,
        address: &CardPageSimpleTableCellAddress,
    ) {
        if data.has_column_structure() {
            return;
        }
        let Some(location) = data.simple_table_cell_location(address) else {
            return;
        };
        let item_index = data
            .sections
            .iter()
            .position(|section| section.unit_range.contains(&location.document_unit_index))
            .map(|index| index + 1);
        let Some(item_index) = item_index else {
            return;
        };
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.page.block_id == data.page.block_id {
                page.list_state.scroll_to_reveal_item(item_index);
                return;
            }
        }
        if let Some(page) = self.standalone.as_ref() {
            if page.data.page.block_id == data.page.block_id {
                page.list_state.scroll_to_reveal_item(item_index);
            }
        }
    }

    pub(in crate::ui) fn page_block_text_focus_target(
        &self,
        block_id: &str,
        offset: usize,
    ) -> Option<(String, usize)> {
        let data = self.page_data_containing_editable_block(block_id)?;
        if data.block_has_text_input(block_id) {
            return Some((block_id.to_string(), offset));
        }
        let block_index = *data.editable_block_indices.get(block_id)?;
        let visible_row_index = data
            .visible_rows
            .binary_search_by_key(&block_index, |row| row.block_index)
            .ok()?;
        let child_row_index = data.callout_first_text_input_row_index(visible_row_index)?;
        let child_index = data.visible_rows[child_row_index].block_index;
        Some((data.page.blocks[child_index].block_id.clone(), 0))
    }
}

fn page_document_list_item_index(data: &LoadedCardPageData, block_id: &str) -> Option<usize> {
    let block_index = *data.editable_block_indices.get(block_id)?;
    let visible_row_index = data
        .visible_rows
        .binary_search_by_key(&block_index, |row| row.block_index)
        .ok()?;
    let document_unit_range = &data.document_unit_ranges[visible_row_index];
    let document_unit_index =
        (!document_unit_range.is_empty()).then_some(document_unit_range.start)?;
    data.sections
        .iter()
        .position(|section| section.unit_range.contains(&document_unit_index))
        .map(|section_index| section_index + 1)
}
