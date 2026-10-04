use super::PageMutationPlan;

use crate::model::{PageMutation, SetPageBlockColorRequest, SetPageQuoteSizeRequest};
use crate::ui::{
    page_block_kind_supports_color, CardPageBlock, CardPageBlockColor, CardPageQuoteSize,
};

#[derive(Default)]
pub(super) struct PageBlockPropertyGroups {
    colors: Vec<PageBlockColorGroup>,
    quote_sizes: Vec<PageQuoteSizeGroup>,
}

struct PageBlockColorGroup {
    color: CardPageBlockColor,
    block_ids: Vec<String>,
}

struct PageQuoteSizeGroup {
    size: CardPageQuoteSize,
    block_ids: Vec<String>,
}

impl PageBlockPropertyGroups {
    pub(super) fn add_color(&mut self, color: CardPageBlockColor, block_id: String) {
        if let Some(group) = self.colors.iter_mut().find(|group| group.color == color) {
            group.block_ids.push(block_id);
            return;
        }
        self.colors.push(PageBlockColorGroup {
            color,
            block_ids: vec![block_id],
        });
    }

    pub(super) fn add_quote_size(&mut self, size: CardPageQuoteSize, block_id: String) {
        if let Some(group) = self.quote_sizes.iter_mut().find(|group| group.size == size) {
            group.block_ids.push(block_id);
            return;
        }
        self.quote_sizes.push(PageQuoteSizeGroup {
            size,
            block_ids: vec![block_id],
        });
    }

    pub(super) fn persist(self, plan: &mut PageMutationPlan, page_id: &str) {
        for group in self.colors {
            let request = SetPageBlockColorRequest::new(group.block_ids, group.color)
                .expect("persisted color target groups must be non-empty and unique");
            plan.enqueue_page_mutation(page_id, PageMutation::SetBlockColor(request));
        }
        for group in self.quote_sizes {
            let request = SetPageQuoteSizeRequest::new(group.block_ids, group.size)
                .expect("persisted quote-size target groups must be non-empty and unique");
            plan.enqueue_page_mutation(page_id, PageMutation::SetQuoteSize(request));
        }
    }
}

pub(super) fn page_block_persists_color(block: &CardPageBlock) -> bool {
    block.alias_content().is_some()
        || block
            .editable_content()
            .is_some_and(|editable| page_block_kind_supports_color(editable.kind))
}
