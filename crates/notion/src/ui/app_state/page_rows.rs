use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::{CardPage, CardPageBlockColor, CardPageBlockColorValue};

use super::{
    LoadedCardPageDocumentUnit, LoadedCardPageSimpleTableCellLocation, LoadedCardPageVisibleRow,
};

mod callouts;
mod canonical;
mod document;

use callouts::{build_callout_projection, CalloutProjection};
use canonical::{build_canonical_projection, CanonicalProjection};
use document::{build_document_projection, DocumentProjection};

pub(super) struct LoadedCardPageRowProjection {
    pub(super) numbered_indices: Vec<usize>,
    pub(super) visible_rows: Vec<LoadedCardPageVisibleRow>,
    pub(super) document_units: Vec<LoadedCardPageDocumentUnit>,
    pub(super) document_unit_ranges: Vec<Range<usize>>,
    pub(super) simple_table_cell_locations:
        HashMap<CardPageSimpleTableCellAddress, LoadedCardPageSimpleTableCellLocation>,
    pub(super) callout_layer_row_indices: Vec<usize>,
    pub(super) callout_layer_row_ranges: Vec<Range<usize>>,
    pub(super) callout_render_unit_ranges: Vec<Range<usize>>,
    pub(super) callout_first_text_input_row_indices: Vec<Option<usize>>,
    pub(super) inherited_text_colors: Vec<Option<CardPageBlockColorValue>>,
    pub(super) effective_callout_colors: Vec<Option<CardPageBlockColor>>,
    pub(super) text_input_block_mask: Vec<bool>,
    pub(super) visible_block_mask: Vec<bool>,
    pub(super) flow_visible_block_mask: Vec<bool>,
    pub(super) collapsed_hidden_owner_indices: Vec<Option<usize>>,
    pub(super) next_root_numbered_index: usize,
}

struct LoadedPageRowProjectionBuilder<'a> {
    page: &'a CardPage,
    expanded_toggle_ids: Option<&'a HashSet<String>>,
}

impl<'a> LoadedPageRowProjectionBuilder<'a> {
    fn new(page: &'a CardPage, expanded_toggle_ids: Option<&'a HashSet<String>>) -> Self {
        Self {
            page,
            expanded_toggle_ids,
        }
    }

    fn build(self) -> LoadedCardPageRowProjection {
        let canonical = self.build_canonical_visible_rows();
        let document = self.build_document_input_masks(&canonical);
        let callouts = self.build_callout_ranges_and_metadata(&canonical, &document);
        self.finish_projection(canonical, document, callouts)
    }

    fn build_canonical_visible_rows(&self) -> CanonicalProjection {
        build_canonical_projection(self.page, self.expanded_toggle_ids)
    }

    fn build_document_input_masks(&self, canonical: &CanonicalProjection) -> DocumentProjection {
        build_document_projection(self.page, canonical)
    }

    fn build_callout_ranges_and_metadata(
        &self,
        canonical: &CanonicalProjection,
        document: &DocumentProjection,
    ) -> CalloutProjection {
        build_callout_projection(self.page, canonical, document)
    }

    fn finish_projection(
        self,
        canonical: CanonicalProjection,
        document: DocumentProjection,
        callouts: CalloutProjection,
    ) -> LoadedCardPageRowProjection {
        LoadedCardPageRowProjection {
            numbered_indices: canonical.numbered_indices,
            visible_rows: canonical.visible_rows,
            document_units: document.document_units,
            document_unit_ranges: document.document_unit_ranges,
            simple_table_cell_locations: document.simple_table_cell_locations,
            callout_layer_row_indices: callouts.layer_row_indices,
            callout_layer_row_ranges: callouts.layer_row_ranges,
            callout_render_unit_ranges: callouts.render_unit_ranges,
            callout_first_text_input_row_indices: callouts.first_text_input_row_indices,
            inherited_text_colors: callouts.inherited_text_colors,
            effective_callout_colors: callouts.effective_colors,
            text_input_block_mask: document.text_input_block_mask,
            visible_block_mask: canonical.visible_block_mask,
            flow_visible_block_mask: canonical.flow_visible_block_mask,
            collapsed_hidden_owner_indices: canonical.collapsed_hidden_owner_indices,
            next_root_numbered_index: canonical.next_root_numbered_index,
        }
    }
}

pub(super) fn build_loaded_card_page_row_projection(
    page: &CardPage,
    expanded_toggle_ids: Option<&HashSet<String>>,
) -> LoadedCardPageRowProjection {
    LoadedPageRowProjectionBuilder::new(page, expanded_toggle_ids).build()
}
