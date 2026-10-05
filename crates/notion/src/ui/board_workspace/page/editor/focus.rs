use std::ops::Range;

use gpui::App;

use super::editing::{PageEditEffect, PageEditSession, PageEditTransition};
use super::history::PageHistoryRestoreState;
use super::{CardPageBlockKind, Context, LoadedCardPageData, SurfaceState};
use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::surface::{
    PageDocumentLayoutInvalidation, PageDocumentLayoutTarget, PageDocuments, PageEditorState,
    PageSimpleTableCellFocusMode,
};
use crate::ui::PageEditFocus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageBlockFocusRequest {
    pub(in crate::ui::board_workspace::page::editor) block_id: String,
    pub(in crate::ui::board_workspace::page::editor) cursor: usize,
    pub(in crate::ui::board_workspace::page::editor) marked_range: Option<Range<usize>>,
}

impl PageBlockFocusRequest {
    pub(in crate::ui::board_workspace::page::editor) fn cursor(
        block_id: String,
        cursor: usize,
    ) -> Self {
        Self {
            block_id,
            cursor,
            marked_range: None,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn marked(
        block_id: String,
        marked_range: Range<usize>,
    ) -> Self {
        Self {
            block_id,
            cursor: marked_range.end,
            marked_range: Some(marked_range),
        }
    }
}

pub(crate) struct PageFocusSession<'a> {
    pub(super) editor: &'a mut PageEditorState,
    pub(super) documents: &'a mut PageDocuments,
}

impl<'a> PageFocusSession<'a> {
    pub(crate) fn new(editor: &'a mut PageEditorState, documents: &'a mut PageDocuments) -> Self {
        Self { editor, documents }
    }

    pub(crate) fn focus_block(&mut self, block_id: String, offset: usize) {
        if !self.editor.tables.editor().clear_unless_composing() {
            return;
        }
        self.reveal_block(&block_id);
        let (block_id, offset) = self
            .documents
            .page_block_text_focus_target(&block_id, offset)
            .unwrap_or((block_id, offset));
        self.reveal_block(&block_id);
        self.editor.active_page_block = Some(block_id.clone());
        *self
            .editor
            .input
            .resource_state()
            .focus_request
            .borrow_mut() = Some(PageBlockFocusRequest::cursor(block_id, offset));
    }

    pub(super) fn restore_history(
        &mut self,
        state: PageHistoryRestoreState,
        cx: &mut App,
    ) -> PageEditTransition<()> {
        let (page_id, focus) = self
            .editor
            .restore_page_history_state(self.documents, state);
        match focus {
            Some(PageEditFocus::Block { block_id, offset }) => {
                self.focus_block(block_id, offset);
                PageEditSession::new(self.editor, self.documents).finish(())
            }
            Some(PageEditFocus::SimpleTableCell { address, offset }) => {
                let mut edit = PageEditSession::new(self.editor, self.documents);
                edit.activate_page_simple_table_cell(
                    &page_id,
                    address,
                    PageSimpleTableCellFocusMode::Offset(offset),
                    cx,
                );
                edit.finish(())
            }
            None => PageEditSession::new(self.editor, self.documents).finish(()),
        }
    }

