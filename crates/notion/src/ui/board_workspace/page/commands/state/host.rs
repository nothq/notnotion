use super::super::super::editor::PageEditSession;
use super::PageComposerAction;
use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::{CardPeekState, Context, KeyDownEvent, PageComposerState, SurfaceState};

impl SurfaceState {
    pub(crate) fn handle_selected_page_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.keystroke.key.as_str() != "enter"
            || self.notion_chrome.ai_autofill_dialog.is_some()
            || self.notion_chrome.toolbar_dialog.is_some()
            || self.database_search.open
            || !self
                .page_editor
                .activate_composer_from_keyboard(&self.page_documents)
        {
            return false;
        }
        cx.notify();
        true
    }

    pub(in crate::ui::board_workspace::page) fn dispatch_page_composer_action(
        &mut self,
        action: PageComposerAction,
        cx: &mut Context<Self>,
    ) {
        let settings = self.page_code_settings.current();
        let transition = {
            let mut edit = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            let submitted = match action {
                PageComposerAction::Change { page_id, snapshot } => {
                    edit.apply_page_composer_text_change(&page_id, snapshot);
                    None
                }
                PageComposerAction::Submit { page_id, snapshot } => {
                    edit.apply_page_composer_text_change(&page_id, snapshot.clone());
                    Some((page_id, snapshot))
                }
                PageComposerAction::Select(target) => {
                    edit.select_page_command(target, settings.clone());
                    None
                }
            };
            edit.finish(submitted)
        };
        let Some((page_id, snapshot)) = self.apply_page_edit_transition(transition, cx) else {
            return;
        };
        let transition = {
            let mut edit = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            let block_id =
                edit.finish_page_composer_submission(&page_id, snapshot.cursor, settings);
            edit.finish(block_id)
        };
        if let Some(block_id) = self.apply_page_edit_transition(transition, cx) {
            self.submit_page_block(&block_id, snapshot, cx);
        }
    }
}

impl PageEditorState {
    fn activate_composer_from_keyboard(&mut self, documents: &PageDocuments) -> bool {
        if self.active_page_block.is_some() {
            return false;
        }
        let page_id = match documents.selected_page.as_ref() {
            Some(CardPeekState::Loaded(page)) => Some(page.data.page.block_id.clone()),
            _ => documents
                .standalone
                .as_ref()
                .map(|page| page.data.page.block_id.clone()),
        };
        let Some(page_id) = page_id else {
            return false;
        };
        if self.input.composer.active
            && self.input.composer.page_id.as_deref() == Some(page_id.as_str())
        {
            return false;
        }
        self.input.composer = PageComposerState {
            page_id: Some(page_id.clone()),
            active: true,
            ..Default::default()
        };
        *self
            .input
            .resource_state()
            .composer_focus_request
            .borrow_mut() = Some(page_id);
        true
    }
}

impl crate::ui::BoardSnapshot {
    pub(in crate::ui::board_workspace::page) fn mark_page_has_content(
        &mut self,
        page_id: &str,
        columns: &mut [crate::ui::ColumnState],
    ) {
        if let Some(card) = columns
            .iter_mut()
            .flat_map(|column| column.cards.iter_mut())
            .find(|card| card.block_id == page_id)
        {
            card.has_content = true;
        }
        if let Some(card) = self
            .columns
            .iter_mut()
            .flat_map(|column| column.cards.iter_mut())
            .find(|card| card.block_id == page_id)
        {
            card.has_content = true;
        }
    }
}
