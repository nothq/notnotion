use super::super::super::{px, CardPageQuoteSize, PAGE_CODE_LANGUAGES};
use super::super::catalog::{PageBlockMenuAction, PAGE_BLOCK_TURN_INTO_ACTIONS};
use super::super::state::{
    page_block_menu_action_panel, page_block_menu_panel_action, PageBlockContextMenuPanel,
    PageBlockContextMenuPresentation, PageBlockContextMenuState, PageBlockContextMenuTarget,
    PageBlockMenuMutation, PageBlockMenuSelection,
};
use super::super::submenu::color::page_block_color_menu_row_color;
use super::editing::PageBlockContextEditCommand;
use super::{
    PageBlockContextMenuAction, PageBlockContextMenuCommand, PageBlockContextMenuTransition,
    PageBlockRootActionOrigin,
};
use crate::model::CardPageCodeLanguage;
use crate::ui::surface::PageEditorState;

impl PageEditorState {
    pub(super) fn reduce_page_block_context_menu(
        &mut self,
        action: PageBlockContextMenuAction,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        match action {
            PageBlockContextMenuAction::Dismiss => self.dismiss_page_block_context_menu(),
            PageBlockContextMenuAction::DismissInteraction => {
                PageBlockContextMenuTransition::command(
                    PageBlockContextMenuCommand::DismissInteraction,
                )
            }
            PageBlockContextMenuAction::SetRootQuery(query) => self
                .page_block_context_menu
                .as_mut()
                .map_or_else(PageBlockContextMenuTransition::default, |menu| {
                    menu.set_root_query(query, target).into()
                }),
            PageBlockContextMenuAction::SetCodeLanguageQuery(query) => self
                .page_block_context_menu
                .as_mut()
                .map_or_else(PageBlockContextMenuTransition::default, |menu| {
                    menu.set_code_language_query(query).into()
                }),
            PageBlockContextMenuAction::Submit => self.submit_page_block_context_menu(target),
            PageBlockContextMenuAction::MoveSelection(delta) => self
                .page_block_context_menu
                .as_mut()
                .map_or_else(PageBlockContextMenuTransition::default, |menu| {
                    menu.move_selection(delta, target).into()
                }),
            PageBlockContextMenuAction::CloseSubmenu => {
                self.close_page_block_context_submenu(target)
            }
            PageBlockContextMenuAction::OpenSelectedSubmenu => {
                self.open_selected_page_block_context_submenu(target)
            }
            PageBlockContextMenuAction::DeleteTarget => {
                self.delete_page_block_context_menu_target(target)
            }
            PageBlockContextMenuAction::ActivateRoot { block_id, action } => {
                self.activate_page_block_menu_root(&block_id, action, target, false)
            }
            PageBlockContextMenuAction::ActivateSubmenuRow {
                block_id,
                panel,
                index,
            } => self.activate_page_block_submenu_row(&block_id, panel, index, target),
            PageBlockContextMenuAction::HoverRoot {
                block_id,
                action,
                index,
            } => self.hover_page_block_menu_root(&block_id, action, index, target),
            PageBlockContextMenuAction::HoverSubmenu {
                block_id,
                panel,
                index,
            } => self.hover_page_block_submenu(&block_id, panel, index),
        }
    }

    fn dismiss_page_block_context_menu(&mut self) -> PageBlockContextMenuTransition {
        if self.page_block_context_menu.take().is_some() {
            PageBlockContextMenuTransition::notified()
        } else {
            PageBlockContextMenuTransition::default()
        }
    }

    fn submit_page_block_context_menu(
        &mut self,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        let Some(menu) = self.page_block_context_menu.as_ref() else {
            return PageBlockContextMenuTransition::default();
        };
        let block_id = menu.block_id.clone();
        let panel = menu.panel;
        let query = menu.query.clone();
        let Some(selected) = menu.selection.index() else {
            self.page_block_context_menu = None;
            return PageBlockContextMenuTransition::notified();
        };
        match panel {
            PageBlockContextMenuPanel::Root => {
                let Some(target) = matching_target(target, &block_id) else {
                    return PageBlockContextMenuTransition::default();
                };
                let Some(action) = target.root_action_at(&query, selected) else {
                    return PageBlockContextMenuTransition::default();
                };
                self.activate_page_block_menu_root(&block_id, action, Some(target), false)
            }
            PageBlockContextMenuPanel::TurnInto
            | PageBlockContextMenuPanel::Color
            | PageBlockContextMenuPanel::QuoteSize
            | PageBlockContextMenuPanel::CodeLanguage => {
                self.activate_page_block_submenu_row(&block_id, panel, selected, target)
            }
        }
    }

