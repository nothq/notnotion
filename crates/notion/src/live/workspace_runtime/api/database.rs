use std::sync::Arc;

use crate::model::{
    DatabaseViewControlMutationRequest, DatabaseViewFilterMutationRequest,
    DatabaseViewFilterQueryRequest, DatabaseViewFilterSaveRequest, LoadDatabaseFilterUsersResult,
    NotionWorkspaceLoad, SearchDatabaseFilterRelationPagesRequest,
    SearchDatabaseFilterRelationPagesResult, SetDatabaseStatusPropertyRequest,
};

use super::super::{NotionWorkspaceRuntime, NotionWorkspaceRuntimeInput};
use crate::live::{
    credentials::NotionDesktopSession, load_database_filter_snapshot_for_query,
    load_database_filter_users, search_database_filter_relation_pages, DatabaseFilterQueryInput,
    NotionLiveError,
};

impl NotionWorkspaceRuntime {
    pub(super) fn mutate_database_view_control_once(
        &self,
        session: &NotionDesktopSession,
        request: DatabaseViewControlMutationRequest,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        let mut mutator = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?;
        mutator.mutate_database_view_control(session, request)?;
        drop(mutator);
        if let Err(error) = self
            .workspace_context
            .invalidate_current_navigation_bootstrap()
        {
            println!(
                "notnotion navigation cache invalidation failed after database view control mutation: {error}"
            );
        }
        Ok(())
    }

    pub(super) fn set_database_status_property_once(
        &self,
        session: &NotionDesktopSession,
        request: SetDatabaseStatusPropertyRequest,
    ) -> Result<(), NotionLiveError> {
        let page_id = request.page_id().to_string();
        self.workspace_context
            .invalidate_navigation_bootstrap(&page_id)?;
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .set_database_status_property(session, request)?;
        if let Err(error) = self
            .workspace_context
            .invalidate_navigation_bootstrap(&page_id)
        {
            println!(
                "notnotion navigation cache invalidation failed after status mutation: {error}"
            );
        }
        Ok(())
    }

    pub(super) fn mutate_database_view_filter_once(
        &self,
        session: &NotionDesktopSession,
        request: DatabaseViewFilterMutationRequest,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        let mut mutator = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?;
        mutator.mutate_database_view_filter(session, request)?;
        drop(mutator);
        if let Err(error) = self
            .workspace_context
            .invalidate_current_navigation_bootstrap()
        {
            println!(
                "notnotion navigation cache invalidation failed after database filter mutation: {error}"
            );
        }
        Ok(())
    }

    pub(super) fn save_database_view_filter_state_once(
        &self,
        session: &NotionDesktopSession,
        request: DatabaseViewFilterSaveRequest,
    ) -> Result<(), NotionLiveError> {
        self.workspace_context
            .invalidate_current_navigation_bootstrap()?;
        let mut mutator = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?;
        mutator.save_database_view_filter_state(session, request)?;
        drop(mutator);
        if let Err(error) = self
            .workspace_context
            .invalidate_current_navigation_bootstrap()
        {
            println!(
                "notnotion navigation cache invalidation failed after database filter save: {error}"
            );
        }
        Ok(())
    }

    pub(super) fn query_database_view_filter_once(
        &self,
        session: &NotionDesktopSession,
        request: DatabaseViewFilterQueryRequest,
    ) -> Result<NotionWorkspaceLoad, NotionLiveError> {
        let (expected_view_id, filter_state) = request.into_parts();
        let query_state = self.database_query_state.clone().ok_or_else(|| {
            "Notion database filter querying requires a compiled collection query".to_string()
        })?;
        let persisted_filter_state = self.persisted_filter_state(&expected_view_id)?;
        let mut loaded = load_database_filter_snapshot_for_query(
            session,
            &self.workspace_context,
            DatabaseFilterQueryInput {
                query_state,
                expected_view_id: expected_view_id.clone(),
                filter_state,
                persisted_filter_state,
            },
        )?;
        let latest_filter_state = self.persisted_filter_state(&expected_view_id)?;
        let mut next_mutator = loaded.mutator.take().ok_or_else(|| {
            "live Notion database filter query did not provide its required mutation capability"
                .to_string()
        })?;
        next_mutator.set_database_filter_state_for_view(&expected_view_id, latest_filter_state)?;
        let next_workspace_context = loaded.workspace_context.ok_or_else(|| {
            "live Notion database filter query did not provide its required workspace context"
                .to_string()
        })?;
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

    fn persisted_filter_state(
        &self,
        view_id: &crate::model::NotionCollectionViewId,
    ) -> Result<crate::model::DatabaseViewFilterState, String> {
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .database_filter_state_for_view(view_id)
    }

    pub(super) fn load_database_filter_users_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<LoadDatabaseFilterUsersResult, NotionLiveError> {
        load_database_filter_users(session, &self.workspace_context)
    }

    pub(super) fn search_database_filter_relation_pages_once(
        &self,
        session: &NotionDesktopSession,
        request: SearchDatabaseFilterRelationPagesRequest,
    ) -> Result<SearchDatabaseFilterRelationPagesResult, NotionLiveError> {
        search_database_filter_relation_pages(session, &self.workspace_context, request)
    }

    pub(super) fn load_date_undated_count_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<usize, NotionLiveError> {
        let state = self.collection_query_state.as_ref().ok_or_else(|| {
            "Notion no-date count loading requires an active date view".to_string()
        })?;
        crate::live::query_date_undated_count(session, state)
    }
}
