use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::{
    NotionQuickFindCache, NotionQuickFindSpaceCache, NotionWorkspaceSnapshotCacheStore,
    NOTION_QUICK_FIND_CACHED_SPACE_LIMIT, NOTION_QUICK_FIND_CACHE_MAX_BYTES,
    NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION,
};
use crate::model::{
    notion_page_identity_key, QuickFindLocalSearchCache, RecentPageResult, RecentPageVisit,
};

impl NotionWorkspaceSnapshotCacheStore {
    pub(super) fn read_quick_find_cache_for_write(&self, path: &Path) -> NotionQuickFindCache {
        match local_cache::read_encrypted_json_strict_bounded::<NotionQuickFindCache>(
            path,
            &self.key,
            NOTION_QUICK_FIND_CACHE_MAX_BYTES,
        ) {
            Ok(Some(cache))
                if cache.schema_version == NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION
                    && cache.active_user_id == self.active_user_id =>
            {
                cache
            }
            Ok(Some(_)) | Ok(None) => self.empty_quick_find_cache(),
            Err(error) => {
                println!(
                    "notnotion Quick Find cache read before write failed; rebuilding it: {error}"
                );
                self.empty_quick_find_cache()
            }
        }
    }

    fn empty_quick_find_cache(&self) -> NotionQuickFindCache {
        NotionQuickFindCache {
            schema_version: NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION,
            active_user_id: self.active_user_id.clone(),
            spaces: Vec::new(),
        }
    }

    pub(super) fn write_quick_find_cache(&self, path: &Path, cache: &mut NotionQuickFindCache) {
        cache.spaces.truncate(NOTION_QUICK_FIND_CACHED_SPACE_LIMIT);
        if let Err(error) = local_cache::write_encrypted_json_durable_bounded(
            path,
            &self.key,
            cache,
            NOTION_QUICK_FIND_CACHE_MAX_BYTES,
        ) {
            println!("notnotion Quick Find cache write failed: {error}");
        }
    }

    pub(super) fn quick_find_path(&self) -> PathBuf {
        self.path.with_file_name("quick-find.bin")
    }
}

pub(super) fn take_quick_find_space(
    cache: &mut NotionQuickFindCache,
    space_id: &str,
) -> Option<NotionQuickFindSpaceCache> {
    cache
        .spaces
        .iter()
        .position(|space| space.space_id == space_id)
        .map(|index| cache.spaces.remove(index))
}

pub(super) fn empty_quick_find_space(space_id: &str) -> NotionQuickFindSpaceCache {
    NotionQuickFindSpaceCache {
        space_id: space_id.to_string(),
        refreshed_at_unix_millis: 0,
        local_search_updated_at_unix_millis: 0,
        authority_version: 0,
        query_authority_version_by_key: HashMap::new(),
        local_visits: Vec::new(),
        results: Vec::new(),
        local_search: QuickFindLocalSearchCache::default(),
    }
}

pub(super) fn merge_quick_find_local_visits(
    mut existing: Vec<RecentPageVisit>,
    additions: &[RecentPageResult],
) -> Vec<RecentPageVisit> {
    for addition in additions {
        let identity = notion_page_identity_key(&addition.page.block_id);
        if existing.iter().any(|visit| {
            notion_page_identity_key(&visit.page_id) == identity
                && visit.visited_at_unix_millis >= addition.visited_at_unix_millis
        }) {
            continue;
        }
        existing.retain(|visit| notion_page_identity_key(&visit.page_id) != identity);
        existing.push(addition.recent_page_visit());
    }
    existing.sort_by(|left, right| {
        right
            .visited_at_unix_millis
            .cmp(&left.visited_at_unix_millis)
            .then_with(|| left.page_id.cmp(&right.page_id))
    });
    existing.truncate(crate::model::LoadRecentPagesResult::MAX_CACHED_RESULTS);
    existing
}