    fn reveal_block(&mut self, block_id: &str) {
        let Some(data) = self.documents.page_data_containing_editable_block(block_id) else {
            return;
        };
        let expanded = expand_page_block_ancestors(self.editor, &data, block_id);
        if expanded {
            self.documents
                .rebuild_loaded_page_disclosure_projections(self.editor);
        }
        self.documents.reveal_page_block_list_item(block_id);
    }
}

fn expand_page_block_ancestors(
    editor: &mut PageEditorState,
    data: &LoadedCardPageData,
    block_id: &str,
) -> bool {
    let Some(block_index) = data.editable_block_indices.get(block_id).copied() else {
        return false;
    };
    let page_id = &data.page.block_id;
    let mut parent_block_id = data.page.blocks[block_index].parent_block_id.clone();
    let mut expanded = false;
    for _ in 0..data.page.blocks.len() {
        if parent_block_id == page_id.as_str() {
            break;
        }
        let Some(parent) = data
            .page
            .blocks
            .iter()
            .find(|block| block.block_id == parent_block_id)
        else {
            break;
        };
        if parent
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
        {
            expanded |= editor
                .page_toggle_disclosure
                .expand(page_id, &parent.block_id);
        }
        parent_block_id.clone_from(&parent.parent_block_id);
    }
    expanded
}

impl PageEditSession<'_> {
    pub(crate) fn set_active_page_block(&mut self, block_id: String) {
        let Some(data) = self
            .documents
            .page_data_containing_editable_block(&block_id)
        else {
            return;
        };
        if !data.editable_block_is_visible(&block_id) || !data.block_has_text_input(&block_id) {
            return;
        }
        if !self.editor.tables.editor().clear_unless_composing() {
            return;
        }
        self.editor.active_page_block = Some(block_id);
        self.effects.push(PageEditEffect::Notify);
    }

    pub(crate) fn remeasure_page_block_layout(&mut self, block_id: &str) {
        self.remeasure_page_layout(PageDocumentLayoutTarget::Block(block_id));
    }

    pub(crate) fn remeasure_page_table_cell_layout(
        &mut self,
        page_id: &str,
        address: &CardPageSimpleTableCellAddress,
    ) {
        self.remeasure_page_layout(PageDocumentLayoutTarget::SimpleTableCell { page_id, address });
    }

    fn remeasure_page_layout(&mut self, target: PageDocumentLayoutTarget<'_>) {
        let invalidations = self.documents.remeasure_page_layout(target);
        self.editor.invalidate_page_document_layouts(invalidations);
        self.effects.push(PageEditEffect::Notify);
    }
}

impl PageEditorState {
    fn invalidate_page_document_layouts(&self, invalidations: Vec<PageDocumentLayoutInvalidation>) {
        let mut virtualizer = self.flow.state().virtualizer.borrow_mut();
        for invalidation in invalidations {
            virtualizer.invalidate_document_unit_layout(&invalidation.page_id, &invalidation.unit);
        }
    }

    pub(super) fn focus_page_title(&mut self, cx: &mut Context<SurfaceState>) {
        if self
            .page_link_icons
            .picker()
            .is_some_and(|picker| picker.upload_committed)
        {
            return;
        }
        if !self.tables.editor().clear_unless_composing() {
            return;
        }
        self.active_page_block = None;
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
        self.flow.set_committed_focus(None);
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_block_context_menu = None;
        self.page_link_icons.clear_picker();
        self.page_forced_text_annotations = None;
        self.page_rich_text_dialog = None;
        self.clear_page_document_selection(cx);
        cx.notify();
    }

    pub(in crate::ui) fn dismiss_page_block_interaction_from_root(
        &mut self,
        cx: &mut Context<SurfaceState>,
    ) {
        if self.page_block_context_menu.is_some()
            || self.page_link_icons.picker_is_open()
            || self.page_slash_menu.is_some()
            || self.mention.menu_is_open()
        {
            return;
        }
        self.dismiss_page_block_interaction(cx);
    }

    pub(in crate::ui) fn dismiss_page_block_interaction(&mut self, cx: &mut Context<SurfaceState>) {
        if self
            .page_link_icons
            .picker()
            .is_some_and(|picker| picker.upload_committed)
        {
            return;
        }
        if !self.tables.editor().clear_unless_composing() {
            return;
        }
        if self.active_page_block.is_none()
            && self.tables.editor().borrow().is_none()
            && self.page_block_context_menu.is_none()
            && !self.page_link_icons.picker_is_open()
            && self.page_slash_menu.is_none()
            && self.mention.menu().is_none()
            && self.page_text_selection.is_none()
            && self.page_block_selection.block_ids.is_empty()
        {
            return;
        }
        self.active_page_block = None;
        self.page_block_context_menu = None;
        self.page_link_icons.clear_picker();
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.clear_page_document_selection(cx);
        cx.notify();
    }
}