    fn close_page_block_context_submenu(
        &mut self,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        let Some(menu) = self.page_block_context_menu.as_ref() else {
            return PageBlockContextMenuTransition::default();
        };
        if menu.panel == PageBlockContextMenuPanel::Root {
            return PageBlockContextMenuTransition::default();
        }
        if menu.presentation == PageBlockContextMenuPresentation::CodeLanguagePicker {
            self.page_block_context_menu = None;
            return PageBlockContextMenuTransition::notified();
        }
        let Some(owner) = page_block_menu_panel_action(menu.panel) else {
            return PageBlockContextMenuTransition::default();
        };
        let root_selection = matching_target(target, &menu.block_id)
            .and_then(|target| target.filtered_action_index(&menu.query, owner))
            .map_or(PageBlockMenuSelection::Hidden, PageBlockMenuSelection::Row);
        let menu = self
            .page_block_context_menu
            .as_mut()
            .expect("page block menu must remain open");
        PageBlockMenuMutation::changed(
            menu.set_panel(PageBlockContextMenuPanel::Root, root_selection),
        )
        .into()
    }

    fn open_selected_page_block_context_submenu(
        &mut self,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        let Some(menu) = self.page_block_context_menu.as_ref() else {
            return PageBlockContextMenuTransition::default();
        };
        let Some(target) = matching_target(target, &menu.block_id) else {
            return PageBlockContextMenuTransition::default();
        };
        let Some(action) = menu.selected_root_action(target) else {
            return PageBlockContextMenuTransition::default();
        };
        if page_block_menu_action_panel(action) == PageBlockContextMenuPanel::Root {
            return PageBlockContextMenuTransition::default();
        }
        let block_id = menu.block_id.clone();
        self.activate_page_block_menu_root(&block_id, action, Some(target), true)
    }

    fn delete_page_block_context_menu_target(
        &self,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        let Some(menu) = self
            .page_block_context_menu
            .as_ref()
            .filter(|menu| menu.query.is_empty())
        else {
            return PageBlockContextMenuTransition::default();
        };
        let Some(target) = matching_target(target, &menu.block_id) else {
            return PageBlockContextMenuTransition::default();
        };
        if !target.action_enabled(PageBlockMenuAction::Delete) {
            return PageBlockContextMenuTransition::default();
        }
        PageBlockContextMenuTransition::command(PageBlockContextMenuCommand::Root {
            block_id: menu.block_id.clone(),
            action: PageBlockMenuAction::Delete,
            origin: PageBlockRootActionOrigin::Menu,
        })
    }

    fn activate_page_block_menu_root(
        &mut self,
        block_id: &str,
        action: PageBlockMenuAction,
        target: Option<&PageBlockContextMenuTarget>,
        submenu_only: bool,
    ) -> PageBlockContextMenuTransition {
        let Some(target) = matching_target(target, block_id) else {
            return PageBlockContextMenuTransition::default();
        };
        if self
            .page_block_context_menu
            .as_ref()
            .is_none_or(|menu| menu.block_id != block_id)
            || !target.action_visible(action)
            || !target.action_enabled(action)
        {
            return PageBlockContextMenuTransition::default();
        }
        let panel = page_block_menu_action_panel(action);
        if panel == PageBlockContextMenuPanel::Root {
            if submenu_only {
                return PageBlockContextMenuTransition::default();
            }
            return PageBlockContextMenuTransition::command(PageBlockContextMenuCommand::Root {
                block_id: block_id.to_string(),
                action,
                origin: PageBlockRootActionOrigin::Menu,
            });
        }
        let selection = target.submenu_selection(action);
        let menu = self
            .page_block_context_menu
            .as_mut()
            .expect("matching page block menu must remain open");
        if let (PageBlockContextMenuPanel::CodeLanguage, Some(selected_index)) =
            (panel, selection.index())
        {
            menu.code_language_list_state.scroll_to(gpui::ListOffset {
                item_ix: selected_index.saturating_sub(10),
                offset_in_item: px(0.0),
            });
        }
        PageBlockMenuMutation::changed(menu.set_panel(panel, selection)).into()
    }

    fn activate_page_block_submenu_row(
        &self,
        block_id: &str,
        panel: PageBlockContextMenuPanel,
        selected: usize,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        let Some(menu) = self
            .page_block_context_menu
            .as_ref()
            .filter(|menu| menu.block_id == block_id && menu.panel == panel)
        else {
            return PageBlockContextMenuTransition::default();
        };
        let Some(target) = matching_target(target, block_id) else {
            return PageBlockContextMenuTransition::default();
        };
        target.submenu_command(menu, selected, self.last_used_page_block_color)
    }

