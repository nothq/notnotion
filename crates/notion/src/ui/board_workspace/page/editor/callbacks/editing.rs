use gpui::{App, Entity};
use gpui_components::text_input::TextInput;

use super::super::input::{
    PageInputAction, PageInputHostAction, PageNavigationInputAction, PageTextInputAction,
};
use super::super::navigation::PageBlockNavigationRequest;
use super::super::PageEditSession;

pub(super) enum PageInputDispatch {
    Edited(Option<Entity<TextInput>>),
    Host(PageInputHostAction),
}

impl PageEditSession<'_> {
    pub(super) fn apply_input_action(
        &mut self,
        action: PageInputAction,
        cx: &mut App,
    ) -> PageInputDispatch {
        match action {
            PageInputAction::Text(action) => {
                self.apply_text_input_action(action);
                PageInputDispatch::Edited(None)
            }
            PageInputAction::Navigation(action) => {
                PageInputDispatch::Edited(self.apply_navigation_input_action(action, cx))
            }
            PageInputAction::Host(action) => PageInputDispatch::Host(action),
        }
    }

    fn apply_text_input_action(&mut self, action: PageTextInputAction) {
        match action {
            PageTextInputAction::BlockChanged { block_id, snapshot } => {
                self.editor.clear_page_composer_handoff(&block_id);
                self.apply_page_block_text_change(&block_id, snapshot);
            }
            PageTextInputAction::MergeBackward { block_id, snapshot } => {
                self.merge_page_block_backward(&block_id, snapshot)
            }
            PageTextInputAction::MergeForward { block_id, snapshot } => {
                self.merge_page_block_forward(&block_id, snapshot)
            }
            PageTextInputAction::Indent {
                block_id,
                outdent,
                snapshot,
            } => self.indent_page_block(&block_id, outdent, snapshot),
            PageTextInputAction::History {
                block_id,
                redo,
                cursor,
            } => {
                self.restore_page_edit_history_for_block(&block_id, cursor, redo);
            }
        }
    }

    fn apply_navigation_input_action(
        &mut self,
        action: PageNavigationInputAction,
        cx: &mut App,
    ) -> Option<Entity<TextInput>> {
        match action {
            PageNavigationInputAction::Navigate {
                block_id,
                navigation,
                snapshot,
                window_x,
                extend,
            } => self.navigate_page_block_text(
                PageBlockNavigationRequest {
                    block_id: &block_id,
                    navigation,
                    snapshot,
                    window_x,
                    extend,
                },
                cx,
            ),
            PageNavigationInputAction::SelectAll { block_id, cursor } => {
                self.select_all_page_blocks(&block_id, cursor, cx);
                None
            }
            PageNavigationInputAction::SelectionChanged { block_id, snapshot } => {
                self.update_page_text_selection_from_input(&block_id, snapshot, cx);
                None
            }
            PageNavigationInputAction::PointerSelectionChanged {
                block_id,
                selection,
            } => {
                self.update_page_text_selection_from_pointer(&block_id, selection, cx);
                None
            }
            PageNavigationInputAction::Focused(block_id) => {
                self.set_active_page_block(block_id);
                None
            }
            PageNavigationInputAction::LayoutChanged(block_id) => {
                self.remeasure_page_block_layout(&block_id);
                None
            }
        }
    }
}
