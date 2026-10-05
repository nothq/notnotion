use gpui_components::text_input::{ceil_grapheme_boundary, TextInputSnapshot};

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::rich_text::annotations::{
    annotated_text_slice, apply_annotated_text, concatenate_annotated_text, AnnotatedText,
};
use super::super::rich_text::PageWriteTextProjection;
use super::super::{
    generated_notion_record_id, persistence::VerifiedPageTextBlockKind,
    support::page_block_subtree_end,
};
use super::replacement::{page_text_selection_structural_mutations, SelectionRemovalPlan};
use super::CrossBlockPageTextSelection;
use crate::model::{
    BreakPageTextSelectionRequest, PageMutation, PageTextLineBreak, PageTextSelectionEndpoint,
};
use crate::ui::{CardPage, CardPageBlock, CardPageEditableBlock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum CrossBlockPageLineBreak {
    Enter,
    ShiftEnter,
}

pub(in super::super) struct PreparedCrossBlockPageLineBreak {
    original_page: CardPage,
    target_page: CardPage,
    original_focus_block_id: String,
    original_focus_offset: usize,
    removed_ids: Vec<String>,
    caret_block_id: String,
    caret_offset: usize,
    mutation: PageMutation,
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn prepare_cross_block_page_line_break(
        &self,
        active_block_id: &str,
        snapshot: &TextInputSnapshot,
        line_break: CrossBlockPageLineBreak,
    ) -> Result<Option<PreparedCrossBlockPageLineBreak>, String> {
        CrossBlockPageTextSelection::parse(self.editor, self.documents, active_block_id, snapshot)?
            .map(|selection| selection.prepare_line_break(line_break))
            .transpose()
    }
}

impl CrossBlockPageTextSelection {
    fn prepare_line_break(
        self,
        line_break: CrossBlockPageLineBreak,
    ) -> Result<PreparedCrossBlockPageLineBreak, String> {
        let endpoints = LineBreakEndpoints::from_selection(&self)?;
        let first_offset = self.first_offset;
        let last_offset = self.last_offset;
        let selected_block_ids = self
            .selected_indices
            .iter()
            .map(|index| self.page.blocks[*index].block_id.clone())
            .collect();
        let original_page = self.page;
        let removal = SelectionRemovalPlan::new(
            &original_page,
            &self.selected_indices,
            &self.collapsed_hidden_owner_indices,
        )?;
        let structural_mutations = page_text_selection_structural_mutations(
            &original_page,
            &removal.reparented,
            &removal.removed,
        )?;

        let mut target_page = original_page.clone();
        removal.apply_to(&mut target_page)?;
        let survivor_index = line_break_survivor_index(&target_page, &endpoints.survivor_id)?;
        let applied = apply_line_break(&mut target_page, survivor_index, &endpoints, line_break);
        let mutation = PageMutation::BreakTextSelection(BreakPageTextSelectionRequest::new(
            PageTextSelectionEndpoint {
                block_id: endpoints.survivor_id.clone(),
                offset_utf8: first_offset,
            },
            PageTextSelectionEndpoint {
                block_id: endpoints.terminal_id,
                offset_utf8: last_offset,
            },
            selected_block_ids,
            applied.model_line_break,
            structural_mutations,
        )?);

        Ok(PreparedCrossBlockPageLineBreak {
            original_page,
            target_page,
            original_focus_block_id: endpoints.survivor_id,
            original_focus_offset: first_offset,
            removed_ids: removal.removed_ids,
            caret_block_id: applied.caret_block_id,
            caret_offset: applied.caret_offset,
            mutation,
        })
    }
}

struct LineBreakEndpoints {
    survivor_id: String,
    survivor_parent_id: String,
    survivor_depth: usize,
    destination_kind: VerifiedPageTextBlockKind,
    terminal_id: String,
    prefix: AnnotatedText,
    suffix: AnnotatedText,
}

impl LineBreakEndpoints {
    fn from_selection(selection: &CrossBlockPageTextSelection) -> Result<Self, String> {
        let first = &selection.page.blocks[selection.first_index];
        let last = &selection.page.blocks[selection.last_index];
        let first_editable = first
            .editable_content()
            .ok_or_else(|| "The first selected page block is not editable.".to_string())?;
        let last_editable = last
            .editable_content()
            .ok_or_else(|| "The last selected page block is not editable.".to_string())?;
        let first_kind = VerifiedPageTextBlockKind::parse(first_editable.kind)
            .ok_or_else(|| "The first selected page block cannot be split.".to_string())?;
        Ok(Self {
            survivor_id: first.block_id.clone(),
            survivor_parent_id: first.parent_block_id.clone(),
            survivor_depth: first.depth,
            destination_kind: first_kind.split_kind(),
            terminal_id: last.block_id.clone(),
            prefix: annotated_text_slice(first_editable, 0..selection.first_offset),
            suffix: annotated_text_slice(
                last_editable,
                selection.last_offset..last_editable.text.len(),
            ),
        })
    }
}

struct AppliedLineBreak {
    model_line_break: PageTextLineBreak,
    caret_block_id: String,
    caret_offset: usize,
}

fn line_break_survivor_index(page: &CardPage, survivor_id: &str) -> Result<usize, String> {
    page.blocks
        .iter()
        .position(|block| block.block_id == survivor_id)
        .ok_or_else(|| format!("line-break survivor {survivor_id} is missing"))
}

fn apply_line_break(
    page: &mut CardPage,
    survivor_index: usize,
    endpoints: &LineBreakEndpoints,
    line_break: CrossBlockPageLineBreak,
) -> AppliedLineBreak {
    match line_break {
        CrossBlockPageLineBreak::Enter => apply_enter(page, survivor_index, endpoints),
        CrossBlockPageLineBreak::ShiftEnter => apply_shift_enter(page, survivor_index, endpoints),
    }
}

fn apply_enter(
    page: &mut CardPage,
    survivor_index: usize,
    endpoints: &LineBreakEndpoints,
) -> AppliedLineBreak {
    apply_annotated_text(
        page.blocks[survivor_index]
            .editable_content_mut()
            .expect("validated line-break survivor must remain editable"),
        endpoints.prefix.clone(),
    );
    let new_block_id = generated_notion_record_id();
    let insertion_index = page_block_subtree_end(&page.blocks, survivor_index);
    let destination = CardPageEditableBlock::new(
        endpoints.destination_kind.card_kind(),
        endpoints.suffix.text.clone(),
        endpoints.suffix.annotations.clone(),
    );
    page.blocks.insert(
        insertion_index,
        CardPageBlock::from_editable(
            new_block_id.clone(),
            endpoints.survivor_parent_id.clone(),
            endpoints.survivor_depth,
            destination,
        ),
    );
    AppliedLineBreak {
        model_line_break: PageTextLineBreak::Enter {
            new_block_id: new_block_id.clone(),
            new_block_kind: endpoints.destination_kind.notion_kind(),
        },
        caret_block_id: new_block_id,
        caret_offset: 0,
    }
}

fn apply_shift_enter(
    page: &mut CardPage,
    survivor_index: usize,
    endpoints: &LineBreakEndpoints,
) -> AppliedLineBreak {
    let raw_caret = endpoints.prefix.text.len() + 1;
    let survivor = concatenate_annotated_text([
        endpoints.prefix.clone(),
        AnnotatedText::plain("\n"),
        endpoints.suffix.clone(),
    ]);
    let caret_offset = ceil_grapheme_boundary(&survivor.text, raw_caret);
    apply_annotated_text(
        page.blocks[survivor_index]
            .editable_content_mut()
            .expect("validated line-break survivor must remain editable"),
        survivor,
    );
    AppliedLineBreak {
        model_line_break: PageTextLineBreak::ShiftEnter,
        caret_block_id: endpoints.survivor_id.clone(),
        caret_offset,
    }
}

impl PreparedCrossBlockPageLineBreak {
    pub(in super::super) fn apply(self, session: &mut PageEditSession<'_>) {
        let projection_ids = [
            self.original_focus_block_id.clone(),
            self.caret_block_id.clone(),
        ];
        let projection =
            PageWriteTextProjection::from_page_targets(&self.target_page, &projection_ids)
                .with_retired_blocks(self.removed_ids.clone());
        session.editor.record_page_structural_edit(
            &self.original_page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: self.original_focus_block_id,
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
            block_id: self.caret_block_id,
            offset: self.caret_offset,
        });
        session.effects.push(PageEditEffect::Notify);
    }
}
