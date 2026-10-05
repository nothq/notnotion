use std::sync::Arc;

use crate::model::{MoveCardRequest, NotionLaunchRoute, NotionWorkspaceLoad};

use super::super::{NotionWorkspaceRuntime, NotionWorkspaceRuntimeInput};
use crate::live::{
    credentials::NotionDesktopSession, load_board_snapshot_for_navigation, NotionLiveError,
};

impl NotionWorkspaceRuntime {
    pub(super) fn cache_favorited_once(
        &self,
        _session: &NotionDesktopSession,
        is_favorited: bool,
    ) -> Result<Option<bool>, NotionLiveError> {
        let mutation_context = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .favorite_context();
        self.workspace_context
            .cache_page_favorited(is_favorited, mutation_context)
            .map(Some)
            .map_err(NotionLiveError::Fatal)
    }

    pub(super) fn flush_favorite_mutations_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<Vec<String>, NotionLiveError> {
        self.workspace_context.flush_favorite_mutations(session)
    }

    pub(super) fn synchronize_workspace_snapshot_once(
        &self,
        _session: &NotionDesktopSession,
        snapshot: &mut crate::model::BoardSnapshot,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context
            .synchronize_snapshot(snapshot)
            .map_err(NotionLiveError::Fatal)
    }

    pub(super) fn prefetch_notion_workspace_once(
        &self,
        session: &NotionDesktopSession,
        board_url: &str,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context
            .prefetch_notion_workspace_with_session(session, board_url)
    }

    pub(super) fn set_favorited_once(
        &self,
        session: &NotionDesktopSession,
        is_favorited: bool,
    ) -> Result<(), NotionLiveError> {
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .set_favorited_with_session(session, is_favorited)
            .map_err(|error| match error {
                NotionLiveError::Fatal(message) => NotionLiveError::Fatal(format!(
                    "failed to update notion favorite state: {message}"
                )),
                error => error,
            })
    }

    pub(super) fn create_page_in_column_once(
        &self,
        session: &NotionDesktopSession,
        target_column_title: &str,
    ) -> Result<String, NotionLiveError> {
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .create_page_in_column_with_session(session, target_column_title)
    }

    pub(super) fn move_card_once(
        &self,
        session: &NotionDesktopSession,
        request: MoveCardRequest,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .move_card_with_session(
                session,
                crate::live::MoveCardMutationRequest {
                    block_id: &request.block_id,
                    source_column_title: &request.source_column_title,
                    target_column_title: &request.target_column_title,
                    before_block_id: request.before_block_id.as_deref(),
                    after_block_id: request.after_block_id.as_deref(),
                },
            )
    }

    pub(super) fn load_notion_workspace_once(
        &self,
        session: &NotionDesktopSession,
        board_url: &str,
    ) -> Result<NotionWorkspaceLoad, NotionLiveError> {
        let loaded =
            load_board_snapshot_for_navigation(session, board_url, &self.workspace_context)?;
        let route = NotionLaunchRoute::board(board_url.parse()?);
        let next_mutator = loaded.mutator.ok_or_else(|| {
            "live Notion navigation did not provide its required mutation capability".to_string()
        })?;
        let next_workspace_context = loaded.workspace_context.ok_or_else(|| {
            "live Notion navigation did not provide its required workspace context".to_string()
        })?;
        if let Some(snapshot_cache) = self.snapshot_cache.as_ref() {
            snapshot_cache.persist_workspace(&route, &loaded.snapshot);
        }
        Ok(NotionWorkspaceLoad {
            workspace: loaded.snapshot,
            workspace_api: Arc::new(Self::new(NotionWorkspaceRuntimeInput {
                active_user_id: self.active_user_id.clone(),
                rebootstrap_api: self.rebootstrap_api.clone(),
                mutator: next_mutator,
                workspace_context: next_workspace_context,
                collection_query_state: loaded.collection_query_state,
                database_query_state: loaded.database_query_state,
                snapshot_cache: self.snapshot_cache.clone(),
                quick_find_runtime_generation: Some(self.quick_find_runtime_generation.clone()),
                code_settings: self.code_settings.clone(),
            })),
            code_settings: self.code_settings.clone(),
        })
    }
}
