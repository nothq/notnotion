use std::rc::Rc;

use super::super::callbacks::{page_slash_menu_close, page_slash_menu_move};
use super::{PageBlockRenderer, TextInputProps};
use crate::ui::board_workspace::page::editor::mention::PageMentionAction;

impl PageBlockRenderer {
    pub(super) fn page_block_menu_input_props(
        &self,
        mut props: TextInputProps,
        block_id: &str,
    ) -> TextInputProps {
        if self.interaction.mention_menu_is_open(block_id) {
            return props
                .on_up(page_mention_menu_action(
                    block_id.to_string(),
                    Some(-1),
                    self.input_bindings.mention_actions.clone(),
                ))
                .on_down(page_mention_menu_action(
                    block_id.to_string(),
                    Some(1),
                    self.input_bindings.mention_actions.clone(),
                ))
                .on_escape(page_mention_menu_action(
                    block_id.to_string(),
                    None,
                    self.input_bindings.mention_actions.clone(),
                ));
        }
        if self.interaction.slash_menu_is_open(block_id) {
            props = props
                .on_up(page_slash_menu_move(
                    block_id.to_string(),
                    -1,
                    self.input_bindings.actions.clone(),
                ))
                .on_down(page_slash_menu_move(
                    block_id.to_string(),
                    1,
                    self.input_bindings.actions.clone(),
                ))
                .on_escape(page_slash_menu_close(
                    block_id.to_string(),
                    self.input_bindings.actions.clone(),
                ));
        }
        props
    }
}

fn page_mention_menu_action(
    block_id: String,
    delta: Option<isize>,
    actions: crate::ui::view_actions::ViewActionSink<PageMentionAction>,
) -> gpui_components::text_input::TextInputAction {
    Rc::new(move |window, cx| {
        let action = match delta {
            Some(delta) => PageMentionAction::MoveMenu {
                block_id: block_id.clone(),
                delta,
            },
            None => PageMentionAction::CloseMenu {
                block_id: block_id.clone(),
            },
        };
        actions.emit(action, window, cx);
    })
}
