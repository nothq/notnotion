use gpui::App;

use super::focus::PageFocusSession;
use super::LoadedCardPageData;

mod simple_table;
mod state;

#[derive(Clone, Copy)]
pub(super) enum PageEditorDragReset {
    PreserveHandleInteraction,
    CancelHandleInteraction,
}

struct ProjectionInvalidations {
    active_block_hidden: bool,
    text_selection_hidden: bool,
    forced_annotations_hidden: bool,
}

impl ProjectionInvalidations {
    fn closes_rich_text_dialog(&self) -> bool {
        self.active_block_hidden || self.text_selection_hidden || self.forced_annotations_hidden
    }
}

impl PageFocusSession<'_> {
    pub(crate) fn reconcile_active_projection(&mut self, cx: &mut App) {
        let Some(data) = self.documents.active_page_data() else {
            self.editor.clear_projection_state();
            return;
        };
        self.editor.page_toggle_disclosure.retain_valid(&data);
        self.editor.retain_valid_simple_table_scroll_handles(&data);
        self.reconcile_projection(&data, None, cx);
    }

    pub(crate) fn reconcile_projection(
        &mut self,
        data: &LoadedCardPageData,
        fallback_focus: Option<(&str, usize)>,
        cx: &mut App,
    ) {
        let authority = self.documents.page_authority_with_id(&data.page.block_id);
        self.editor
            .prune_projection_simple_table_cell(data, authority.as_deref());
        self.redirect_projection_focus_targets(data);
        let active_block_hidden = self.reconcile_projection_focus_visibility(data, fallback_focus);
        self.editor.prune_projection_overlays(data);
        let text_selection_hidden = self.editor.reconcile_projection_text_selection(data, cx);
        self.editor.prune_projection_inputs(data);
        let forced_annotations_hidden = self.editor.prune_projection_rich_text_state(data);
        let invalidations = ProjectionInvalidations {
            active_block_hidden,
            text_selection_hidden,
            forced_annotations_hidden,
        };
        if invalidations.closes_rich_text_dialog() {
            self.editor.clear_projection_rich_text_dialog();
        }
        self.editor
            .reset_drag(PageEditorDragReset::PreserveHandleInteraction);
    }

    fn redirect_projection_focus_targets(&mut self, data: &LoadedCardPageData) {
        self.redirect_active_projection_focus(data);
        self.redirect_pending_projection_focus(data);
    }

    fn redirect_active_projection_focus(&mut self, data: &LoadedCardPageData) {
        let active_without_input = self
            .editor
            .active_page_block
            .as_deref()
            .filter(|block_id| {
                data.block_is_visible(block_id) && !data.block_has_text_input(block_id)
            })
            .map(str::to_owned);
        let Some(block_id) = active_without_input else {
            return;
        };
        if let Some((target_id, offset)) = self.documents.page_block_text_focus_target(&block_id, 0)
        {
            self.focus_block(target_id, offset);
        } else {
            self.editor.active_page_block = None;
        }
    }

    fn redirect_pending_projection_focus(&mut self, data: &LoadedCardPageData) {
        let pending_without_input = self
            .editor
            .input
            .resource_state()
            .focus_request
            .borrow()
            .as_ref()
            .filter(|focus| {
                data.block_is_visible(&focus.block_id)
                    && !data.block_has_text_input(&focus.block_id)
            })
            .map(|focus| (focus.block_id.clone(), focus.cursor));
        let Some((block_id, offset)) = pending_without_input else {
            return;
        };
        if let Some((target_id, target_offset)) = self
            .documents
            .page_block_text_focus_target(&block_id, offset)
        {
            self.focus_block(target_id, target_offset);
        } else {
            self.editor
                .input
                .resource_state()
                .focus_request
                .borrow_mut()
                .take();
        }
    }

    fn reconcile_projection_focus_visibility(
        &mut self,
        data: &LoadedCardPageData,
        fallback_focus: Option<(&str, usize)>,
    ) -> bool {
        let active_hidden = self
            .editor
            .active_page_block
            .as_ref()
            .is_some_and(|block_id| !data.block_is_visible(block_id));
        let pending_hidden = self
            .editor
            .input
            .resource_state()
            .focus_request
            .borrow()
            .as_ref()
            .is_some_and(|focus| !data.editable_block_is_visible(&focus.block_id));
        if active_hidden {
            if let Some((block_id, offset)) = fallback_focus {
                self.focus_block(block_id.to_string(), offset);
            } else {
                self.editor.active_page_block = None;
                self.editor
                    .input
                    .resource_state()
                    .focus_request
                    .borrow_mut()
                    .take();
            }
        } else if pending_hidden {
            self.editor
                .input
                .resource_state()
                .focus_request
                .borrow_mut()
                .take();
        }
        active_hidden
    }
}
