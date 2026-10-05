use std::collections::HashSet;

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::{CardPeekState, LoadedCardPage, PageDocumentUnitKey};

use super::PageDocuments;

#[derive(Clone, Copy)]
pub(crate) enum PageDocumentLayoutTarget<'a> {
    Block(&'a str),
    SimpleTableCell {
        page_id: &'a str,
        address: &'a CardPageSimpleTableCellAddress,
    },
}

pub(crate) struct PageDocumentLayoutInvalidation {
    pub(crate) page_id: String,
    pub(crate) unit: PageDocumentUnitKey,
}

impl PageDocuments {
    pub(in crate::ui) fn remeasure_page_layout(
        &self,
        target: PageDocumentLayoutTarget<'_>,
    ) -> Vec<PageDocumentLayoutInvalidation> {
        let mut invalidations = HashSet::new();
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            remeasure_loaded_page(page, target, &mut invalidations);
        }
        if let Some(page) = self.standalone.as_ref() {
            remeasure_loaded_page(page, target, &mut invalidations);
        }
        invalidations
            .into_iter()
            .map(|(page_id, unit)| PageDocumentLayoutInvalidation { page_id, unit })
            .collect()
    }
}

fn remeasure_loaded_page(
    page: &LoadedCardPage,
    target: PageDocumentLayoutTarget<'_>,
    invalidations: &mut HashSet<(String, PageDocumentUnitKey)>,
) {
    if !page.data.has_column_structure() {
        page.list_state.remeasure();
        return;
    }
    let unit = match target {
        PageDocumentLayoutTarget::Block(block_id) => {
            page.data.document_unit_key_for_block(block_id)
        }
        PageDocumentLayoutTarget::SimpleTableCell { page_id, address } => (page.data.page.block_id
            == page_id)
            .then(|| page.data.document_unit_key_for_table_cell(address))
            .flatten(),
    };
    if let Some(unit) = unit {
        invalidations.insert((page.data.page.block_id.clone(), unit));
    }
}