    fn hover_page_block_menu_root(
        &mut self,
        block_id: &str,
        action: PageBlockMenuAction,
        index: usize,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockContextMenuTransition {
        let Some(target) = matching_target(target, block_id) else {
            return PageBlockContextMenuTransition::default();
        };
        if !target.action_visible(action) || !target.action_enabled(action) {
            return PageBlockContextMenuTransition::default();
        }
        let panel = page_block_menu_action_panel(action);
        let selection = if panel == PageBlockContextMenuPanel::Root {
            PageBlockMenuSelection::Row(index)
        } else {
            target.submenu_selection(action)
        };
        let Some(menu) = self
            .page_block_context_menu
            .as_mut()
            .filter(|menu| menu.block_id == block_id)
        else {
            return PageBlockContextMenuTransition::default();
        };
        if menu.panel == panel && menu.selection == selection {
            return PageBlockContextMenuTransition::default();
        }
        PageBlockMenuMutation::changed(menu.set_panel(panel, selection)).into()
    }

    fn hover_page_block_submenu(
        &mut self,
        block_id: &str,
        panel: PageBlockContextMenuPanel,
        index: usize,
    ) -> PageBlockContextMenuTransition {
        let Some(menu) = self
            .page_block_context_menu
            .as_mut()
            .filter(|menu| menu.block_id == block_id && menu.panel == panel)
        else {
            return PageBlockContextMenuTransition::default();
        };
        let selection = PageBlockMenuSelection::Row(index);
        if menu.selection == selection {
            return PageBlockContextMenuTransition::default();
        }
        menu.selection = selection;
        PageBlockContextMenuTransition::notified()
    }
}

fn matching_target<'a>(
    target: Option<&'a PageBlockContextMenuTarget>,
    block_id: &str,
) -> Option<&'a PageBlockContextMenuTarget> {
    target.filter(|target| target.block.block_id == block_id)
}

impl PageBlockContextMenuTarget {
    fn submenu_command(
        &self,
        menu: &PageBlockContextMenuState,
        selected: usize,
        last_used_color: crate::ui::CardPageBlockColor,
    ) -> PageBlockContextMenuTransition {
        let block_id = &menu.block_id;
        let command = match menu.panel {
            PageBlockContextMenuPanel::Root => return PageBlockContextMenuTransition::default(),
            PageBlockContextMenuPanel::TurnInto if self.can_turn_into => {
                let Some(kind) = PAGE_BLOCK_TURN_INTO_ACTIONS
                    .get(selected)
                    .and_then(|conversion| self.turn_into_kind(conversion))
                else {
                    return PageBlockContextMenuTransition::default();
                };
                PageBlockContextEditCommand::TurnInto {
                    block_id: block_id.to_string(),
                    kind,
                }
            }
            PageBlockContextMenuPanel::Color if self.can_color => {
                let Some(color) = page_block_color_menu_row_color(selected, last_used_color) else {
                    return PageBlockContextMenuTransition::default();
                };
                PageBlockContextEditCommand::SetColor {
                    block_id: block_id.to_string(),
                    color,
                }
            }
            PageBlockContextMenuPanel::QuoteSize if self.targets_are_quotes => {
                let size = match selected {
                    0 => CardPageQuoteSize::Default,
                    1 => CardPageQuoteSize::Large,
                    _ => return PageBlockContextMenuTransition::default(),
                };
                PageBlockContextEditCommand::SetQuoteSize {
                    block_id: block_id.to_string(),
                    size,
                }
            }
            PageBlockContextMenuPanel::CodeLanguage if self.is_code => {
                let Some(language) = menu.language_at_filtered_row(selected) else {
                    return PageBlockContextMenuTransition::default();
                };
                PageBlockContextEditCommand::SetCodeLanguage {
                    block_id: block_id.to_string(),
                    language,
                }
            }
            PageBlockContextMenuPanel::TurnInto
            | PageBlockContextMenuPanel::Color
            | PageBlockContextMenuPanel::QuoteSize
            | PageBlockContextMenuPanel::CodeLanguage => {
                return PageBlockContextMenuTransition::default();
            }
        };
        PageBlockContextMenuTransition::command(PageBlockContextMenuCommand::Edit(command))
    }
}

impl PageBlockContextMenuState {
    fn language_at_filtered_row(&self, selected: usize) -> Option<CardPageCodeLanguage> {
        let catalog_index = *self.code_language_indices.get(selected)?;
        Some(
            CardPageCodeLanguage::try_from(PAGE_CODE_LANGUAGES[catalog_index].to_string())
                .expect("the fixed Code language catalog must contain non-empty labels"),
        )
    }
}
