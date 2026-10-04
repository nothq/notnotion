use gpui::{App, Pixels, Point};
use gpui_components::text_input::{
    TextInputPointerSelection, TextInputPointerSelectionPhase, TextInputSnapshot,
};

use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::PageTextSelection;

use super::editing::{PageEditEffect, PageEditSession, PageEditorEffect};

mod geometry;
mod inputs;
mod visible;

pub(super) use geometry::{page_text_selection_range, page_text_selection_ranges};
pub(super) use visible::{DirectedTextRange, VisiblePageTextSelection};

use geometry::PageTextPointer;

impl PageEditSession<'_> {
    pub(crate) fn update_page_text_selection_from_input(
        &mut self,
        block_id: &str,
        snapshot: TextInputSnapshot,
        cx: &mut App,
    ) {
        let Some(data) = self.documents.page_data_containing_editable_block(block_id) else {
            return;
        };
        if !data.editable_block_is_visible(block_id) {
            return;
        }
        let pointer_selection_is_visible = self
            .editor
            .page_text_selection
            .as_ref()
            .filter(|selection| selection.pointer_active)
            .is_none_or(|selection| VisiblePageTextSelection::new(&data, selection).is_some());
        if !pointer_selection_is_visible {
            self.editor.page_text_selection = None;
            return;
        }
        self.editor
            .retain_page_forced_annotations_at(block_id, &snapshot);
        self.effects.push(PageEditEffect::UpdateSlashMenu {
            block_id: block_id.to_string(),
            snapshot: snapshot.clone(),
        });
        self.editor
            .apply_page_text_selection_snapshot(self.documents, block_id, &snapshot, cx);
        self.effects.push(PageEditEffect::Notify);
    }

    pub(crate) fn update_page_text_selection_from_pointer(
        &mut self,
        block_id: &str,
        event: TextInputPointerSelection,
        cx: &mut App,
    ) {
        let Some(data) = self.documents.page_data_containing_editable_block(block_id) else {
            return;
        };
        if !data.editable_block_is_visible(block_id) {
            return;
        }
        let pointer_anchor_is_visible =
            matches!(event.phase, TextInputPointerSelectionPhase::Begin)
                || self
                    .editor
                    .page_text_selection
                    .as_ref()
                    .filter(|selection| selection.pointer_active)
                    .is_none_or(|selection| {
                        VisiblePageTextSelection::new(&data, selection).is_some()
                    });
        if !pointer_anchor_is_visible {
            self.editor.page_text_selection = None;
            return;
        }
        self.editor.page_forced_text_annotations = None;
        self.editor.page_rich_text_dialog = None;
        self.effects.push(PageEditEffect::UpdateSlashMenu {
            block_id: block_id.to_string(),
            snapshot: event.snapshot.clone(),
        });
        if !self.apply_page_text_pointer_selection_phase(block_id, &event, cx) {
            return;
        }
        self.editor
            .synchronize_page_text_input_selections(self.documents, Some(block_id), cx);
        self.editor.drop_empty_page_text_selection();
        self.effects.push(PageEditEffect::Notify);
    }

    fn apply_page_text_pointer_selection_phase(
        &mut self,
        block_id: &str,
        event: &TextInputPointerSelection,
        cx: &mut App,
    ) -> bool {
        match event.phase {
            TextInputPointerSelectionPhase::Begin => {
                self.editor.page_text_selection = Some(PageTextSelection {
                    anchor_block_id: block_id.to_string(),
                    anchor_offset: event.anchor,
                    focus_block_id: block_id.to_string(),
                    focus_offset: event.snapshot.cursor,
                    pointer_active: true,
                });
                self.editor.page_block_selection.block_ids.clear();
            }
            TextInputPointerSelectionPhase::Update => {
                let Some(selection) = self.editor.page_text_selection.as_mut() else {
                    return false;
                };
                selection.focus_block_id = block_id.to_string();
                selection.focus_offset = event.snapshot.cursor;
                selection.pointer_active = true;
            }
            TextInputPointerSelectionPhase::End => {
                let Some(selection) = self.editor.page_text_selection.as_mut() else {
                    return false;
                };
                selection.focus_block_id = block_id.to_string();
                selection.focus_offset = event.snapshot.cursor;
                selection.pointer_active = false;
            }
            TextInputPointerSelectionPhase::EndOutside => {
                self.update_page_text_selection_at_position(event.position, Some(block_id), cx);
                if let Some(selection) = self.editor.page_text_selection.as_mut() {
                    selection.pointer_active = false;
                }
            }
        }
        true
    }

    pub(crate) fn update_page_text_selection_at_position(
        &mut self,
        position: Point<Pixels>,
        excluded_block_id: Option<&str>,
        cx: &mut App,
    ) -> bool {
        let Some(selection) = self
            .editor
            .page_text_selection
            .as_ref()
            .filter(|selection| selection.pointer_active)
            .cloned()
        else {
            return false;
        };
        let Some((data, surface)) = self
            .documents
            .page_data_and_flow_surface_containing_editable_block(&selection.anchor_block_id)
        else {
            self.editor.page_text_selection = None;
            return false;
        };
        let pointer = PageTextPointer {
            position,
            excluded_block_id,
        };
        let Some((block_id, offset)) = self.editor.page_text_position(&data, surface, pointer, cx)
        else {
            return true;
        };
        if let Some(selection) = self.editor.page_text_selection.as_mut() {
            selection.focus_block_id = block_id;
            selection.focus_offset = offset;
        }
        self.editor
            .synchronize_page_text_input_selections(self.documents, excluded_block_id, cx);
        self.effects.push(PageEditEffect::Notify);
        true
    }

    pub(crate) fn select_all_page_blocks(&mut self, block_id: &str, cursor: usize, _cx: &mut App) {
        self.editor.page_forced_text_annotations = None;
        let Some(page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        self.editor.page_text_selection = None;
        self.editor.page_block_selection.block_ids = page
            .blocks
            .iter()
            .filter(|block| {
                !block.is_layout_container()
                    && !block.is_opaque_unavailable()
                    && block.simple_table_row_content().is_none()
            })
            .map(|block| block.block_id.clone())
            .collect();
        if let Some(input) = self
            .editor
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .get(block_id)
            .cloned()
        {
            self.effects.push(PageEditEffect::Editor(
                PageEditorEffect::DeferInputSelection {
                    input,
                    range: cursor..cursor,
                    reversed: false,
                },
            ));
        }
        self.effects.push(PageEditEffect::Notify);
    }
}

