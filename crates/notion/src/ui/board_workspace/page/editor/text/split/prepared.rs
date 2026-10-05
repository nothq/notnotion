use gpui_components::text_input::TextInputSnapshot;

use super::LoadedPageBlockSplitSource;
use crate::model::{PageMutation, SplitPageBlockRequest};
use crate::ui::board_workspace::page::editor::persistence::VerifiedPageTextBlockKind;
use crate::ui::board_workspace::page::editor::rich_text::annotations::{
    annotated_text_slice, apply_annotated_text,
};
use crate::ui::board_workspace::page::editor::rich_text::PageWriteTextProjection;
use crate::ui::board_workspace::page::editor::support::reparent_direct_children;
use crate::ui::board_workspace::page::editor::{generated_notion_record_id, PageMutationPlan};
use crate::ui::{CardPage, CardPageBlock, CardPageEditableBlock};

pub(super) struct PreparedPageBlockSplit {
    page: CardPage,
    index: usize,
    block: CardPageBlock,
    editable: CardPageEditableBlock,
    kind: VerifiedPageTextBlockKind,
    snapshot: TextInputSnapshot,
}

pub(super) struct AppliedPageBlockSplit {
    pub(super) page: CardPage,
    pub(super) page_id: String,
    pub(super) text_plan: PageMutationPlan,
    pub(super) mutation: PageMutation,
    pub(super) projection: PageWriteTextProjection,
    pub(super) new_block_id: String,
}

impl LoadedPageBlockSplitSource {
    pub(super) fn prepare(
        self,
        kind: VerifiedPageTextBlockKind,
        snapshot: TextInputSnapshot,
    ) -> PreparedPageBlockSplit {
        PreparedPageBlockSplit {
            page: self.page,
            index: self.index,
            block: self.block,
            editable: self.editable,
            kind,
            snapshot,
        }
    }
}

impl PreparedPageBlockSplit {
    pub(super) fn apply(self) -> AppliedPageBlockSplit {
        let Self {
            mut page,
            index,
            block,
            mut editable,
            kind,
            snapshot,
        } = self;
        let block_id = block.block_id.clone();
        let left = annotated_text_slice(&editable, 0..snapshot.selection.start);
        let right = annotated_text_slice(&editable, snapshot.selection.end..editable.text.len());
        let split_offset_utf16 = left.text.encode_utf16().count();
        let text_without_selection = format!("{}{}", left.text, right.text);
        apply_annotated_text(
            page.blocks[index]
                .editable_content_mut()
                .expect("split page block must remain editable"),
            left,
        );
        let new_block_id = generated_notion_record_id();
        reparent_direct_children(&mut page.blocks, index, &new_block_id);
        let new_block_kind = kind.split_kind();
        editable.set_kind(new_block_kind.card_kind());
        apply_annotated_text(&mut editable, right);
        page.blocks.insert(
            index + 1,
            CardPageBlock::from_editable(
                new_block_id.clone(),
                block.parent_block_id,
                block.depth,
                editable,
            ),
        );
        let page_id = page.block_id.clone();
        let text_plan = PageMutationPlan::text(&page_id, &block_id, text_without_selection);
        let projection = PageWriteTextProjection::from_page_blocks([
            &page.blocks[index],
            &page.blocks[index + 1],
        ]);
        let mutation = PageMutation::SplitBlock(SplitPageBlockRequest {
            block_id,
            new_block_id: new_block_id.clone(),
            split_offset_utf16,
            new_block_kind: new_block_kind.notion_kind(),
        });
        AppliedPageBlockSplit {
            page,
            page_id,
            text_plan,
            mutation,
            projection,
            new_block_id,
        }
    }
}
