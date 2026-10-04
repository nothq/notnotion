use std::sync::Arc;

use crate::live::{
    code_preferences::NotionCodePreferences,
    credentials::{stored_notion_desktop_session, NotionDesktopSession},
    load_board_snapshot, load_board_snapshot_for_bootstrap, routing,
    snapshot_cache::{stored_notion_workspace_cache_store, NotionWorkspaceSnapshotCacheStore},
    NotionLiveError, NotionWorkspaceRuntime, NotionWorkspaceRuntimeInput,
};
use crate::model::{
    BoardSnapshot, NotionCachedBootstrapOutcome, NotionPreviousStateDisposition, NotionRouteSource,
    NotionWorkspaceBootstrap,
};

#[derive(Clone)]
struct ProductionNotionBootstrapApi {
    route_source: NotionRouteSource,
    code_preferences: Arc<NotionCodePreferences>,
}

pub fn production_notion_bootstrap_api(
    route_source: NotionRouteSource,
) -> Arc<dyn crate::model::NotionBootstrapApi> {
    Arc::new(ProductionNotionBootstrapApi {
        route_source,
        code_preferences: Arc::new(NotionCodePreferences::default()),
    })
}

pub fn load_notion_board_snapshot(
    route: &crate::model::NotionLaunchRoute,
) -> Result<BoardSnapshot, String> {
    load_board_snapshot(route.board_url().as_str()).map(|loaded| loaded.snapshot)
}

impl crate::model::NotionBootstrapApi for ProductionNotionBootstrapApi {
    fn load_cached_workspace(&self) -> NotionCachedBootstrapOutcome {
        let session = match stored_notion_desktop_session() {
            Ok(Some(session)) => session,
            Ok(None) => return NotionCachedBootstrapOutcome::Miss,
            Err(crate::live::credentials::StoredNotionSessionError::Invalid(error)) => {
                return NotionCachedBootstrapOutcome::Failed(
                    crate::model::NotionCacheFailure::Invalid { diagnostic: error },
                );
            }
            Err(crate::live::credentials::StoredNotionSessionError::Storage(error)) => {
                return NotionCachedBootstrapOutcome::Failed(
                    crate::model::NotionCacheFailure::Unreadable { diagnostic: error },
                );
            }
        };
        match routing::notion_desktop_restoration_has_different_user(session.user_id()) {
            Ok(true) => return NotionCachedBootstrapOutcome::Miss,
            Ok(false) => {}
            Err(diagnostic) => {
                return NotionCachedBootstrapOutcome::Failed(
                    crate::model::NotionCacheFailure::Unreadable { diagnostic },
                );
            }
        }
        let code_settings = self.code_preferences.for_session(&session);
        match NotionWorkspaceSnapshotCacheStore::from_session(&session) {
            Ok(store) => store.load_workspace(&self.route_source, code_settings),
            Err(error) => {
                NotionCachedBootstrapOutcome::Failed(crate::model::NotionCacheFailure::Unreadable {
                    diagnostic: error,
                })
            }
        }
    }

    fn bootstrap_workspace(&self) -> crate::model::NotionBootstrapOutcome {
        let recovery = crate::live::credentials::with_notion_session_recovery(|session| {
            let validated = routing::validate_notion_desktop_session(session)?;
            self.bootstrap_workspace_once(session, &validated)
        })
        .map_err(bootstrap_recovery_failure)?;
        let (mut bootstrap, identity_change) = recovery.into_parts();
        if identity_change
            .recovery_disposition()
            .discards_previous_state()
        {
            bootstrap.previous_state_disposition = NotionPreviousStateDisposition::Discard;
        }
        Ok(bootstrap)
    }
}

fn bootstrap_recovery_failure(
    error: crate::live::credentials::NotionSessionRecoveryError,
) -> crate::model::NotionBootstrapFailure {
    crate::model::NotionBootstrapFailure::recovering(
        error.to_string(),
        error.recovery_disposition(),
    )
}

impl ProductionNotionBootstrapApi {
    fn bootstrap_workspace_once(
        &self,
        session: &NotionDesktopSession,
        validated: &routing::ValidatedNotionSession,
    ) -> Result<NotionWorkspaceBootstrap, NotionLiveError> {
        match &self.route_source {
            NotionRouteSource::Explicit(route) => {
                self.load_candidate(session, routing::explicit_route_candidate(route.clone()))
            }
            NotionRouteSource::LastOpened => self.load_last_opened(session, validated),
        }
    }

