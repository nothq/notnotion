use super::initialization::notion_sidebar_should_be_visible;
use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::{CardPage, CardPeekState, ColumnState, PageComposerState, SurfaceState};
use std::collections::HashMap;

impl SurfaceState {
    pub fn restore_notion_route(
        &mut self,
        route: &crate::model::NotionLaunchRoute,
    ) -> Result<(), String> {
        let board_state = route.board_url_state();
        self.reset_notion_route_state();
        if let Some(block_id) = board_state.selected_page_block_id {
            self.page_documents.restore_selected_route(
                &block_id,
                &self.columns,
                self.snapshot_pages.as_ref(),
                &self.page_editor,
            );
            if board_state.slash_command_open && self.page_documents.selected_page.is_some() {
                self.page_editor.restore_route_composer(block_id);
            }
        }
        Ok(())
    }

    fn reset_notion_route_state(&mut self) {
        self.reset_date_query_state_for_route();
        self.page_documents.reset_selected_page();
        self.page_editor.reset_for_route();

        self.comments.dismiss_notion_comments_panel_without_notify();
        self.database_search.close();
        self.notion_chrome
            .reset_for_route(notion_sidebar_should_be_visible(&self.board));
    }
}

impl PageDocuments {
    fn restore_selected_route(
        &mut self,
        block_id: &str,
        columns: &[ColumnState],
        snapshot_pages: Option<&HashMap<String, CardPage>>,
        editor: &PageEditorState,
    ) {
        self.selected_page = columns
            .iter()
            .flat_map(|column| column.cards.iter())
            .find(|card| card.block_id == block_id)
            .map(
                |card| match snapshot_pages.and_then(|pages| pages.get(block_id)) {
                    Some(page) => {
                        CardPeekState::Loaded(editor.loaded_card_page_with_disclosure(page.clone()))
                    }
                    None => CardPeekState::Loading {
                        block_id: block_id.to_string(),
                        title: card.title.clone(),
                    },
                },
            );
    }
}

impl PageEditorState {
    fn restore_route_composer(&mut self, block_id: String) {
        self.input.composer = PageComposerState {
            page_id: Some(block_id.clone()),
            active: true,
            slash_command_open: true,
            text: "/".to_string(),
            ..Default::default()
        };
        *self
            .input
            .resource_state()
            .composer_focus_request
            .borrow_mut() = Some(block_id);
    }
}
