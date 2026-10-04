use gpui::App;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::support::page_block_subtree_end;
use super::super::{generated_notion_record_id, CardPage, CardPageBlock, CardPageBlockKind};
use crate::model::{CreatePageBlockRequest, PageBlockPlacement, PageMutation};
use crate::ui::PageEditFocus;

struct PreparedRelativePageBlockInsertion {
    page: CardPage,
    page_id: String,
    new_block_id: String,
    request: CreatePageBlockRequest,
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn insert_first_page_toggle_child(
        &mut self,
        block_id: &str,
        cx: &App,
    ) -> bool {
        let Some(page) = self.documents.page_containing_block(block_id) else {
            return false;
        };
        let Some(index) = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)
        else {
            return false;
        };
        if !page.blocks[index]
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
            || !self
                .editor
                .page_toggle_disclosure
                .disclosure(&page.block_id, block_id)
                .is_expanded()
        {
            return false;
        }
        let source = page.blocks[index].clone();
        let focus = self.editor.current_page_edit_focus(cx);
        self.insert_first_page_toggle_child_in_page(page, index, source, focus);
        true
    }

    pub(in crate::ui::board_workspace::page::editor) fn insert_first_page_toggle_child_in_page(
        &mut self,
        mut page: CardPage,
        index: usize,
        source: CardPageBlock,
        focus: Option<PageEditFocus>,
    ) {
        debug_assert!(source
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList));
        let block_id = source.block_id.as_str();
        debug_assert!(self
            .editor
            .page_toggle_disclosure
            .disclosure(&page.block_id, block_id)
            .is_expanded());
        let first_child_id = page.blocks.get(index + 1).and_then(|candidate| {
            (candidate.parent_block_id == block_id).then(|| candidate.block_id.clone())
        });
        let placement = first_child_id
            .map(PageBlockPlacement::Before)
            .unwrap_or(PageBlockPlacement::Append);
        let new_block_id = generated_notion_record_id();
        let new_block = CardPageBlock::editable(
            new_block_id.clone(),
            block_id,
            source.depth + 1,
            CardPageBlockKind::Text,
            String::new(),
        );
        self.editor.record_page_structural_edit(&page, focus);
        page.blocks.insert(index + 1, new_block);
        let page_id = page.block_id.clone();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::CreateBlock(CreatePageBlockRequest {
                    block_id: new_block_id.clone(),
                    parent_block_id: block_id.to_string(),
                    kind: crate::model::NotionPageBlockKind::Text,
                    text: String::new(),
                    placement,
                }),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: new_block_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }

    pub(in crate::ui::board_workspace::page::editor) fn insert_page_block_relative(
        &mut self,
        block_id: &str,
        before: bool,
        cx: &App,
    ) {
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let Some(index) = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)
        else {
            return;
        };
        let source = page.blocks[index].clone();
        let source_parent_block_id = source.parent_block_id.clone();
        let page_block_id = page.block_id.clone();
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let new_block_id = generated_notion_record_id();
        let insertion_index = if before {
            index
        } else {
            page_block_subtree_end(&page.blocks, index)
        };
        page.blocks.insert(
            insertion_index,
            CardPageBlock::editable(
                new_block_id.clone(),
                source_parent_block_id.clone(),
                source.depth,
                CardPageBlockKind::Text,
                String::new(),
            ),
        );
        let request = CreatePageBlockRequest {
            block_id: new_block_id.clone(),
            parent_block_id: source_parent_block_id,
            kind: crate::model::NotionPageBlockKind::Text,
            text: String::new(),
            placement: if before {
                PageBlockPlacement::Before(block_id.to_string())
            } else {
                PageBlockPlacement::After(block_id.to_string())
            },
        };
        self.finish_page_block_relative_insertion(PreparedRelativePageBlockInsertion {
            page,
            page_id: page_block_id,
            new_block_id,
            request,
        });
    }

    fn finish_page_block_relative_insertion(
        &mut self,
        insertion: PreparedRelativePageBlockInsertion,
    ) {
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: insertion.page_id,
                mutation: PageMutation::CreateBlock(insertion.request),
            },
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(insertion.page));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenu,
        ));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: insertion.new_block_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}
