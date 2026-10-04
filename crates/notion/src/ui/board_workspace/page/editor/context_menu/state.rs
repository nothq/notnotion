use std::sync::Arc;

use gpui::{App, Entity, FocusHandle};
use gpui_components::text_input::TextInput;

use super::super::PAGE_CODE_LANGUAGES;
use super::catalog::{
    PageBlockMenuAction, PAGE_BLOCK_COLOR_MENU_ROW_COUNT, PAGE_BLOCK_TURN_INTO_ACTIONS,
};

mod target;
pub(super) use target::PageBlockContextMenuTarget;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum PageBlockContextMenuPanel {
    #[default]
    Root,
    TurnInto,
    Color,
    QuoteSize,
    CodeLanguage,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum PageBlockContextMenuPresentation {
    #[default]
    BlockActions,
    CodeLanguagePicker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SupportedPageBlockAction {
    CopyLink,
    Duplicate,
    Delete,
    AskAi,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum PageBlockMenuSelection {
    #[default]
    Hidden,
    Row(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageBlockMenuFocusTarget {
    RootSearch,
    CodeLanguageSearch,
}

impl PageBlockMenuFocusTarget {
    pub(super) const fn for_panel(panel: PageBlockContextMenuPanel) -> Self {
        match panel {
            PageBlockContextMenuPanel::CodeLanguage => Self::CodeLanguageSearch,
            PageBlockContextMenuPanel::Root
            | PageBlockContextMenuPanel::TurnInto
            | PageBlockContextMenuPanel::Color
            | PageBlockContextMenuPanel::QuoteSize => Self::RootSearch,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PageCodeLanguageSelection {
    Known(usize),
    Unlisted,
}

impl PageCodeLanguageSelection {
    pub(super) fn from_language(language: &str) -> Self {
        match PAGE_CODE_LANGUAGES
            .iter()
            .position(|candidate| *candidate == language)
        {
            Some(index) => Self::Known(index),
            None => Self::Unlisted,
        }
    }

    pub(super) const fn menu_selection(self) -> PageBlockMenuSelection {
        match self {
            Self::Known(index) => PageBlockMenuSelection::Row(index),
            Self::Unlisted => PageBlockMenuSelection::Hidden,
        }
    }
}

impl PageBlockMenuSelection {
    pub(super) const fn index(self) -> Option<usize> {
        match self {
            Self::Hidden => None,
            Self::Row(index) => Some(index),
        }
    }

    pub(super) const fn is_row(self, index: usize) -> bool {
        matches!(self, Self::Row(selected) if selected == index)
    }
}

#[derive(Clone)]
pub(crate) struct PageBlockContextMenuState {
    pub(crate) block_id: String,
    pub(crate) presentation: PageBlockContextMenuPresentation,
    pub(crate) panel: PageBlockContextMenuPanel,
    pub(crate) query: String,
    pub(super) selection: PageBlockMenuSelection,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) search_input: Entity<TextInput>,
    pub(crate) code_language_query: String,
    pub(crate) code_language_search_input: Entity<TextInput>,
    pub(crate) code_language_list_state: gpui::ListState,
    pub(crate) code_language_indices: Arc<[usize]>,
    pub(super) focus_target: PageBlockMenuFocusTarget,
}

impl PageBlockContextMenuState {
    pub(super) fn matches(
        &self,
        block_id: &str,
        presentation: PageBlockContextMenuPresentation,
    ) -> bool {
        self.block_id == block_id && self.presentation == presentation
    }

    pub(super) fn active_input(&self) -> Entity<TextInput> {
        match self.focus_target {
            PageBlockMenuFocusTarget::RootSearch => self.search_input.clone(),
            PageBlockMenuFocusTarget::CodeLanguageSearch => self.code_language_search_input.clone(),
        }
    }

    pub(super) fn set_panel(
        &mut self,
        panel: PageBlockContextMenuPanel,
        selection: PageBlockMenuSelection,
    ) -> Option<Entity<TextInput>> {
        let focus_target = PageBlockMenuFocusTarget::for_panel(panel);
        let focus_changed = self.focus_target != focus_target;
        self.panel = panel;
        self.selection = selection;
        self.focus_target = focus_target;
        focus_changed.then(|| self.active_input())
    }

    pub(super) fn request_active_input_focus(&self, cx: &mut App) {
        self.active_input()
            .update(cx, |input, cx| input.request_focus(cx));
    }

    pub(super) fn set_root_query(
        &mut self,
        query: String,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockMenuMutation {
        let selection = if query.is_empty() {
            PageBlockMenuSelection::Hidden
        } else {
            target
                .filter(|target| target.block.block_id == self.block_id)
                .and_then(|target| target.root_selectable_indices(&query).first().copied())
                .map_or(PageBlockMenuSelection::Hidden, PageBlockMenuSelection::Row)
        };
        if self.query == query
            && self.panel == PageBlockContextMenuPanel::Root
            && self.selection == selection
        {
            return PageBlockMenuMutation::default();
        }
        self.query = query;
        PageBlockMenuMutation::changed(self.set_panel(PageBlockContextMenuPanel::Root, selection))
    }

    pub(super) fn set_code_language_query(&mut self, query: String) -> PageBlockMenuMutation {
        if self.panel != PageBlockContextMenuPanel::CodeLanguage
            || self.code_language_query == query
        {
            return PageBlockMenuMutation::default();
        }
        let indices = super::search::page_code_language_indices(&query);
        self.code_language_list_state.reset(indices.len());
        self.code_language_query = query;
        self.code_language_indices = indices;
        self.selection = if self.code_language_indices.is_empty() {
            PageBlockMenuSelection::Hidden
        } else {
            PageBlockMenuSelection::Row(0)
        };
        PageBlockMenuMutation::notified()
    }

    pub(super) fn move_selection(
        &mut self,
        delta: isize,
        target: Option<&PageBlockContextMenuTarget>,
    ) -> PageBlockMenuMutation {
        let Some(target) = target.filter(|target| target.block.block_id == self.block_id) else {
            return PageBlockMenuMutation::default();
        };
        let selectable = self.selectable_indices(target);
        if selectable.is_empty() {
            return PageBlockMenuMutation::default();
        }
        let next = page_block_menu_next_selection(&selectable, self.selection.index(), delta);
        self.selection = PageBlockMenuSelection::Row(next);
        if self.panel == PageBlockContextMenuPanel::CodeLanguage {
            self.code_language_list_state.scroll_to_reveal_item(next);
        }
        PageBlockMenuMutation::notified()
    }

    pub(super) fn selectable_indices(&self, target: &PageBlockContextMenuTarget) -> Vec<usize> {
        if target.block.block_id != self.block_id {
            return Vec::new();
        }
        match self.panel {
            PageBlockContextMenuPanel::Root => target.root_selectable_indices(&self.query),
            PageBlockContextMenuPanel::TurnInto if target.can_turn_into => {
                PAGE_BLOCK_TURN_INTO_ACTIONS
                    .iter()
                    .enumerate()
                    .filter_map(|(index, conversion)| {
                        target.turn_into_kind(conversion).map(|_| index)
                    })
                    .collect()
            }
            PageBlockContextMenuPanel::Color if target.can_color => {
                (0..PAGE_BLOCK_COLOR_MENU_ROW_COUNT).collect()
            }
            PageBlockContextMenuPanel::QuoteSize if target.targets_are_quotes => vec![0, 1],
            PageBlockContextMenuPanel::CodeLanguage if target.is_code => {
                (0..self.code_language_indices.len()).collect()
            }
            PageBlockContextMenuPanel::TurnInto
            | PageBlockContextMenuPanel::Color
            | PageBlockContextMenuPanel::QuoteSize
            | PageBlockContextMenuPanel::CodeLanguage => Vec::new(),
        }
    }

    pub(super) fn selected_root_action(
        &self,
        target: &PageBlockContextMenuTarget,
    ) -> Option<PageBlockMenuAction> {
        if self.panel != PageBlockContextMenuPanel::Root || target.block.block_id != self.block_id {
            return None;
        }
        target.root_action_at(&self.query, self.selection.index()?)
    }

    pub(super) fn root_selection(
        &self,
        target: &PageBlockContextMenuTarget,
        action_count: usize,
    ) -> PageBlockMenuSelection {
        match page_block_menu_panel_action(self.panel) {
            None => page_block_menu_visible_selection(self.selection, action_count),
            Some(owner) => target
                .filtered_action_index(&self.query, owner)
                .map_or(PageBlockMenuSelection::Hidden, PageBlockMenuSelection::Row),
        }
    }
}

#[derive(Default)]
pub(super) struct PageBlockMenuMutation {
    pub(super) notify: bool,
    pub(super) focus: Option<Entity<TextInput>>,
}

impl PageBlockMenuMutation {
    pub(super) fn notified() -> Self {
        Self {
            notify: true,
            focus: None,
        }
    }

    pub(super) fn changed(focus: Option<Entity<TextInput>>) -> Self {
        Self {
            notify: true,
            focus,
        }
    }
}

fn page_block_menu_next_selection(
    selectable: &[usize],
    selected: Option<usize>,
    delta: isize,
) -> usize {
    let Some(selected) = selected else {
        return if delta < 0 {
            *selectable
                .last()
                .expect("selectable menu rows are non-empty")
        } else {
            selectable[0]
        };
    };
    let position = selectable.iter().position(|index| *index == selected);
    match (position, delta < 0) {
        (Some(position), true) => selectable[(position + selectable.len() - 1) % selectable.len()],
        (Some(position), false) => selectable[(position + 1) % selectable.len()],
        (None, true) => *selectable
            .last()
            .expect("selectable menu rows are non-empty"),
        (None, false) => selectable[0],
    }
}

pub(super) const fn page_block_menu_action_panel(
    action: PageBlockMenuAction,
) -> PageBlockContextMenuPanel {
    match action {
        PageBlockMenuAction::CodeLanguage => PageBlockContextMenuPanel::CodeLanguage,
        PageBlockMenuAction::TurnInto => PageBlockContextMenuPanel::TurnInto,
        PageBlockMenuAction::Color => PageBlockContextMenuPanel::Color,
        PageBlockMenuAction::QuoteSize => PageBlockContextMenuPanel::QuoteSize,
        _ => PageBlockContextMenuPanel::Root,
    }
}

pub(super) fn page_block_menu_panel_action(
    panel: PageBlockContextMenuPanel,
) -> Option<PageBlockMenuAction> {
    match panel {
        PageBlockContextMenuPanel::Root => None,
        PageBlockContextMenuPanel::TurnInto => Some(PageBlockMenuAction::TurnInto),
        PageBlockContextMenuPanel::Color => Some(PageBlockMenuAction::Color),
        PageBlockContextMenuPanel::QuoteSize => Some(PageBlockMenuAction::QuoteSize),
        PageBlockContextMenuPanel::CodeLanguage => Some(PageBlockMenuAction::CodeLanguage),
    }
}

fn page_block_menu_visible_selection(
    selection: PageBlockMenuSelection,
    action_count: usize,
) -> PageBlockMenuSelection {
    match selection {
        PageBlockMenuSelection::Hidden => PageBlockMenuSelection::Hidden,
        PageBlockMenuSelection::Row(_) if action_count == 0 => PageBlockMenuSelection::Hidden,
        PageBlockMenuSelection::Row(selected) => {
            PageBlockMenuSelection::Row(selected.min(action_count - 1))
        }
    }
}
