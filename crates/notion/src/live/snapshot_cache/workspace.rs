use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    cache_key_lock, invalid_cache_failure, unreadable_cache_failure, workspace_cache_path,
    NotionWorkspaceSnapshotCache, NotionWorkspaceSnapshotCacheStore,
    ValidatedNotionWorkspaceSnapshot, NOTION_CACHE_DIR_ENV, NOTION_WORKSPACE_CACHE_KEY_NAMESPACE,
    NOTION_WORKSPACE_CACHE_SCHEMA_VERSION,
};
use crate::live::{board::canonicalize_board_url, credentials::NotionDesktopSession};
use crate::model::{
    BoardSnapshot, NotionCacheFailure, NotionCachedBootstrapOutcome,
    NotionCachedWorkspaceBootstrap, NotionLaunchRoute, NotionRouteSource,
};

impl NotionWorkspaceSnapshotCacheStore {
    pub(crate) fn from_session(session: &NotionDesktopSession) -> Result<Self, String> {
        let active_user_id = session.user_id().to_string();
        let cache_key = format!("{NOTION_WORKSPACE_CACHE_KEY_NAMESPACE}|{active_user_id}");
        let key = {
            let _guard = cache_key_lock()
                .lock()
                .map_err(|_| "Notion workspace cache key lock is poisoned".to_string())?;
            local_cache::load_or_create_cache_key(&cache_key, "Notion workspace snapshot")?
        };
        let cache_root = local_cache::default_cache_root_dir(
            NOTION_CACHE_DIR_ENV,
            "notion",
            "Notion workspace snapshot",
        )?;
        Ok(Self {
            path: workspace_cache_path(&cache_root, &cache_key),
            key,
            active_user_id,
        })
    }

    pub(crate) fn load_workspace(
        &self,
        route_source: &NotionRouteSource,
        code_settings: crate::model::CardPageCodeSettingsCapability,
    ) -> NotionCachedBootstrapOutcome {
        match self.load_validated_workspace(route_source) {
            Ok(Some(snapshot)) => {
                NotionCachedBootstrapOutcome::Loaded(Box::new(NotionCachedWorkspaceBootstrap {
                    route: snapshot.route,
                    workspace: snapshot.workspace,
                    code_settings,
                }))
            }
            Ok(None) => NotionCachedBootstrapOutcome::Miss,
            Err(error) => NotionCachedBootstrapOutcome::Failed(error),
        }
    }

    pub(super) fn load_route(
        &self,
        route_source: &NotionRouteSource,
    ) -> Result<Option<NotionLaunchRoute>, NotionCacheFailure> {
        self.load_validated_workspace(route_source)
            .map(|snapshot| snapshot.map(|snapshot| snapshot.route))
    }

    fn load_validated_workspace(
        &self,
        route_source: &NotionRouteSource,
    ) -> Result<Option<ValidatedNotionWorkspaceSnapshot>, NotionCacheFailure> {
        let Some(cache) = self.read_workspace_cache()? else {
            return Ok(None);
        };
        if cache.schema_version != NOTION_WORKSPACE_CACHE_SCHEMA_VERSION
            || cache.active_user_id != self.active_user_id
        {
            return Err(invalid_cache_failure(
                "cached Notion workspace identity or schema does not match this session",
            ));
        }
        if let Some(page) = cache.workspace.page_content.as_ref() {
            page.validate_block_hierarchy().map_err(|error| {
                invalid_cache_failure(format!(
                    "cached Notion page block hierarchy is invalid: {error}"
                ))
            })?;
        }
        let Ok(cached_canonical_board_url) =
            canonicalize_board_url(cache.route.board_url().as_str())
        else {
            return Err(invalid_cache_failure(
                "cached Notion workspace route is invalid",
            ));
        };
        if cached_canonical_board_url != cache.canonical_board_url {
            return Err(invalid_cache_failure(
                "cached Notion workspace route does not match its canonical URL",
            ));
        }
        let route = match route_source {
            NotionRouteSource::Explicit(route) => {
                let Ok(explicit_canonical_board_url) =
                    canonicalize_board_url(route.board_url().as_str())
                else {
                    return Err(invalid_cache_failure(
                        "explicit Notion route could not be canonicalized",
                    ));
                };
                if explicit_canonical_board_url != cache.canonical_board_url {
                    return Ok(None);
                }
                route.clone()
            }
            NotionRouteSource::LastOpened => cache.route,
        };
        Ok(Some(ValidatedNotionWorkspaceSnapshot {
            route,
            workspace: cache.workspace,
        }))
    }

    fn read_workspace_cache(
        &self,
    ) -> Result<Option<NotionWorkspaceSnapshotCache>, NotionCacheFailure> {
        local_cache::read_encrypted_json(&self.path, &self.key).map_err(|error| {
            unreadable_cache_failure(format!(
                "failed to read the cached Notion workspace: {error}"
            ))
        })
    }

    pub(crate) fn persist_workspace(&self, route: &NotionLaunchRoute, workspace: &BoardSnapshot) {
        let Ok(canonical_board_url) = canonicalize_board_url(route.board_url().as_str()) else {
            return;
        };
        let cache = NotionWorkspaceSnapshotCache {
            schema_version: NOTION_WORKSPACE_CACHE_SCHEMA_VERSION,
            fetched_at_unix_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after Unix epoch for Notion cache")
                .as_secs(),
            active_user_id: self.active_user_id.clone(),
            canonical_board_url,
            route: route.clone(),
            workspace: workspace.clone(),
        };
        if let Err(error) = local_cache::write_encrypted_json(&self.path, &self.key, &cache) {
            println!("notnotion workspace cache write failed: {error}");
        }
    }
}
