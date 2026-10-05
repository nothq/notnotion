use std::sync::Arc;

use super::{NotionWorkspaceRuntime, NotionWorkspaceRuntimeInput};
use crate::live::{
    board::{CalendarDateMutation, CalendarPageCreation, LiveCollectionQueryState},
    credentials::NotionDesktopSession,
    NotionLiveError,
};
use crate::model::{
    CreateCalendarPageRequest, NotionBoardUrl, NotionCollectionViewId, NotionLaunchRoute,
    NotionPreviousStateDisposition, NotionWorkspaceBootstrap,
};

impl NotionWorkspaceRuntime {
    pub(super) fn load_inline_database_from_cached_bootstrap(
        &self,
        session: &NotionDesktopSession,
        collection_view_block_id: &str,
        provider_view_id: Option<&NotionCollectionViewId>,
    ) -> Result<NotionWorkspaceBootstrap, NotionLiveError> {
        let board_url: NotionBoardUrl = self
            .workspace_context
            .child_board_url(collection_view_block_id)?
            .parse()?;
        let board_url = match provider_view_id {
            Some(provider_view_id) => board_url.with_collection_view_id(provider_view_id),
            None => board_url,
        };
        let cached_parent_bootstrap = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .cached_page_bootstrap_containing_block(collection_view_block_id)?;
        let loaded = super::super::board::load_inline_database_snapshot_from_bootstrap(
            session,
            board_url.as_str(),
            cached_parent_bootstrap,
            &self.workspace_context,
        )?;
        let next_mutator = loaded.mutator.ok_or_else(|| {
            "live Notion inline database did not provide its required mutation capability"
                .to_string()
        })?;
        let next_workspace_context = loaded.workspace_context.ok_or_else(|| {
            "live Notion inline database did not provide its required workspace context".to_string()
        })?;
        let next_collection_query_state = loaded.collection_query_state;
        let next_database_query_state = loaded.database_query_state;
        Ok(NotionWorkspaceBootstrap {
            route: NotionLaunchRoute::board(board_url),
            workspace: loaded.snapshot,
            workspace_api: Arc::new(Self::new(NotionWorkspaceRuntimeInput {
                active_user_id: self.active_user_id.clone(),
                rebootstrap_api: self.rebootstrap_api.clone(),
                mutator: next_mutator,
                workspace_context: next_workspace_context,
                collection_query_state: next_collection_query_state,
                database_query_state: next_database_query_state,
                snapshot_cache: self.snapshot_cache.clone(),
                quick_find_runtime_generation: Some(self.quick_find_runtime_generation.clone()),
                code_settings: self.code_settings.clone(),
            })),
            code_settings: self.code_settings.clone(),
            previous_state_disposition: NotionPreviousStateDisposition::PreserveCompatible,
        })
    }

    pub(super) fn mutate_calendar_date(
        &self,
        session: &NotionDesktopSession,
        prepare_mutation: impl FnOnce(&LiveCollectionQueryState) -> Result<CalendarDateMutation, String>,
    ) -> Result<(), NotionLiveError> {
        let state = self.collection_query_state.as_ref().ok_or_else(|| {
            "Notion Calendar date mutation requires an active date view".to_string()
        })?;
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        let mutator = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?;
        let mutation = {
            let state = state
                .lock()
                .map_err(|_| "Notion collection query state lock is poisoned".to_string())?;
            prepare_mutation(&state)?
        };
        mutator.set_calendar_page_date(session, &mutation)?;
        drop(mutator);
        if let Err(error) = self
            .workspace_context
            .invalidate_current_navigation_bootstrap()
        {
            println!(
                "notnotion navigation cache invalidation failed after remote date mutation: {error}"
            );
        }
        let cache_commit = state
            .lock()
            .map_err(|_| "Notion collection query state lock is poisoned".to_string())
            .and_then(|mut state| state.commit_calendar_date_mutation(&mutation));
        if let Err(error) = cache_commit {
            println!(
                "notnotion retained date-query cache update failed after remote mutation: {error}"
            );
        }
        Ok(())
    }

    pub(super) fn create_calendar_page_atomic(
        &self,
        session: &NotionDesktopSession,
        request: &CreateCalendarPageRequest,
    ) -> Result<String, NotionLiveError> {
        let state = self.collection_query_state.as_ref().ok_or_else(|| {
            "Notion Calendar page creation requires an active Calendar view".to_string()
        })?;
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        let mutator = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?;
        let creation: CalendarPageCreation = state
            .lock()
            .map_err(|_| "Notion collection query state lock is poisoned".to_string())?
            .prepare_calendar_page_creation(request)?;
        let block_id = mutator.create_calendar_page(session, &creation)?;
        drop(mutator);
        if let Err(error) = self
            .workspace_context
            .invalidate_current_navigation_bootstrap()
        {
            println!(
                "notnotion navigation cache invalidation failed after Calendar page creation: {error}"
            );
        }
        Ok(block_id)
    }
}
