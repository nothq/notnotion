use gpui_components::text_input::TextInputSnapshot;

use super::super::super::rich_text::PageWriteTextProjection;
use super::super::super::{generated_notion_record_id, support::page_block_subtree_end};
use super::super::{validate_endpoint, CrossBlockPageTextSelection};
use crate::model::{PageMutation, PageTextSelectionEndpoint, PastePageTextSelectionRequest};
use crate::ui::surface::PageEditorState;
use crate::ui::{CardPage, CardPageBlock, CardPageBlockKind, PageTextSelection};

use super::super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};

pub(in crate::ui::board_workspace::page::editor) struct PreparedMultilinePageTextPaste {
    original_page: CardPage,
    target_page: CardPage,
    original_focus_block_id: String,
    original_focus_offset: usize,
    original_selection: Option<PageTextSelection>,
    inserted_block_id: String,
    mutation: PageMutation,
}

struct MultilinePageTextPasteSelection {
    first_index: usize,
    first_offset: usize,
    last_index: usize,
    last_offset: usize,
    focus_block_id: String,
    focus_offset: usize,
    original_selection: Option<PageTextSelection>,
}

impl CrossBlockPageTextSelection {
    pub(in super::super) fn prepare_multiline_paste(
        self,
        text: String,
    ) -> Result<PreparedMultilinePageTextPaste, String> {
        let focus_block_id = self.original_selection.focus_block_id.clone();
        let focus_offset = self.original_selection.focus_offset;
        prepare_multiline_page_text_paste_at(
            self.page,
            MultilinePageTextPasteSelection {
                first_index: self.first_index,
                first_offset: self.first_offset,
                last_index: self.last_index,
                last_offset: self.last_offset,
                focus_block_id,
                focus_offset,
                original_selection: Some(self.original_selection),
            },
            text,
        )
    }
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn prepare_multiline_page_text_paste(
        &self,
        active_block_id: &str,
        snapshot: &TextInputSnapshot,
        text: String,
    ) -> Result<PreparedMultilinePageTextPaste, String> {
        if let Some(selection) = CrossBlockPageTextSelection::parse(
            self.editor,
            self.documents,
            active_block_id,
            snapshot,
        )? {
            return selection.prepare_multiline_paste(text);
        }
        let page = self
            .documents
            .page_containing_block(active_block_id)
            .ok_or_else(|| "The active page text is no longer available.".to_string())?;
        let block_index = page
            .blocks
            .iter()
            .position(|block| block.block_id == active_block_id)
            .ok_or_else(|| "The active page text block is missing.".to_string())?;
        let editable = page.blocks[block_index]
            .editable_content()
            .ok_or_else(|| "The active page block is not editable text.".to_string())?;
        if editable.text != snapshot.text {
            return Err(
                "The active page text changed before the paste could be applied.".to_string(),
            );
        }
        validate_endpoint(&editable.text, snapshot.selection.start, "start")?;
        validate_endpoint(&editable.text, snapshot.selection.end, "end")?;
        prepare_multiline_page_text_paste_at(
            page,
            MultilinePageTextPasteSelection {
                first_index: block_index,
                first_offset: snapshot.selection.start,
                last_index: block_index,
                last_offset: snapshot.selection.end,
                focus_block_id: active_block_id.to_string(),
                focus_offset: snapshot.cursor,
                original_selection: selection_from_snapshot(active_block_id, snapshot),
            },
            text,
        )
    }
}

fn prepare_multiline_page_text_paste_at(
    original_page: CardPage,
    selection: MultilinePageTextPasteSelection,
    text: String,
) -> Result<PreparedMultilinePageTextPaste, String> {
    // Notion preserves the current text/selection and inserts one multiline Text sibling after
    // the terminal block, even when both endpoints are in the same block.
    let first_block_id = original_page.blocks[selection.first_index].block_id.clone();
    let last = &original_page.blocks[selection.last_index];
    let last_block_id = last.block_id.clone();
    let new_block_id = generated_notion_record_id();
    let insertion_index = page_block_subtree_end(&original_page.blocks, selection.last_index);
    let mut target_page = original_page.clone();
    target_page.blocks.insert(
        insertion_index,
        CardPageBlock::editable(
            new_block_id.clone(),
            last.parent_block_id.clone(),
            last.depth,
            CardPageBlockKind::Text,
            text.clone(),
        ),
    );
    let mutation = PageMutation::PasteTextSelection(PastePageTextSelectionRequest::new(
        PageTextSelectionEndpoint {
            block_id: first_block_id,
            offset_utf8: selection.first_offset,
        },
        PageTextSelectionEndpoint {
            block_id: last_block_id,
            offset_utf8: selection.last_offset,
        },
        new_block_id.clone(),
        text.clone(),
    )?);
    Ok(PreparedMultilinePageTextPaste {
        original_page,
        target_page,
        original_focus_block_id: selection.focus_block_id,
        original_focus_offset: selection.focus_offset,
        original_selection: selection.original_selection,
        inserted_block_id: new_block_id,
        mutation,
    })
}

impl PreparedMultilinePageTextPaste {
    pub(in crate::ui::board_workspace::page::editor) fn apply(
        self,
        session: &mut PageEditSession<'_>,
    ) {
        let projection = PageWriteTextProjection::from_page_targets(
            &self.target_page,
            std::slice::from_ref(&self.inserted_block_id),
        );
        session.editor.record_page_structural_edit(
            &self.original_page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: self.original_focus_block_id.clone(),
                offset: self.original_focus_offset,
            }),
        );
        session.editor.page_text_selection = self.original_selection;
        session
            .effects
            .push(PageEditEffect::ReplaceLoadedPage(self.target_page));
        session.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutationWithProjection {
                page_id: self.original_page.block_id,
                mutation: self.mutation,
                projection,
            },
        ));
        session.effects.push(PageEditEffect::Editor(
            PageEditorEffect::FinishMultilinePaste,
        ));
        session.effects.push(PageEditEffect::FocusBlock {
            block_id: self.original_focus_block_id,
            offset: self.original_focus_offset,
        });
        session.effects.push(PageEditEffect::Notify);
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn finish_multiline_page_text_paste(
        &mut self,
    ) {
        self.page_block_selection.block_ids.clear();
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_block_context_menu = None;
    }
}

fn selection_from_snapshot(
    block_id: &str,
    snapshot: &TextInputSnapshot,
) -> Option<PageTextSelection> {
    if snapshot.selection.is_empty() {
        return None;
    }
    let anchor_offset = if snapshot.cursor == snapshot.selection.start {
        snapshot.selection.end
    } else {
        snapshot.selection.start
    };
    Some(PageTextSelection {
        anchor_block_id: block_id.to_string(),
        anchor_offset,
        focus_block_id: block_id.to_string(),
        focus_offset: snapshot.cursor,
        pointer_active: false,
    })
}
