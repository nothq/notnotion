use std::collections::HashSet;
use std::sync::Arc;

use super::super::selection::DirectedTextRange;
use super::super::{
    PageBlockContextMenuPresentation, PageBlockContextMenuState, PageSlashMenuState,
};
use crate::ui::surface::PageEditorState;
use crate::ui::LoadedCardPageData;
use crate::ui::PageTextSelection;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageRenderInteraction {
    pub(super) active_block: Option<String>,
    pub(super) hovered_block: Option<String>,
    pub(super) composing_blocks: Arc<[String]>,
    pub(super) selected_blocks: Arc<[String]>,
    pub(super) text_selection: Option<PageTextSelection>,
    pub(super) context_menu: Option<PageBlockContextMenuState>,
    pub(super) page_link_icon_picker: Option<(String, String)>,
    pub(super) slash_menu: Option<PageSlashMenuState>,
    pub(super) mention_menu_block: Option<String>,
    pub(super) expanded_blocks: Arc<HashSet<String>>,
    pub(super) active_drag: bool,
}

impl PageRenderInteraction {
    pub(super) fn capture(editor: &PageEditorState, page_id: &str, active_drag: bool) -> Self {
        let page_link_icon_picker = editor
            .page_link_icons
            .picker()
            .map(|picker| (picker.page_id.clone(), picker.block_id.clone()));
        Self {
            active_block: editor.active_page_block.clone(),
            hovered_block: editor.hovered_page_block.clone(),
            composing_blocks: editor
                .page_block_compositions
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .into(),
            selected_blocks: editor.page_block_selection.block_ids.clone().into(),
            text_selection: editor.page_text_selection.clone(),
            context_menu: editor.page_block_context_menu.clone(),
            page_link_icon_picker,
            slash_menu: editor.page_slash_menu.clone(),
            mention_menu_block: editor.mention.menu().map(|menu| menu.block_id.clone()),
            expanded_blocks: editor
                .page_toggle_disclosure
                .expanded_block_ids(page_id)
                .cloned()
                .unwrap_or_default()
                .into(),
            active_drag,
        }
    }

    pub(super) fn block_selected(&self, block_id: &str) -> bool {
        self.selected_blocks
            .iter()
            .any(|selected| selected == block_id)
    }

    pub(super) fn block_selected_without_drag(&self, block_id: &str) -> bool {
        self.block_selected(block_id) && !self.active_drag
    }

    pub(super) fn block_is_composing(&self, block_id: &str) -> bool {
        self.composing_blocks
            .iter()
            .any(|composing| composing == block_id)
    }

    pub(super) fn block_menu_is_open(&self, block_id: &str) -> bool {
        self.context_menu
            .as_ref()
            .is_some_and(|menu| menu.block_id == block_id)
    }

    pub(super) fn block_actions_menu_is_open(&self, block_id: &str) -> bool {
        self.context_menu.as_ref().is_some_and(|menu| {
            menu.block_id == block_id
                && menu.presentation == PageBlockContextMenuPresentation::BlockActions
        })
    }

    pub(super) fn page_link_icon_picker_is_open(&self, page_id: &str, block_id: &str) -> bool {
        self.page_link_icon_picker
            .as_ref()
            .is_some_and(|(page, block)| page == page_id && block == block_id)
    }

    pub(in crate::ui::board_workspace::page::editor) fn active_block_is(
        &self,
        block_id: &str,
    ) -> bool {
        self.active_block.as_deref() == Some(block_id)
    }

    pub(in crate::ui::board_workspace::page::editor) fn slash_menu_is_open(
        &self,
        block_id: &str,
    ) -> bool {
        self.slash_menu
            .as_ref()
            .is_some_and(|menu| menu.block_id == block_id)
    }

    pub(in crate::ui::board_workspace::page::editor) fn mention_menu_is_open(
        &self,
        block_id: &str,
    ) -> bool {
        self.mention_menu_block.as_deref() == Some(block_id)
    }

    pub(in crate::ui::board_workspace::page::editor) fn input_selection(
        &self,
        data: &LoadedCardPageData,
        block_id: &str,
    ) -> Option<DirectedTextRange> {
        let selection = self.text_selection.as_ref()?;
        let candidate = *data.editable_block_indices.get(block_id)?;
        super::super::selection::page_text_selection_range(data, selection, candidate)
    }
}
