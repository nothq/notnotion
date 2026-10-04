use gpui_components::text_input::ceil_grapheme_boundary;
use std::{collections::HashSet, ops::Range};

use super::CrossBlockPageTextSelection;
use crate::model::{PageMutation, PageTextSelectionAction, PageTextSelectionEndpoint};
use crate::ui::board_workspace::page::editor::rich_text::annotations::{
    annotated_text_slice, apply_annotated_text, concatenate_annotated_text, AnnotatedText,
};
use crate::ui::board_workspace::page::editor::rich_text::PageWriteTextProjection;
use crate::ui::surface::PageEditorState;
use crate::ui::CardPage;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};

mod hierarchy;
mod multiline;
mod mutation;

pub(super) use hierarchy::{ReparentedRoot, SelectionRemovalPlan};
pub(in crate::ui::board_workspace::page::editor) use multiline::PreparedMultilinePageTextPaste;
use mutation::page_text_replacement_mutation;
pub(super) use mutation::page_text_selection_structural_mutations;

pub(in crate::ui::board_workspace::page::editor) struct PreparedPageTextReplacement {
    original_page: CardPage,
    target_page: CardPage,
    survivor_id: String,
    original_focus_offset: usize,
    caret: usize,
    removed_ids: Vec<String>,
    mutation: PageMutation,
}

pub(crate) struct PagePendingCrossBlockComposition {
    page_id: String,
    survivor_id: String,
    removed_ids: Vec<String>,
    survivor_prefix: String,
    survivor_suffix: String,
    mutation: PageMutation,
}

impl PagePendingCrossBlockComposition {
    pub(crate) fn survivor_block_id(&self) -> &str {
        &self.survivor_id
    }
}

pub(in crate::ui::board_workspace::page::editor) struct PreparedCrossBlockPageComposition {
    replacement: PreparedPageTextReplacement,
    marked_range: Range<usize>,
    survivor_prefix: String,
    survivor_suffix: String,
}

struct PageTextReplacementMutationInput<'a> {
    page: &'a CardPage,
    start: PageTextSelectionEndpoint,
    end: PageTextSelectionEndpoint,
    selected_block_ids: Vec<String>,
    replacement: String,
    action: PageTextSelectionAction,
    reparented: &'a [ReparentedRoot],
    removed: &'a HashSet<String>,
}

struct PageTextReplacementSpan {
    first_index: usize,
    first_offset: usize,
    last_index: usize,
    last_offset: usize,
}

impl CrossBlockPageTextSelection {
    fn selected_block_ids(&self) -> Vec<String> {
        self.selected_indices
            .iter()
            .map(|index| self.page.blocks[*index].block_id.clone())
            .collect()
    }

    pub(super) fn prepare_replacement(
        self,
        replacement: String,
        action: PageTextSelectionAction,
    ) -> Result<PreparedPageTextReplacement, String> {
        let selected_block_ids = self.selected_block_ids();
        let original_page = self.page;
        let content = build_replacement_content(
            &original_page,
            PageTextReplacementSpan {
                first_index: self.first_index,
                first_offset: self.first_offset,
                last_index: self.last_index,
                last_offset: self.last_offset,
            },
            &replacement,
        )?;
        let removal = SelectionRemovalPlan::new(
            &original_page,
            &self.selected_indices,
            &self.collapsed_hidden_owner_indices,
        )?;

        let mut target_page = original_page.clone();
        apply_annotated_text(
            target_page.blocks[self.first_index]
                .editable_content_mut()
                .expect("validated replacement survivor must be editable"),
            content.survivor,
        );
        removal.apply_to(&mut target_page)?;

        let mutation = page_text_replacement_mutation(PageTextReplacementMutationInput {
            page: &original_page,
            start: PageTextSelectionEndpoint {
                block_id: content.survivor_id.clone(),
                offset_utf8: self.first_offset,
            },
            end: PageTextSelectionEndpoint {
                block_id: content.terminal_id,
                offset_utf8: self.last_offset,
            },
            selected_block_ids,
            replacement,
            action,
            reparented: &removal.reparented,
            removed: &removal.removed,
        })?;
        Ok(PreparedPageTextReplacement {
            original_page,
            target_page,
            survivor_id: content.survivor_id,
            original_focus_offset: self.first_offset,
            caret: content.caret,
            removed_ids: removal.removed_ids,
            mutation,
        })
    }

    pub(super) fn prepare_composition(
        self,
        text: String,
    ) -> Result<PreparedCrossBlockPageComposition, String> {
        assert!(!text.is_empty(), "a marked composition must contain text");
        let marked_start = self.first_offset;
        let marked_end = marked_start + text.len();
        let first_text = &self.page.blocks[self.first_index]
            .editable_content()
            .expect("validated composition survivor must be editable")
            .text;
        let last_text = &self.page.blocks[self.last_index]
            .editable_content()
            .expect("validated composition terminal must be editable")
            .text;
        let survivor_prefix = first_text[..self.first_offset].to_string();
        let survivor_suffix = last_text[self.last_offset..].to_string();
        Ok(PreparedCrossBlockPageComposition {
            replacement: self.prepare_replacement(text, PageTextSelectionAction::TextMutation)?,
            marked_range: marked_start..marked_end,
            survivor_prefix,
            survivor_suffix,
        })
    }
}

struct ReplacementContent {
    survivor_id: String,
    terminal_id: String,
    survivor: AnnotatedText,
    caret: usize,
}