    fn load_last_opened(
        &self,
        session: &NotionDesktopSession,
        validated: &routing::ValidatedNotionSession,
    ) -> Result<NotionWorkspaceBootstrap, NotionLiveError> {
        if let Some(bootstrap) = self.try_cached_candidate(session)? {
            return Ok(bootstrap);
        }
        let fallbacks = routing::NotionFallbackRouteResolver::new(session, validated)?;
        if let Some(recent) = fallbacks.recent()? {
            if let Some(bootstrap) = self.try_fallback_candidate(session, recent)? {
                return Ok(bootstrap);
            }
        }
        self.load_candidate(session, fallbacks.home()?)
    }

    fn try_cached_candidate(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<Option<NotionWorkspaceBootstrap>, NotionLiveError> {
        match routing::cached_route_candidate(session) {
            routing::NotionCachedRouteCandidate::Miss => Ok(None),
            routing::NotionCachedRouteCandidate::Failed(error) => {
                println!("notnotion cached route is unusable: {error}");
                Ok(None)
            }
            routing::NotionCachedRouteCandidate::Candidate(candidate) => {
                self.try_fallback_candidate(session, candidate)
            }
        }
    }

    fn try_fallback_candidate(
        &self,
        session: &NotionDesktopSession,
        candidate: routing::NotionRouteCandidate,
    ) -> Result<Option<NotionWorkspaceBootstrap>, NotionLiveError> {
        let kind = candidate.kind;
        match self.load_candidate(session, candidate) {
            Ok(bootstrap) => Ok(Some(bootstrap)),
            Err(NotionLiveError::Unavailable(error)) => {
                println!("notnotion {kind} route is unavailable: {error}");
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn load_candidate(
        &self,
        session: &NotionDesktopSession,
        candidate: routing::NotionRouteCandidate,
    ) -> Result<NotionWorkspaceBootstrap, NotionLiveError> {
        let board_url = candidate.route.board_url();
        let loaded = load_board_snapshot_for_bootstrap(session, board_url.as_str())?;
        let code_settings = self.code_preferences.for_session(session);
        build_workspace_bootstrap(
            session,
            Arc::new(self.clone()),
            candidate,
            loaded,
            code_settings,
        )
    }
}

fn build_workspace_bootstrap(
    session: &NotionDesktopSession,
    rebootstrap_api: Arc<dyn crate::model::NotionBootstrapApi>,
    candidate: routing::NotionRouteCandidate,
    loaded: crate::live::LoadedBoardSnapshot,
    code_settings: crate::model::CardPageCodeSettingsCapability,
) -> Result<NotionWorkspaceBootstrap, NotionLiveError> {
    let mutator = loaded.mutator.ok_or_else(|| {
        NotionLiveError::Fatal(
            "live Notion bootstrap did not provide its required mutation capability".to_string(),
        )
    })?;
    let workspace_context = loaded.workspace_context.ok_or_else(|| {
        NotionLiveError::Fatal(
            "live Notion bootstrap did not provide its required workspace context".to_string(),
        )
    })?;
    let snapshot_cache = workspace_cache_store();
    if let Some(snapshot_cache) = snapshot_cache.as_ref() {
        snapshot_cache.persist_workspace(&candidate.route, &loaded.snapshot);
    }
    let workspace = loaded.snapshot;
    let workspace_api = Arc::new(NotionWorkspaceRuntime::new(NotionWorkspaceRuntimeInput {
        active_user_id: session.user_id().to_string(),
        rebootstrap_api,
        mutator,
        workspace_context,
        collection_query_state: loaded.collection_query_state,
        database_query_state: loaded.database_query_state,
        snapshot_cache,
        quick_find_runtime_generation: None,
        code_settings: code_settings.clone(),
    }));
    Ok(NotionWorkspaceBootstrap {
        route: candidate.route,
        workspace,
        workspace_api,
        code_settings,
        previous_state_disposition: NotionPreviousStateDisposition::PreserveCompatible,
    })
}

fn workspace_cache_store() -> Option<NotionWorkspaceSnapshotCacheStore> {
    match stored_notion_workspace_cache_store() {
        Ok(store) => store,
        Err(error) => {
            println!("notnotion workspace cache is unavailable: {error}");
            None
        }
    }
}
