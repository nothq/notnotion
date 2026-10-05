use std::collections::HashSet;

use super::super::persistence::{page_block_creation, VerifiedPageTextBlockKind};
use super::super::support::page_block_subtree_end;
use super::super::{
    CardPage, CardPageBlock, CardPageBlockColor, CardPageBlockKind, CardPageQuoteSize,
};
use super::delete::page_block_can_delete;
use super::duplicate::page_alias_duplicate_source;
use crate::ui::surface::PageEditorState;

/// The selected root blocks resolved against one immutable page snapshot.
pub(in crate::ui::board_workspace::page::editor) struct PageBlockContextTargets<'a> {
    page: &'a CardPage,
    roots: Vec<usize>,
}

impl<'a> PageBlockContextTargets<'a> {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        page: &'a CardPage,
        roots: Vec<usize>,
    ) -> Self {
        Self { page, roots }
    }

    pub(in crate::ui::board_workspace::page::editor) fn can_color(&self) -> bool {
        self.roots
            .iter()
            .any(|index| page_block_supports_color(&self.page.blocks[*index]))
    }

    pub(in crate::ui::board_workspace::page::editor) fn current_color(
        &self,
    ) -> Option<CardPageBlockColor> {
        let mut colors = self
            .roots
            .iter()
            .filter(|index| page_block_supports_color(&self.page.blocks[**index]))
            .map(|index| self.page.blocks[*index].color);
        let first = colors.next()?;
        colors.all(|color| color == first).then_some(first)
    }

    pub(in crate::ui::board_workspace::page::editor) fn are_quotes(&self) -> bool {
        !self.roots.is_empty()
            && self.roots.iter().all(|index| {
                self.page.blocks[*index]
                    .editable_content()
                    .and_then(|editable| editable.quote_size())
                    .is_some()
            })
    }

    pub(in crate::ui::board_workspace::page::editor) fn current_quote_size(
        &self,
    ) -> Option<CardPageQuoteSize> {
        let mut sizes = self
            .roots
            .iter()
            .map(|index| self.page.blocks[*index].editable_content()?.quote_size());
        let first = sizes.next()??;
        sizes.all(|size| size == Some(first)).then_some(first)
    }

    pub(in crate::ui::board_workspace::page::editor) fn can_turn_into(&self) -> bool {
        !self.roots.is_empty()
            && self.roots.iter().all(|index| {
                self.page.blocks[*index]
                    .editable_content()
                    .and_then(|editable| VerifiedPageTextBlockKind::parse(editable.kind))
                    .is_some()
            })
    }

    pub(in crate::ui::board_workspace::page::editor) fn can_duplicate(&self) -> bool {
        page_alias_duplicate_source(self.page, &self.roots).is_some()
            || (!self.roots.is_empty()
                && self.roots.iter().all(|index| {
                    let end = page_block_subtree_end(&self.page.blocks, *index);
                    self.page.blocks[*index..end]
                        .iter()
                        .all(|block| page_block_creation(block).is_some())
                }))
    }

    pub(in crate::ui::board_workspace::page::editor) fn can_delete(&self) -> bool {
        !self.roots.is_empty()
            && self.roots.iter().all(|index| {
                let end = page_block_subtree_end(&self.page.blocks, *index);
                self.page.blocks[*index..end]
                    .iter()
                    .all(page_block_can_delete)
            })
    }

    pub(in crate::ui::board_workspace::page::editor) fn code_block(
        &self,
    ) -> Option<&crate::model::CardPageEditableBlock> {
        let [index] = self.roots.as_slice() else {
            return None;
        };
        self.page.blocks[*index]
            .editable_content()
            .filter(|editable| editable.kind == CardPageBlockKind::Code)
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn page_block_context_menu_root_indices(
        &self,
        page: &CardPage,
        block_id: &str,
    ) -> Vec<usize> {
        let selected_ids = &self.page_block_selection.block_ids;
        let target_ids = if selected_ids.iter().any(|selected| selected == block_id) {
            selected_ids
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>()
        } else {
            HashSet::from([block_id])
        };
        let mut roots = Vec::new();
        for (index, block) in page.blocks.iter().enumerate() {
            if !target_ids.contains(block.block_id.as_str())
                || roots
                    .iter()
                    .any(|root| index < page_block_subtree_end(&page.blocks, *root))
            {
                continue;
            }
            roots.push(index);
        }
        roots
    }

    pub(in crate::ui::board_workspace::page::editor) fn clear_page_block_context_menu_selection(
        &mut self,
    ) {
        self.page_block_context_menu = None;
        self.page_block_selection.block_ids.clear();
    }
}

pub(super) fn page_block_supports_color(block: &CardPageBlock) -> bool {
    block.alias_content().is_some()
        || block
            .editable_content()
            .is_some_and(|editable| crate::ui::page_block_kind_supports_color(editable.kind))
}
