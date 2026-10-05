use crate::model::BoardItem;
use crate::ui::surface::PageDocuments;
use crate::ui::{Card, CardPeekState, ColumnState, LoadedCardPage};

pub(super) fn card_for_board_item(columns: &[ColumnState], item: &BoardItem) -> Card {
    columns
        .iter()
        .flat_map(|column| column.cards.iter())
        .find(|card| card.block_id == item.block_id)
        .cloned()
        .unwrap_or_else(|| Card {
            title: item.title.clone(),
            block_id: item.block_id.clone(),
            height: 40.0,
            has_content: true,
            icon: item.icon.clone(),
            fill_override: None,
        })
}

impl PageDocuments {
    pub(in crate::ui::board_workspace) fn opening_switches_page_projection(
        &self,
        block_id: &str,
    ) -> bool {
        let switching_selected = self
            .selected_page
            .as_ref()
            .is_some_and(|selected| selected.block_id() != block_id);
        let switching_standalone = self.selected_page.is_none()
            && self
                .standalone
                .as_ref()
                .is_some_and(|page| page.data.page.block_id != block_id);
        switching_selected || switching_standalone
    }

    pub(in crate::ui::board_workspace) fn remember_selected_page_before_open(
        &mut self,
        block_id: &str,
    ) {
        if let Some(selected_page) = self.selected_page.clone() {
            if selected_page.block_id() != block_id {
                self.selected_page_history.push(selected_page);
            }
        }
    }

    pub(in crate::ui::board_workspace) fn set_selected_page_loading(
        &mut self,
        block_id: String,
        title: String,
    ) {
        self.selected_page = Some(CardPeekState::Loading { block_id, title });
    }

    pub(in crate::ui::board_workspace) fn selected_page_matches(&self, block_id: &str) -> bool {
        self.selected_page
            .as_ref()
            .is_some_and(|selected| selected.block_id() == block_id)
    }

    pub(in crate::ui::board_workspace) fn install_selected_page(&mut self, page: LoadedCardPage) {
        self.selected_page = Some(CardPeekState::Loaded(page));
    }

    pub(in crate::ui::board_workspace) fn mark_selected_page_error(
        &mut self,
        block_id: String,
        title: String,
    ) -> bool {
        if !self.selected_page_matches(&block_id) {
            return false;
        }
        self.selected_page = Some(CardPeekState::Error { block_id, title });
        true
    }

    pub(in crate::ui::board_workspace) fn restore_page_before_failed_open(
        &mut self,
        block_id: &str,
    ) -> bool {
        if !self.selected_page_matches(block_id) {
            return false;
        }
        self.selected_page = self.selected_page_history.pop();
        true
    }

    pub(in crate::ui::board_workspace) fn navigate_selected_page_back(&mut self) {
        self.selected_page = self.selected_page_history.pop();
    }

    pub(in crate::ui::board_workspace) fn close_selected_page(&mut self) {
        self.selected_page = None;
        self.selected_page_history.clear();
    }
}
