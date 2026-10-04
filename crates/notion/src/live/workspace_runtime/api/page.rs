use crate::model::{CardPage, EditPageBlockTextRequest, PageMutationRequest, PageMutationResult};

use super::super::NotionWorkspaceRuntime;
use crate::live::{
    credentials::NotionDesktopSession, load_card_page_snapshot_with_session, NotionLiveError,
};

impl NotionWorkspaceRuntime {
    pub(super) fn load_card_page_preview_once(
        &self,
        session: &NotionDesktopSession,
        block_id: &str,
    ) -> Result<CardPage, NotionLiveError> {
        self.workspace_context
            .load_quick_find_preview(session, block_id)
    }

    pub(super) fn load_card_page_once(
        &self,
        session: &NotionDesktopSession,
        block_id: &str,
    ) -> Result<CardPage, NotionLiveError> {
        let mut mutator = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?;
        let snapshot = load_card_page_snapshot_with_session(session, block_id)?;
        mutator.register_page_snapshot(snapshot.mutation_state, snapshot.response);
        Ok(snapshot.page)
    }

    pub(super) fn apply_page_mutation_once(
        &self,
        session: &NotionDesktopSession,
        request: PageMutationRequest,
    ) -> Result<PageMutationResult, NotionLiveError> {
        self.workspace_context
            .invalidate_navigation_bootstrap(&request.page_block_id)?;
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .apply_page_mutation(session, &request)
    }

    pub(super) fn edit_page_block_text_once(
        &self,
        session: &NotionDesktopSession,
        request: EditPageBlockTextRequest,
    ) -> Result<PageMutationResult, NotionLiveError> {
        self.workspace_context
            .invalidate_navigation_bootstrap(&request.page_block_id)?;
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .apply_page_text_edit(session, &request)
    }
}
