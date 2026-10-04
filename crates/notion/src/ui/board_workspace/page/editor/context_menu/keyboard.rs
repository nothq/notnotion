use super::super::KeyDownEvent;
use super::actions::PageBlockContextMenuAction;
use super::state::{
    page_block_menu_action_panel, PageBlockContextMenuPanel, PageBlockContextMenuState,
    PageBlockContextMenuTarget,
};
use crate::ui::keystroke_input_text;

impl PageBlockContextMenuState {
    pub(super) fn keyboard_action(
        &self,
        event: &KeyDownEvent,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> Option<PageBlockContextMenuAction> {
        match event.keystroke.key.as_str() {
            "escape" => Some(PageBlockContextMenuAction::Dismiss),
            "backspace"
                if self.panel == PageBlockContextMenuPanel::Root && !self.query.is_empty() =>
            {
                let mut query = self.query.clone();
                query.pop();
                Some(PageBlockContextMenuAction::SetRootQuery(query))
            }
            "up" => Some(PageBlockContextMenuAction::MoveSelection(-1)),
            "down" => Some(PageBlockContextMenuAction::MoveSelection(1)),
            "left" if self.panel != PageBlockContextMenuPanel::Root => {
                Some(PageBlockContextMenuAction::CloseSubmenu)
            }
            "right" if self.panel == PageBlockContextMenuPanel::Root => {
                let target = target.filter(|target| target.block.block_id == self.block_id)?;
                let action = self.selected_root_action(target)?;
                (page_block_menu_action_panel(action) != PageBlockContextMenuPanel::Root
                    && target.action_enabled(action))
                .then(|| PageBlockContextMenuAction::ActivateRoot {
                    block_id: self.block_id.clone(),
                    action,
                })
            }
            "enter" => Some(PageBlockContextMenuAction::Submit),
            _ if self.panel == PageBlockContextMenuPanel::Root => {
                let text = keystroke_input_text(event)?;
                Some(PageBlockContextMenuAction::SetRootQuery(format!(
                    "{}{text}",
                    self.query
                )))
            }
            _ => None,
        }
    }
}
