use super::super::super::{
    CardPageBlock, CardPageBlockColor, CardPageBlockKind, CardPageQuoteSize,
};
use super::super::catalog::{
    PageBlockMenuAction, PageBlockMenuActionSpec, PageBlockTurnIntoSpec, PAGE_BLOCK_MENU_ACTIONS,
};
use super::super::search::page_block_menu_label_matches;
use super::{PageBlockMenuSelection, PageCodeLanguageSelection};
use crate::model::{CardPageCodeLanguage, CardPageCodeWrap};

#[derive(Clone)]
pub(in super::super) struct PageBlockContextMenuTarget {
    pub(in super::super) block: CardPageBlock,
    pub(in super::super) page_id: String,
    pub(in super::super) is_code: bool,
    pub(in super::super) code_wrap: Option<CardPageCodeWrap>,
    pub(in super::super) code_language: Option<CardPageCodeLanguage>,
    pub(in super::super) can_turn_into: bool,
    pub(in super::super) can_color: bool,
    pub(in super::super) current_color: Option<CardPageBlockColor>,
    pub(in super::super) targets_are_quotes: bool,
    pub(in super::super) current_quote_size: Option<CardPageQuoteSize>,
    pub(in super::super) can_copy_link: bool,
    pub(in super::super) can_duplicate: bool,
    pub(in super::super) can_delete: bool,
    pub(in super::super) writable_comment_target: Option<(String, String)>,
}

impl PageBlockContextMenuTarget {
    pub(in super::super) fn turn_into_kind(
        &self,
        conversion: &PageBlockTurnIntoSpec,
    ) -> Option<CardPageBlockKind> {
        conversion.kind
    }

    pub(in super::super) fn action_visible(&self, action: PageBlockMenuAction) -> bool {
        let kind = self.block.editable_content().map(|editable| editable.kind);
        let is_alias = self.block.alias_content().is_some();
        let is_generic_leaf = self.block.resource_content().is_some()
            || self.block.unsupported_leaf_content().is_some();
        let is_simple_table = self.block.simple_table_content().is_some();
        match action {
            PageBlockMenuAction::CopyLink | PageBlockMenuAction::Delete if is_simple_table => true,
            _ if is_simple_table => false,
            PageBlockMenuAction::CopyLink | PageBlockMenuAction::Delete if is_generic_leaf => true,
            _ if is_generic_leaf => false,
            PageBlockMenuAction::CodeLanguage
            | PageBlockMenuAction::CodeWrap
            | PageBlockMenuAction::CopyCode => self.is_code,
            PageBlockMenuAction::TurnInto
            | PageBlockMenuAction::SuggestEdits
            | PageBlockMenuAction::Skills
                if is_alias =>
            {
                false
            }
            PageBlockMenuAction::QuoteSize => self.targets_are_quotes,
            PageBlockMenuAction::Color if self.is_code => false,
            PageBlockMenuAction::EditIcon => kind.is_some_and(|kind| {
                matches!(
                    kind,
                    CardPageBlockKind::PageLink | CardPageBlockKind::Callout
                )
            }),
            PageBlockMenuAction::Comment => kind.is_some(),
            _ => true,
        }
    }

    pub(in super::super) fn action_enabled(&self, action: PageBlockMenuAction) -> bool {
        match action {
            PageBlockMenuAction::CodeLanguage
            | PageBlockMenuAction::CodeWrap
            | PageBlockMenuAction::CopyCode => self.is_code,
            PageBlockMenuAction::TurnInto => self.can_turn_into,
            PageBlockMenuAction::Color => self.can_color,
            PageBlockMenuAction::QuoteSize => self.targets_are_quotes,
            PageBlockMenuAction::EditIcon => {
                self.block.editable_content().is_some_and(|editable| {
                    matches!(
                        editable.kind,
                        CardPageBlockKind::PageLink | CardPageBlockKind::Callout
                    )
                })
            }
            PageBlockMenuAction::CopyLink => self.can_copy_link,
            PageBlockMenuAction::Duplicate => self.can_duplicate,
            PageBlockMenuAction::Delete => self.can_delete,
            PageBlockMenuAction::AskAi => self.block.editable_content().is_some(),
            PageBlockMenuAction::Comment => self.writable_comment_target.is_some(),
            PageBlockMenuAction::MoveTo
            | PageBlockMenuAction::SuggestEdits
            | PageBlockMenuAction::Present
            | PageBlockMenuAction::Skills => false,
        }
    }

    pub(in super::super) fn visible_actions(
        &self,
        query: &str,
    ) -> Vec<&'static PageBlockMenuActionSpec> {
        PAGE_BLOCK_MENU_ACTIONS
            .iter()
            .filter(|spec| {
                self.action_visible(spec.action) && page_block_menu_label_matches(spec.label, query)
            })
            .collect()
    }

    pub(in super::super) fn filtered_action_index(
        &self,
        query: &str,
        action: PageBlockMenuAction,
    ) -> Option<usize> {
        self.visible_actions(query)
            .iter()
            .position(|spec| spec.action == action)
    }

    pub(in super::super) fn root_action_at(
        &self,
        query: &str,
        selected: usize,
    ) -> Option<PageBlockMenuAction> {
        self.visible_actions(query)
            .get(selected)
            .map(|spec| spec.action)
    }

    pub(in super::super) fn root_selectable_indices(&self, query: &str) -> Vec<usize> {
        self.visible_actions(query)
            .iter()
            .enumerate()
            .filter_map(|(index, spec)| self.action_enabled(spec.action).then_some(index))
            .collect()
    }

    pub(in super::super) fn submenu_selection(
        &self,
        action: PageBlockMenuAction,
    ) -> PageBlockMenuSelection {
        match action {
            PageBlockMenuAction::QuoteSize => PageBlockMenuSelection::Row(usize::from(
                self.current_quote_size == Some(CardPageQuoteSize::Large),
            )),
            PageBlockMenuAction::CodeLanguage => {
                self.code_language
                    .as_ref()
                    .map_or(PageBlockMenuSelection::Hidden, |language| {
                        PageCodeLanguageSelection::from_language(language.as_str()).menu_selection()
                    })
            }
            PageBlockMenuAction::TurnInto | PageBlockMenuAction::Color => {
                PageBlockMenuSelection::Row(0)
            }
            PageBlockMenuAction::CodeWrap
            | PageBlockMenuAction::CopyCode
            | PageBlockMenuAction::EditIcon
            | PageBlockMenuAction::CopyLink
            | PageBlockMenuAction::Duplicate
            | PageBlockMenuAction::Delete
            | PageBlockMenuAction::AskAi
            | PageBlockMenuAction::MoveTo
            | PageBlockMenuAction::Comment
            | PageBlockMenuAction::SuggestEdits
            | PageBlockMenuAction::Present
            | PageBlockMenuAction::Skills => PageBlockMenuSelection::Hidden,
        }
    }
}
