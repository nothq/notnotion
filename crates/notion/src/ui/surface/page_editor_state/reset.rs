use super::{PageBlockSelection, PageEditorState};

impl PageEditorState {
    pub(crate) fn reset_page_composer(&mut self) {
        self.input.reset_composer();
        self.page_slash_menu = None;
        self.mention.clear_menu();
    }

    pub(crate) fn reset_for_route(&mut self) {
        self.reset_page_composer();
        self.input.reset_for_route();
        self.flow.clear_for_route();

        self.active_page_block = None;
        self.hovered_page_block = None;
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
        self.page_block_compositions.clear();
        self.page_block_context_menu = None;
        self.page_link_icons.reset_for_route();
        self.page_text_selection = None;
        self.page_forced_text_annotations = None;
        self.page_pending_rich_text_typing = None;
        self.page_pending_rich_text_composition = None;
        self.page_pending_cross_block_composition = None;
        self.page_rich_text_dialog = None;
        self.page_rich_text_link_value.clear();
        self.page_rich_text_link_input.borrow_mut().take();
        self.page_block_selection = PageBlockSelection::default();

        self.drag.reset_for_route();
    }
}
