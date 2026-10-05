use super::quick_find_store::{
    empty_quick_find_space, merge_quick_find_local_visits, take_quick_find_space,
};
use super::{
    invalid_cache_failure, quick_find_cache_lock, unreadable_cache_failure, NotionQuickFindCache,
    NotionQuickFindLocalSearchSnapshot, NotionQuickFindSpaceCache,
    NotionWorkspaceSnapshotCacheStore, NOTION_QUICK_FIND_CACHE_MAX_BYTES,
    NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION,
};
#[cfg(test)]
use crate::model::QuickFindLocalSearchCache;
use crate::model::{NotionCacheFailure, RecentPageResult};

impl NotionWorkspaceSnapshotCacheStore {
    pub(crate) fn load_quick_find_recents(
        &self,
        space_id: &str,
    ) -> Result<Option<Vec<RecentPageResult>>, NotionCacheFailure> {
        let _guard = quick_find_cache_lock()
            .lock()
            .map_err(|_| unreadable_cache_failure("Notion Quick Find cache lock is poisoned"))?;
        let cache: Option<NotionQuickFindCache> = local_cache::read_encrypted_json_strict_bounded(
            &self.quick_find_path(),
            &self.key,
            NOTION_QUICK_FIND_CACHE_MAX_BYTES,
        )
        .map_err(|error| {
            unreadable_cache_failure(format!(
                "failed to read the cached Notion Quick Find recents: {error}"
            ))
        })?;
        let Some(cache) = cache else {
            return Ok(None);
        };
        if cache.schema_version != NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION
            || cache.active_user_id != self.active_user_id
        {
            return Err(invalid_cache_failure(
                "cached Notion Quick Find identity or schema does not match this session",
            ));
        }
        Ok(cache
            .spaces
            .into_iter()
            .find(|space| space.space_id == space_id)
            .map(|space| space.results))
    }

    pub(crate) fn persist_quick_find_recents(&self, space_id: &str, results: &[RecentPageResult]) {
        let Ok(_guard) = quick_find_cache_lock().lock() else {
            println!("notnotion Quick Find cache lock is poisoned");
            return;
        };
        let path = self.quick_find_path();
        let mut cache = self.read_quick_find_cache_for_write(&path);
        let existing = take_quick_find_space(&mut cache, space_id)
            .unwrap_or_else(|| empty_quick_find_space(space_id));
        let additions = results
            .iter()
            .filter(|result| result.visited_at_unix_millis >= existing.refreshed_at_unix_millis)
            .cloned()
            .collect::<Vec<_>>();
        let local_visits = merge_quick_find_local_visits(existing.local_visits, &additions);
        cache.spaces.insert(
            0,
            NotionQuickFindSpaceCache {
                space_id: space_id.to_string(),
                refreshed_at_unix_millis: existing.refreshed_at_unix_millis,
                local_search_updated_at_unix_millis: existing.local_search_updated_at_unix_millis,
                authority_version: existing.authority_version,
                query_authority_version_by_key: existing.query_authority_version_by_key,
                local_visits,
                results: crate::model::LoadRecentPagesResult::merge_results(
                    existing.results,
                    additions,
                ),
                local_search: existing.local_search,
            },
        );
        self.write_quick_find_cache(&path, &mut cache);
    }

    #[cfg(test)]
    pub(crate) fn load_quick_find_local_search(
        &self,
        space_id: &str,
    ) -> Result<Option<QuickFindLocalSearchCache>, NotionCacheFailure> {
        self.load_quick_find_local_search_with_authority(space_id)
            .map(|loaded| loaded.map(|loaded| loaded.local_search))
    }

    pub(crate) fn load_quick_find_local_search_with_authority(
        &self,
        space_id: &str,
    ) -> Result<Option<NotionQuickFindLocalSearchSnapshot>, NotionCacheFailure> {
        let _guard = quick_find_cache_lock()
            .lock()
            .map_err(|_| unreadable_cache_failure("Notion Quick Find cache lock is poisoned"))?;
        let cache: Option<NotionQuickFindCache> = local_cache::read_encrypted_json_strict_bounded(
            &self.quick_find_path(),
            &self.key,
            NOTION_QUICK_FIND_CACHE_MAX_BYTES,
        )
        .map_err(|error| {
            unreadable_cache_failure(format!(
                "failed to read the cached Notion Quick Find local index: {error}"
            ))
        })?;
        let Some(cache) = cache else {
            return Ok(None);
        };
        if cache.schema_version != NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION
            || cache.active_user_id != self.active_user_id
        {
            return Err(invalid_cache_failure(
                "cached Notion Quick Find identity or schema does not match this session",
            ));
        }
        Ok(cache
            .spaces
            .into_iter()
            .find(|space| space.space_id == space_id)
            .map(|space| NotionQuickFindLocalSearchSnapshot {
                local_search: space.local_search,
                authority_version: space.authority_version,
            }))
    }
}
