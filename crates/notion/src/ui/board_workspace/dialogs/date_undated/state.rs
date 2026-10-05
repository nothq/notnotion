use super::Arc;
use crate::ui::surface::NotionChromeState;

impl NotionChromeState {
    pub(super) fn close_competing_date_undated_dialogs(&mut self) {
        self.toolbar_dialog = None;
        self.inline_toolbar_dialog = None;
        self.inline_database_view_menu = None;
        self.ai_autofill_dialog = None;
        self.notion_search_open = false;
        self.notion_ai_open = false;
    }

    pub(super) fn clear_date_undated_dialog_for_source_session(
        &mut self,
        session: &Arc<()>,
    ) -> bool {
        let matching = self
            .date_undated_dialog
            .as_ref()
            .is_some_and(|state| state.source.matches_session(session));
        if matching {
            self.date_undated_dialog = None;
        }
        matching
    }
}
