use std::{collections::HashSet, ops::Range};

use gpui::App;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::rich_text::PageWriteTextProjection;
use super::super::support::page_block_subtree_end;
use super::super::{CardPage, CardPageBlock};
use crate::model::{DeletePageBlockRequest, PageMutation};
use crate::ui::surface::{PageDocuments, PageEditorState};

struct PageBlockDeletion {
    page: CardPage,
    page_id: String,
    ranges: Vec<Range<usize>>,
}

impl PageBlockDeletion {
    fn for_target(
        documents: &PageDocuments,
        editor: &PageEditorState,
        block_id: &str,
    ) -> Option<Self> {
        let page = documents.page_containing_block(block_id)?;
        let root_indices = editor.page_block_context_menu_root_indices(&page, block_id);
        if root_indices.is_empty() {
            return None;
        }
        let ranges = root_indices
            .iter()
            .map(|index| *index..page_block_subtree_end(&page.blocks, *index))
            .collect::<Vec<_>>();
        Some(Self {
            page_id: page.block_id.clone(),
            page,
            ranges,
        })
    }

    fn is_supported(&self) -> bool {
        self.ranges.iter().all(|range| {
            self.page.blocks[range.clone()]
                .iter()
                .all(page_block_can_delete)
        })
    }

    fn removed_ids(&self) -> Vec<String> {
        self.ranges
            .iter()
            .flat_map(|range| self.page.blocks[range.clone()].iter())
            .map(|block| block.block_id.clone())
            .collect()
    }
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn delete_page_block(
        &mut self,
        block_id: &str,
        cx: &App,
    ) {
        let Some(mut deletion) =
            PageBlockDeletion::for_target(self.documents, self.editor, block_id)
        else {
            return;
        };
        if !deletion.is_supported() {
            self.effects.push(PageEditEffect::Error(
                "This block type cannot be deleted through the page editor yet.".to_string(),
            ));
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor
            .record_page_structural_edit(&deletion.page, focus);
        let removed_ids = deletion.removed_ids();
        self.apply_page_block_deletion(&mut deletion);
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearRemovedBlockEditorState { removed_ids },
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(deletion.page));
        self.effects.push(PageEditEffect::Notify);
    }

    fn apply_page_block_deletion(&mut self, deletion: &mut PageBlockDeletion) {
        for range in deletion.ranges.iter().rev() {
            let root_block_id = deletion.page.blocks[range.start].block_id.clone();
            let retired_ids = deletion.page.blocks[range.clone()]
                .iter()
                .map(|block| block.block_id.clone())
                .collect::<Vec<_>>();
            deletion.page.blocks.drain(range.clone());
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::EnqueueMutationWithProjection {
                    page_id: deletion.page_id.clone(),
                    mutation: PageMutation::DeleteBlock(DeletePageBlockRequest {
                        block_id: root_block_id,
                    }),
                    projection: PageWriteTextProjection::default().with_retired_blocks(retired_ids),
                },
            ));
        }
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn clear_removed_page_block_editor_state(
        &mut self,
        removed_ids: &[String],
    ) {
        let removed = removed_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        self.clear_removed_page_block_inputs(&removed);
        self.clear_removed_page_block_interactions(&removed);
        self.clear_removed_page_block_rich_text_state(&removed);
    }

    fn clear_removed_page_block_inputs(&mut self, removed: &HashSet<&str>) {
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .retain(|block_id, _| !removed.contains(block_id.as_str()));
        self.input
            .resource_state()
            .block_input_props
            .borrow_mut()
            .retain(|block_id, _| !removed.contains(block_id.as_str()));
        self.input
            .resource_state()
            .code_syntax
            .borrow_mut()
            .retain(|block_id| !removed.contains(block_id));
        self.page_block_compositions
            .retain(|block_id| !removed.contains(block_id.as_str()));
        for block_id in removed {
            self.clear_page_composer_handoff(block_id);
        }
    }

    fn clear_removed_page_block_interactions(&mut self, removed: &HashSet<&str>) {
        if self
            .active_page_block
            .as_deref()
            .is_some_and(|block_id| removed.contains(block_id))
        {
            self.active_page_block = None;
        }
        if self
            .hovered_page_block
            .as_deref()
            .is_some_and(|block_id| removed.contains(block_id))
        {
            self.hovered_page_block = None;
        }
        if self
            .input
            .resource_state()
            .focus_request
            .borrow()
            .as_ref()
            .is_some_and(|focus| removed.contains(focus.block_id.as_str()))
        {
            self.input
                .resource_state()
                .focus_request
                .borrow_mut()
                .take();
        }
        if self
            .page_block_context_menu
            .as_ref()
            .is_some_and(|menu| removed.contains(menu.block_id.as_str()))
        {
            self.page_block_context_menu = None;
        }
        if self
            .page_slash_menu
            .as_ref()
            .is_some_and(|menu| removed.contains(menu.block_id.as_str()))
        {
            self.page_slash_menu = None;
            self.mention.clear_menu();
        }
        self.page_block_selection
            .block_ids
            .retain(|block_id| !removed.contains(block_id.as_str()));
        if self.page_text_selection.as_ref().is_some_and(|selection| {
            removed.contains(selection.anchor_block_id.as_str())
                || removed.contains(selection.focus_block_id.as_str())
        }) {
            self.page_text_selection = None;
        }
    }

    fn clear_removed_page_block_rich_text_state(&mut self, removed: &HashSet<&str>) {
        if self
            .page_forced_text_annotations
            .as_ref()
            .is_some_and(|state| removed.contains(state.block_id.as_str()))
        {
            self.page_forced_text_annotations = None;
        }
        if self
            .page_pending_rich_text_typing
            .as_ref()
            .is_some_and(|state| removed.contains(state.block_id.as_str()))
        {
            self.page_pending_rich_text_typing = None;
        }
        if self
            .page_pending_rich_text_composition
            .as_ref()
            .is_some_and(|state| removed.contains(state.block_id.as_str()))
        {
            self.page_pending_rich_text_composition = None;
        }
    }
}

pub(super) fn page_block_can_delete(block: &CardPageBlock) -> bool {
    block.is_editable()
        || block.alias_content().is_some()
        || block.resource_content().is_some()
        || block.unsupported_leaf_content().is_some()
        || block.simple_table_content().is_some()
        || block.simple_table_row_content().is_some()
        || matches!(
            block.structural_content(),
            Some(crate::ui::CardPageStructuralBlock::Divider)
        )
}