impl PageEditorState {
    fn apply_page_text_selection_snapshot(
        &mut self,
        documents: &PageDocuments,
        block_id: &str,
        snapshot: &TextInputSnapshot,
        cx: &mut App,
    ) {
        if let Some(selection) = self
            .page_text_selection
            .as_mut()
            .filter(|selection| selection.pointer_active)
        {
            selection.focus_block_id = block_id.to_string();
            selection.focus_offset = snapshot.cursor;
            self.synchronize_page_text_input_selections(documents, Some(block_id), cx);
            return;
        }
        if snapshot.selection.is_empty() {
            self.collapse_page_text_input_selections(documents, block_id, Some(block_id), cx);
            self.page_text_selection = None;
            self.page_block_selection.block_ids.clear();
            return;
        }
        let anchor_offset = if snapshot.cursor == snapshot.selection.start {
            snapshot.selection.end
        } else {
            snapshot.selection.start
        };
        self.page_text_selection = Some(PageTextSelection {
            anchor_block_id: block_id.to_string(),
            anchor_offset,
            focus_block_id: block_id.to_string(),
            focus_offset: snapshot.cursor,
            pointer_active: false,
        });
        self.page_block_selection.block_ids.clear();
        self.synchronize_page_text_input_selections(documents, Some(block_id), cx);
    }
    pub(crate) fn finish_page_text_selection(&mut self) -> bool {
        let Some(selection) = self.page_text_selection.as_mut() else {
            return false;
        };
        if !selection.pointer_active {
            return false;
        }
        selection.pointer_active = false;
        self.drop_empty_page_text_selection();
        true
    }

    pub(super) fn drop_empty_page_text_selection(&mut self) {
        let empty = self.page_text_selection.as_ref().is_some_and(|selection| {
            selection.anchor_block_id == selection.focus_block_id
                && selection.anchor_offset == selection.focus_offset
        });
        if empty {
            self.page_text_selection = None;
        }
    }

    fn retain_page_forced_annotations_at(&mut self, block_id: &str, snapshot: &TextInputSnapshot) {
        let composing = snapshot.is_composing
            && self
                .page_pending_rich_text_composition
                .as_ref()
                .is_some_and(|composition| composition.block_id == block_id);
        let keep = composing
            || snapshot.selection.is_empty()
                && self
                    .page_forced_text_annotations
                    .as_ref()
                    .is_some_and(|forced| {
                        forced.block_id == block_id && forced.offset_utf8 == snapshot.cursor
                    });
        if !keep {
            self.page_forced_text_annotations = None;
        }
        if !snapshot.selection.is_empty() {
            self.page_rich_text_dialog = None;
        }
    }
}
