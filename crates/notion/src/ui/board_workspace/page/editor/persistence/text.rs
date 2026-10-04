use super::block_shape::VerifiedPageTextBlockKind;
use super::PageMutationPlan;
use crate::model::{
    PageMutation, ReplacePageBlockTextAndConvertRequest, ReplacePageBlockTextRequest,
};

pub(in crate::ui::board_workspace::page::editor) struct PageBlockTextConversion {
    pub(in crate::ui::board_workspace::page::editor) text: String,
    pub(in crate::ui::board_workspace::page::editor) conversion: VerifiedPageTextBlockKind,
}

impl PageMutationPlan {
    pub(in crate::ui::board_workspace::page::editor) fn persist_page_block_text(
        &mut self,
        page_block_id: &str,
        block_id: &str,
        text: String,
    ) {
        self.enqueue_page_mutation(
            page_block_id,
            PageMutation::ReplaceBlockText(ReplacePageBlockTextRequest {
                block_id: block_id.to_string(),
                text,
            }),
        );
    }

    pub(in crate::ui::board_workspace::page::editor) fn persist_page_block_text_and_convert(
        &mut self,
        page_block_id: &str,
        block_id: &str,
        change: PageBlockTextConversion,
    ) {
        self.enqueue_page_mutation(
            page_block_id,
            PageMutation::ReplaceBlockTextAndConvert(ReplacePageBlockTextAndConvertRequest {
                block_id: block_id.to_string(),
                text: change.text,
                kind: change.conversion.notion_kind(),
            }),
        );
    }
}
