use gpui::App;
use gpui_components::text_input::TextInput;

use super::VisiblePageTextSelection;
use crate::ui::surface::{PageDocuments, PageEditorState};

pub(in crate::ui::board_workspace::page::editor) struct PageDeferredInputSelection {
    pub(in crate::ui::board_workspace::page::editor) input: Option<gpui::Entity<TextInput>>,
    pub(in crate::ui::board_workspace::page::editor) cursor: usize,
}

impl PageEditorState {
    pub(crate) fn clear_page_document_selection(&mut self, cx: &mut App) -> bool {
        self.page_forced_text_annotations = None;
        self.page_rich_text_dialog = None;
        self.clear_page_document_selection_excluding(None, cx)
    }

    pub(in crate::ui::board_workspace::page::editor) fn clear_page_document_selection_from_input(
        &mut self,
        source_block_id: &str,
        source_cursor: usize,
        cx: &mut App,
    ) -> Option<PageDeferredInputSelection> {
        if !self.clear_page_document_selection_excluding(Some(source_block_id), cx) {
            return None;
        }
        let source = self
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .get(source_block_id)
            .cloned();
        Some(PageDeferredInputSelection {
            input: source,
            cursor: source_cursor,
        })
    }
    pub(in crate::ui::board_workspace::page::editor) fn synchronize_page_text_input_selections(
        &self,
        documents: &PageDocuments,
        excluded_block_id: Option<&str>,
        cx: &mut App,
    ) {
        let Some(selection) = self.page_text_selection.as_ref() else {
            return;
        };
        let Some(data) = documents.page_data_containing_editable_block(&selection.anchor_block_id)
        else {
            return;
        };
        let Some(visible_selection) = VisiblePageTextSelection::new(&data, selection) else {
            return;
        };
        let inputs = self.input.resource_state().block_inputs.borrow();
        let mut updates = inputs
            .iter()
            .filter_map(|(block_id, input)| {
                if excluded_block_id == Some(block_id.as_str()) {
                    return None;
                }
                let index = *data.editable_block_indices.get(block_id)?;
                let (range, reversed) = visible_selection.range_for(index).unwrap_or_else(|| {
                    let cursor = input.read(cx).selection_range().end;
                    (cursor..cursor, false)
                });
                Some((index, input.clone(), range, reversed))
            })
            .collect::<Vec<_>>();
        drop(inputs);
        updates.sort_by_key(|(index, _, _, _)| *index);
        for (_, input, range, reversed) in updates {
            input.update(cx, |input, cx| {
                input.set_selection(range, reversed, cx);
            });
        }
    }

    pub(super) fn collapse_page_text_input_selections(
        &self,
        documents: &PageDocuments,
        page_member_block_id: &str,
        excluded_block_id: Option<&str>,
        cx: &mut App,
    ) {
        let Some(data) = documents.page_data_containing_editable_block(page_member_block_id) else {
            return;
        };
        let inputs = self.input.resource_state().block_inputs.borrow();
        let mut updates = inputs
            .iter()
            .filter_map(|(block_id, input)| {
                if excluded_block_id == Some(block_id.as_str()) {
                    return None;
                }
                let index = *data.editable_block_indices.get(block_id)?;
                let cursor = input.read(cx).selection_range().end;
                Some((index, input.clone(), cursor))
            })
            .collect::<Vec<_>>();
        drop(inputs);
        updates.sort_by_key(|(index, _, _)| *index);
        for (_, input, cursor) in updates {
            input.update(cx, |input, cx| {
                input.set_selection(cursor..cursor, false, cx);
            });
        }
    }
    fn clear_page_document_selection_excluding(
        &mut self,
        excluded_block_id: Option<&str>,
        cx: &mut App,
    ) -> bool {
        if self.page_text_selection.take().is_none()
            && self.page_block_selection.block_ids.is_empty()
        {
            return false;
        }
        self.page_block_selection.block_ids.clear();
        let inputs = self
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .iter()
            .filter(|(block_id, _)| excluded_block_id != Some(block_id.as_str()))
            .map(|(_, input)| input.clone())
            .collect::<Vec<_>>();
        for input in inputs {
            let cursor = input.read(cx).selection_range().end;
            input.update(cx, |input, cx| {
                input.set_selection(cursor..cursor, false, cx);
            });
        }
        true
    }
}
