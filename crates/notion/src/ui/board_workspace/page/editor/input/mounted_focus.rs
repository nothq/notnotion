use gpui::App;

use crate::ui::surface::{PageFlowPinTarget, PageInputResources};
use crate::ui::{LoadedCardPageData, PageDocumentUnitKey};

use super::super::render::PageBlockRenderer;
use super::super::selection::DirectedTextRange;

impl PageInputResources {
    fn complete_recursive_page_block_focus(
        &self,
        data: &LoadedCardPageData,
        target: &PageFlowPinTarget,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> bool {
        let PageFlowPinTarget::Unit(PageDocumentUnitKey::Block { block_id }) = target else {
            return false;
        };
        let Some(request) = self.take_page_block_focus_request(block_id) else {
            return false;
        };
        let Some(input) = self
            .state()
            .block_inputs
            .borrow()
            .get(block_id.as_ref())
            .cloned()
        else {
            *self.state().focus_request.borrow_mut() = Some(request);
            return false;
        };
        let Some(text) = data
            .page_block(block_id)
            .and_then(|block| block.editable_content())
            .map(|editable| editable.text.clone())
        else {
            return false;
        };
        input.update(cx, move |input, cx| {
            if let Some(marked_range) = request.marked_range {
                input.set_text_and_mark_range(text, marked_range, cx);
            } else {
                input.set_text_and_move_cursor(text, request.cursor, cx);
            }
        });
        window.focus(&input.read(cx).focus_handle_clone(), cx);
        true
    }

    pub(super) fn take_page_block_focus_request(
        &self,
        block_id: &str,
    ) -> Option<super::super::PageBlockFocusRequest> {
        let matches = self
            .state()
            .focus_request
            .borrow()
            .as_ref()
            .is_some_and(|request| request.block_id == block_id);
        matches
            .then(|| self.state().focus_request.borrow_mut().take())
            .flatten()
    }
}

impl PageBlockRenderer {
    pub(in crate::ui::board_workspace::page::editor) fn complete_recursive_page_block_focus(
        &self,
        data: &LoadedCardPageData,
        target: &PageFlowPinTarget,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> bool {
        self.input_resources
            .complete_recursive_page_block_focus(data, target, window, cx)
    }
}

impl PageBlockRenderer {
    pub(super) fn page_text_input_selection(
        &self,
        page_data: &std::sync::Arc<LoadedCardPageData>,
        block_id: &str,
    ) -> Option<DirectedTextRange> {
        self.interaction.input_selection(page_data, block_id)
    }
}