fn build_replacement_content(
    page: &CardPage,
    span: PageTextReplacementSpan,
    replacement: &str,
) -> Result<ReplacementContent, String> {
    let first = &page.blocks[span.first_index];
    let last = &page.blocks[span.last_index];
    let first_editable = first
        .editable_content()
        .ok_or_else(|| "The first selected page block is not editable.".to_string())?;
    let last_editable = last
        .editable_content()
        .ok_or_else(|| "The last selected page block is not editable.".to_string())?;
    let prefix = annotated_text_slice(first_editable, 0..span.first_offset);
    let raw_caret = prefix.text.len() + replacement.len();
    let suffix = annotated_text_slice(last_editable, span.last_offset..last_editable.text.len());
    let survivor = concatenate_annotated_text([prefix, AnnotatedText::plain(replacement), suffix]);
    let caret = ceil_grapheme_boundary(&survivor.text, raw_caret);
    Ok(ReplacementContent {
        survivor_id: first.block_id.clone(),
        terminal_id: last.block_id.clone(),
        survivor,
        caret,
    })
}

impl PreparedPageTextReplacement {
    pub(in crate::ui::board_workspace::page::editor) fn apply(
        self,
        session: &mut PageEditSession<'_>,
    ) {
        let projection = PageWriteTextProjection::from_page_targets(
            &self.target_page,
            std::slice::from_ref(&self.survivor_id),
        )
        .with_retired_blocks(self.removed_ids.clone());
        session.editor.record_page_structural_edit(
            &self.original_page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: self.survivor_id.clone(),
                offset: self.original_focus_offset,
            }),
        );
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
            PageEditorEffect::FinishCrossBlockReplacement {
                removed_ids: self.removed_ids,
            },
        ));
        session.effects.push(PageEditEffect::FocusBlock {
            block_id: self.survivor_id,
            offset: self.caret,
        });
        session.effects.push(PageEditEffect::Notify);
    }
}

impl PreparedCrossBlockPageComposition {
    pub(in crate::ui::board_workspace::page::editor) fn apply(
        self,
        session: &mut PageEditSession<'_>,
    ) {
        let replacement = self.replacement;
        session.editor.record_page_structural_edit(
            &replacement.original_page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: replacement.survivor_id.clone(),
                offset: replacement.original_focus_offset,
            }),
        );
        session
            .effects
            .push(PageEditEffect::ReplaceLoadedPage(replacement.target_page));
        let removed_ids = replacement.removed_ids;
        let survivor_id = replacement.survivor_id;
        let pending = PagePendingCrossBlockComposition {
            page_id: replacement.original_page.block_id,
            survivor_id: survivor_id.clone(),
            removed_ids: removed_ids.clone(),
            survivor_prefix: self.survivor_prefix,
            survivor_suffix: self.survivor_suffix,
            mutation: replacement.mutation,
        };
        session.effects.push(PageEditEffect::Editor(
            PageEditorEffect::BeginCrossBlockComposition {
                removed_ids,
                pending: Box::new(pending),
                survivor_id,
                marked_range: self.marked_range,
            },
        ));
        session.effects.push(PageEditEffect::Notify);
    }
}

impl PagePendingCrossBlockComposition {
    pub(in crate::ui::board_workspace::page::editor) fn matches(&self, block_id: &str) -> bool {
        self.survivor_id == block_id
    }

    pub(in crate::ui::board_workspace::page::editor) fn into_persistence(
        mut self,
        survivor_text: String,
    ) -> (String, String, Vec<String>, PageMutation) {
        let replacement = survivor_text
            .strip_prefix(&self.survivor_prefix)
            .and_then(|text| text.strip_suffix(&self.survivor_suffix))
            .expect("cross-block composition must preserve survivor boundaries")
            .to_string();
        let PageMutation::ReplaceTextSelection(request) = &mut self.mutation else {
            unreachable!("cross-block composition must retain a replacement mutation")
        };
        request.set_replacement(replacement);
        (
            self.page_id,
            self.survivor_id,
            self.removed_ids,
            self.mutation,
        )
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn finish_cross_block_page_text_replacement(
        &mut self,
        removed_ids: &[String],
    ) {
        self.clear_removed_cross_block_page_text(removed_ids);
        self.page_text_selection = None;
        self.page_block_selection.block_ids.clear();
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_block_context_menu = None;
    }

    pub(in crate::ui::board_workspace::page::editor) fn begin_cross_block_page_composition(
        &mut self,
        removed_ids: &[String],
        pending: PagePendingCrossBlockComposition,
        survivor_id: String,
        marked_range: Range<usize>,
    ) {
        self.finish_cross_block_page_text_replacement(removed_ids);
        self.page_block_compositions.insert(survivor_id.clone());
        self.page_pending_cross_block_composition = Some(pending);
        *self.input.resource_state().focus_request.borrow_mut() = Some(
            crate::ui::board_workspace::PageBlockFocusRequest::marked(survivor_id, marked_range),
        );
    }

    fn clear_removed_cross_block_page_text(&mut self, removed_ids: &[String]) {
        let removed = removed_ids.iter().collect::<HashSet<_>>();
        {
            let mut inputs = self.input.resource_state().block_inputs.borrow_mut();
            for block_id in removed_ids {
                inputs.remove(block_id);
            }
        }
        self.page_block_compositions
            .retain(|block_id| !removed.contains(block_id));
        if self
            .hovered_page_block
            .as_ref()
            .is_some_and(|block_id| removed.contains(block_id))
        {
            self.hovered_page_block = None;
        }
    }
}
