use std::collections::HashMap;
use std::sync::Arc;

use crate::ui::{CardPage, CardPageLayoutBlock};

pub(super) struct PageBlockIndices {
    pub(super) all: HashMap<String, usize>,
    pub(super) editable: HashMap<String, usize>,
}

#[derive(Clone)]
pub(crate) struct PageDocumentListAllocation(Arc<()>);

impl PageDocumentListAllocation {
    pub(super) fn fresh() -> Self {
        Self(Arc::new(()))
    }

    pub(crate) fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub(super) fn build_page_block_indices(page: &CardPage) -> PageBlockIndices {
    let mut all = HashMap::with_capacity(page.blocks.len());
    let mut editable = HashMap::new();
    for (index, block) in page.blocks.iter().enumerate() {
        all.insert(block.block_id.clone(), index);
        if block.is_editable() {
            editable.insert(block.block_id.clone(), index);
        }
    }
    PageBlockIndices { all, editable }
}

pub(super) fn card_page_has_column_structure(page: &CardPage) -> bool {
    page.blocks.iter().any(|block| {
        matches!(
            block.layout_content(),
            Some(CardPageLayoutBlock::ColumnList)
        )
    })
}

pub(super) fn card_pages_match(left: &CardPage, right: &CardPage) -> bool {
    left.block_id == right.block_id
        && left.title == right.title
        && left.status == right.status
        && left.properties.len() == right.properties.len()
        && left
            .properties
            .iter()
            .zip(&right.properties)
            .all(|(left, right)| {
                left.label == right.label
                    && left.property_id == right.property_id
                    && left.property_type == right.property_type
                    && left.value == right.value
                    && left.status_options == right.status_options
            })
        && left.blocks == right.blocks
}
