use std::collections::HashSet;

use super::quick_find_store::{empty_quick_find_space, take_quick_find_space};
use super::{quick_find_cache_lock, NotionWorkspaceSnapshotCacheStore};
use crate::model::notion_page_identity_key;

impl NotionWorkspaceSnapshotCacheStore {
    pub(crate) fn persist_quick_find_page_mutation(
        &self,
        space_id: &str,
        page_ids: &[String],
        mutation_authority_version: u64,
        invalidated_at_unix_millis: u64,
    ) {
        let Ok(_guard) = quick_find_cache_lock().lock() else {
            println!("notnotion Quick Find cache lock is poisoned");
            return;
        };
        let path = self.quick_find_path();
        let mut cache = self.read_quick_find_cache_for_write(&path);
        let existing = take_quick_find_space(&mut cache, space_id);
        if existing
            .as_ref()
            .is_some_and(|space| space.authority_version > mutation_authority_version)
        {
            cache.spaces.insert(
                0,
                existing.expect("newer Quick Find cache space must exist"),
            );
            return;
        }
        let mut space = existing.unwrap_or_else(|| empty_quick_find_space(space_id));
        let invalidated_identities = page_ids
            .iter()
            .map(|page_id| notion_page_identity_key(page_id))
            .collect::<HashSet<_>>();
        for page_id in &invalidated_identities {
            space.local_search.invalidate_page_mutation(page_id);
        }
        space.results.retain(|result| {
            !invalidated_identities.contains(&notion_page_identity_key(&result.page.block_id))
        });
        space.local_visits.retain(|visit| {
            !invalidated_identities.contains(&notion_page_identity_key(&visit.page_id))
        });
        space.refreshed_at_unix_millis = 0;
        space.local_search_updated_at_unix_millis = space
            .local_search_updated_at_unix_millis
            .max(invalidated_at_unix_millis);
        space.authority_version = space.authority_version.max(mutation_authority_version);
        space.query_authority_version_by_key.clear();
        cache.spaces.insert(0, space);
        self.write_quick_find_cache(&path, &mut cache);
    }
}
