use super::quick_find_store::{empty_quick_find_space, take_quick_find_space};
use super::{quick_find_cache_lock, NotionWorkspaceSnapshotCacheStore};

pub(crate) struct QuickFindQueryCacheUpdate<'a> {
    pub(crate) query_key: &'a str,
    pub(crate) results: &'a [crate::model::PageShellSearchResult],
    pub(crate) response_is_complete: bool,
    pub(crate) query_authority_version: u64,
    pub(crate) accepted_at_unix_millis: u64,
}

impl NotionWorkspaceSnapshotCacheStore {
    pub(crate) fn persist_quick_find_query_cache(
        &self,
        space_id: &str,
        update: QuickFindQueryCacheUpdate<'_>,
    ) {
        let QuickFindQueryCacheUpdate {
            query_key,
            results,
            response_is_complete,
            query_authority_version,
            accepted_at_unix_millis,
        } = update;
        let Ok(_guard) = quick_find_cache_lock().lock() else {
            println!("notnotion Quick Find cache lock is poisoned");
            return;
        };
        let path = self.quick_find_path();
        let mut cache = self.read_quick_find_cache_for_write(&path);
        let mut existing = take_quick_find_space(&mut cache, space_id);
        if existing
            .as_ref()
            .is_some_and(|space| space.authority_version > query_authority_version)
        {
            cache.spaces.insert(
                0,
                existing.expect("newer Quick Find cache space must exist"),
            );
            return;
        }
        let mut space = existing
            .take()
            .unwrap_or_else(|| empty_quick_find_space(space_id));
        if response_is_complete {
            space
                .local_search
                .record_complete_query_results(query_key, results.iter().cloned());
        } else {
            space
                .local_search
                .record_partial_query_results(query_key, results.iter().cloned());
        }
        let query_key = query_key.trim().to_lowercase();
        space
            .query_authority_version_by_key
            .insert(query_key, query_authority_version);
        space.query_authority_version_by_key.retain(|query_key, _| {
            space
                .local_search
                .indexed_queries
                .iter()
                .any(|indexed| indexed.query == *query_key)
        });
        space.local_search_updated_at_unix_millis = space
            .local_search_updated_at_unix_millis
            .max(accepted_at_unix_millis);
        space.authority_version = space.authority_version.max(query_authority_version);
        cache.spaces.insert(0, space);
        self.write_quick_find_cache(&path, &mut cache);
    }
}
