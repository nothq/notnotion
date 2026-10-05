use std::collections::HashMap;

use gpui::Context;

use crate::model::{BoardItem, BoardSnapshot, CardPage, CardPageProperty, CardSummary};
use crate::ui::{Card, ColumnState, SurfaceState};

use super::page_document::PageDocumentAction;

pub(super) fn primary_page_creation(columns: &[ColumnState]) -> PageDocumentAction {
    columns
        .iter()
        .position(|column| !column.title.trim().is_empty())
        .map(PageDocumentAction::CreateInColumn)
        .unwrap_or_else(|| {
            PageDocumentAction::creation_error(
                "missing writable Notion column for new page creation".to_string(),
            )
        })
}

pub(super) struct PageCreationError(String);

impl PageCreationError {
    pub(super) fn new(message: String) -> Self {
        Self(message)
    }

    pub(super) fn apply(self, surface: &mut SurfaceState, cx: &mut Context<SurfaceState>) {
        let page_host = surface.presentation.page_host.clone();
        let editor = &mut surface.page_editor;
        let chrome = &mut surface.notion_chrome;
        let search = &mut surface.database_search;
        if let Some(page_host) = page_host {
            page_host
                .update(cx, |surface, cx| {
                    surface.dispatch_page_document_action(
                        PageDocumentAction::creation_error(self.0),
                        cx,
                    );
                })
                .expect("inline database page presentation requires its parent surface");
            return;
        }
        editor.reset_page_composer();
        chrome.toolbar_dialog = None;
        chrome.inline_toolbar_dialog = None;
        chrome.ai_autofill_dialog = None;
        search.close();
        chrome.notion_search_open = false;
        println!("notnotion: {}", self.0);
        cx.notify();
    }
}

pub(super) struct PageCreationState<'a> {
    columns: &'a mut Vec<ColumnState>,
    board: &'a mut BoardSnapshot,
    snapshot_pages: &'a mut Option<HashMap<String, CardPage>>,
}

impl<'a> PageCreationState<'a> {
    pub(super) fn new(
        columns: &'a mut Vec<ColumnState>,
        board: &'a mut BoardSnapshot,
        snapshot_pages: &'a mut Option<HashMap<String, CardPage>>,
    ) -> Self {
        Self {
            columns,
            board,
            snapshot_pages,
        }
    }

    pub(super) fn insert(
        &mut self,
        column_index: usize,
        block_id: String,
        column_title: String,
    ) -> Option<Card> {
        if !self.can_insert(column_index, &block_id) {
            return None;
        }
        let card = new_page_card(&block_id);
        self.columns[column_index].cards.push(card.clone());
        if let Some(column) = self.board.columns.get_mut(column_index) {
            column.cards.push(new_page_summary(&block_id));
        }
        self.board.items.push(BoardItem {
            block_id: block_id.clone(),
            title: String::new(),
            status: Some(column_title.clone()),
            icon: None,
            properties: Vec::new(),
        });
        if let Some(pages) = self.snapshot_pages {
            pages.insert(block_id.clone(), new_page(block_id, column_title));
        }
        Some(card)
    }

    fn can_insert(&self, column_index: usize, block_id: &str) -> bool {
        self.columns
            .get(column_index)
            .is_some_and(|column| !column.cards.iter().any(|card| card.block_id == block_id))
    }
}

fn new_page_card(block_id: &str) -> Card {
    Card {
        title: String::new(),
        block_id: block_id.to_string(),
        height: 40.0,
        has_content: false,
        icon: None,
        fill_override: None,
    }
}

fn new_page_summary(block_id: &str) -> CardSummary {
    CardSummary {
        block_id: block_id.to_string(),
        title: String::new(),
        height: 40.0,
        has_content: false,
        icon: None,
    }
}

fn new_page(block_id: String, column_title: String) -> CardPage {
    CardPage {
        block_id,
        title: String::new(),
        status: Some(column_title.clone()),
        properties: vec![CardPageProperty {
            property_id: String::new(),
            label: "Status".to_string(),
            property_type: "status".to_string(),
            value: column_title,
            status_options: Vec::new(),
        }],
        blocks: Vec::new(),
        discussions: Vec::new(),
        comments_writable: true,
        format: Default::default(),
    }
}
