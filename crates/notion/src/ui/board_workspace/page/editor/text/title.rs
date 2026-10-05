use super::super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::super::{
    generated_notion_record_id, CardPageBlock, CardPageBlockKind, TextInputSnapshot,
};
use crate::model::{CreatePageBlockRequest, PageBlockPlacement, PageMutation};
use crate::ui::board_workspace::page::editor::PageMutationPlan;

impl PageEditSession<'_> {
    pub(crate) fn apply_page_title_change(&mut self, page_id: &str, snapshot: TextInputSnapshot) {
        let Some(mut page) = self.documents.page_with_id(page_id) else {
            return;
        };
        let was_composing = self.editor.page_block_compositions.contains(page_id);
        if snapshot.is_composing {
            self.editor
                .page_block_compositions
                .insert(page_id.to_string());
        } else {
            self.editor.page_block_compositions.remove(page_id);
        }
        let composition_committed = was_composing && !snapshot.is_composing;
        let title_changed = page.title != snapshot.text;
        if title_changed {
            page.title = snapshot.text.clone();
            self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        }
        if title_changed && !snapshot.is_composing || composition_committed {
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::text(
                    page_id,
                    page_id,
                    snapshot.text,
                )),
            ));
        }
        if title_changed || composition_committed {
            self.effects.push(PageEditEffect::Notify);
        }
    }

    pub(crate) fn focus_first_page_block(&mut self, page_id: &str) {
        let Some(mut page) = self.documents.page_with_id(page_id) else {
            return;
        };
        if let Some(first_block) = page.blocks.iter().find(|block| block.is_editable()) {
            self.effects.push(PageEditEffect::FocusBlock {
                block_id: first_block.block_id.clone(),
                offset: 0,
            });
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let block_id = generated_notion_record_id();
        let page_block_id = page.block_id.clone();
        self.editor.record_page_structural_edit(&page, None);
        page.blocks.push(CardPageBlock::editable(
            block_id.clone(),
            page.block_id.clone(),
            0,
            CardPageBlockKind::Text,
            String::new(),
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page_block_id.clone(),
                mutation: PageMutation::CreateBlock(CreatePageBlockRequest {
                    block_id: block_id.clone(),
                    parent_block_id: page_block_id.clone(),
                    kind: crate::model::NotionPageBlockKind::Text,
                    text: String::new(),
                    placement: PageBlockPlacement::Append,
                }),
            },
        ));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}
