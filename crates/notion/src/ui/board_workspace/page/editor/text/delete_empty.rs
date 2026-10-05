use std::ops::Range;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::rich_text::PageWriteTextProjection;
use super::super::support::page_block_subtree_end;
use super::super::{CardPage, CardPageBlockKind, TextInputSnapshot};
use crate::model::{DeletePageBlockRequest, PageMutation};

#[derive(Clone, Copy)]
pub(super) enum EmptyPageBlockDeleteDirection {
    Backward,
    Forward,
}

pub(super) struct PreparedEmptyPageBlockDeletion {
    block_id: String,
    block_range: Range<usize>,
    focus: Option<EmptyPageBlockDeleteFocus>,
    original_cursor: usize,
}

struct EmptyPageBlockDeleteFocus {
    block_id: String,
    offset: usize,
}

impl PreparedEmptyPageBlockDeletion {
    pub(super) fn new(
        page: &CardPage,
        block_index: usize,
        block_id: &str,
        snapshot: &TextInputSnapshot,
        direction: EmptyPageBlockDeleteDirection,
    ) -> Option<Self> {
        let block = page.blocks.get(block_index)?;
        let editable = block.editable_content()?;
        if block.block_id != block_id
            || block.depth != 0
            || editable.kind != CardPageBlockKind::Text
            || !snapshot.text.is_empty()
        {
            return None;
        }
        assert_eq!(
            editable.text, snapshot.text,
            "empty block deletion snapshot must match the loaded block"
        );
        let subtree_end = page_block_subtree_end(&page.blocks, block_index);
        if subtree_end != block_index + 1 {
            return None;
        }
        let adjacent_index = adjacent_root_block_index(page, block_index, direction)?;
        Some(Self {
            block_id: block_id.to_string(),
            block_range: block_index..subtree_end,
            focus: empty_block_delete_focus(&page.blocks[adjacent_index], direction),
            original_cursor: snapshot.cursor,
        })
    }

    pub(super) fn apply(self, session: &mut PageEditSession<'_>, mut page: CardPage) {
        session.editor.record_page_structural_edit(
            &page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: self.block_id.clone(),
                offset: self.original_cursor,
            }),
        );
        let page_id = page.block_id.clone();
        let retired_ids = page.blocks[self.block_range.clone()]
            .iter()
            .map(|block| block.block_id.clone())
            .collect::<Vec<_>>();
        page.blocks.drain(self.block_range);
        session.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutationWithProjection {
                page_id,
                mutation: PageMutation::DeleteBlock(DeletePageBlockRequest {
                    block_id: self.block_id,
                }),
                projection: PageWriteTextProjection::default()
                    .with_retired_blocks(retired_ids.clone()),
            },
        ));
        session.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearRemovedBlockEditorState {
                removed_ids: retired_ids,
            },
        ));
        session
            .effects
            .push(PageEditEffect::ReplaceLoadedPage(page));
        if let Some(focus) = self.focus {
            session.effects.push(PageEditEffect::FocusBlock {
                block_id: focus.block_id,
                offset: focus.offset,
            });
        }
        session.effects.push(PageEditEffect::Notify);
    }
}

fn adjacent_root_block_index(
    page: &CardPage,
    block_index: usize,
    direction: EmptyPageBlockDeleteDirection,
) -> Option<usize> {
    let parent_id = &page.blocks[block_index].parent_block_id;
    match direction {
        EmptyPageBlockDeleteDirection::Backward => (0..block_index)
            .rev()
            .find(|index| page.blocks[*index].parent_block_id == *parent_id),
        EmptyPageBlockDeleteDirection::Forward => (block_index + 1..page.blocks.len())
            .find(|index| page.blocks[*index].parent_block_id == *parent_id),
    }
}

fn empty_block_delete_focus(
    adjacent: &crate::ui::CardPageBlock,
    direction: EmptyPageBlockDeleteDirection,
) -> Option<EmptyPageBlockDeleteFocus> {
    let editable = adjacent.editable_content()?;
    let offset = match direction {
        EmptyPageBlockDeleteDirection::Backward => editable.text.len(),
        EmptyPageBlockDeleteDirection::Forward => 0,
    };
    Some(EmptyPageBlockDeleteFocus {
        block_id: adjacent.block_id.clone(),
        offset,
    })
}
