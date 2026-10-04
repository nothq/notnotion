use gpui::App;
use gpui_components::text_input::TextInputProps;

use super::super::callbacks::{
    page_block_on_focus, page_block_on_history, page_block_on_keyboard_move,
    page_block_on_layout_change, page_block_on_navigation, page_block_on_pointer_selection,
    page_block_on_select_all_again, page_block_on_selection_change, page_block_on_tab,
    page_block_on_vertical_navigation,
};
use super::super::navigation::PageBlockNavigation;
use super::super::render::PageBlockRenderer;
use super::PageBlockInputBehavior;

impl PageBlockRenderer {
    pub(super) fn page_block_navigation_input_props(
        &self,
        props: TextInputProps,
        block_id: &str,
        behavior: PageBlockInputBehavior,
        _cx: &mut App,
    ) -> TextInputProps {
        let props = self.page_block_directional_input_props(props, block_id);
        let props = props
            .on_undo_with_state(page_block_on_history(
                block_id.to_string(),
                false,
                self.input_bindings.actions.clone(),
            ))
            .on_redo_with_state(page_block_on_history(
                block_id.to_string(),
                true,
                self.input_bindings.actions.clone(),
            ))
            .on_platform_shift_up_with_state(page_block_on_keyboard_move(
                block_id.to_string(),
                -1,
                self.input_bindings.actions.clone(),
            ))
            .on_platform_shift_down_with_state(page_block_on_keyboard_move(
                block_id.to_string(),
                1,
                self.input_bindings.actions.clone(),
            ))
            .on_select_all_again_with_state(page_block_on_select_all_again(
                block_id.to_string(),
                self.input_bindings.actions.clone(),
            ))
            .on_selection_change(page_block_on_selection_change(
                block_id.to_string(),
                self.input_bindings.actions.clone(),
            ))
            .on_pointer_selection(page_block_on_pointer_selection(
                block_id.to_string(),
                self.input_bindings.actions.clone(),
            ))
            .on_focus(page_block_on_focus(
                block_id.to_string(),
                self.input_bindings.actions.clone(),
            ))
            .on_layout_change(page_block_on_layout_change(
                block_id.to_string(),
                self.input_bindings.actions.clone(),
            ));
        match behavior {
            PageBlockInputBehavior::Document | PageBlockInputBehavior::PageLink => props
                .on_tab_with_state(page_block_on_tab(
                    block_id.to_string(),
                    self.input_bindings.actions.clone(),
                )),
            PageBlockInputBehavior::Code { .. } => props,
        }
    }

    fn page_block_directional_input_props(
        &self,
        props: TextInputProps,
        block_id: &str,
    ) -> TextInputProps {
        props
            .on_left_before_default(self.input_bindings.collapse_left(block_id))
            .on_right_before_default(self.input_bindings.collapse_right(block_id))
            .on_left_at_start(page_block_on_navigation(
                block_id.to_string(),
                PageBlockNavigation::Left,
                self.input_bindings.actions.clone(),
            ))
            .on_right_at_end(page_block_on_navigation(
                block_id.to_string(),
                PageBlockNavigation::Right,
                self.input_bindings.actions.clone(),
            ))
            .on_up_at_first_visual_line(page_block_on_vertical_navigation(
                block_id.to_string(),
                PageBlockNavigation::Up,
                self.input_bindings.actions.clone(),
            ))
            .on_down_at_last_visual_line(page_block_on_vertical_navigation(
                block_id.to_string(),
                PageBlockNavigation::Down,
                self.input_bindings.actions.clone(),
            ))
    }
}
