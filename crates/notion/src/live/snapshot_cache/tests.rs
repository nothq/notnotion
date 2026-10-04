use std::collections::HashMap;

use super::{
    quick_find_cache_lock, NotionQuickFindCache, NotionWorkspaceSnapshotCacheStore,
    QuickFindQueryCacheUpdate, QuickFindRecentsRefreshUpdate,
    NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION,
};
use crate::live::workspace_runtime::{
    QuickFindQueryCacheWrite, QuickFindQueryCacheWriter, QuickFindRecentsCacheWrite,
    QuickFindRecentsCacheWriter,
};
use crate::model::{
    notion_page_identity_key, PageShellIcon, PageShellSearchResult, QuickFindLocalSearchCache,
    RecentPageResult,
};

fn recent_page(block_id: &str, title: &str, visited_at_unix_millis: u64) -> RecentPageResult {
    RecentPageResult {
        page: PageShellSearchResult {
            block_id: block_id.to_string(),
            title: title.to_string(),
            icon: PageShellIcon::named("page"),
            target_board_url: format!("https://www.notion.so/{block_id}"),
            highlight: None,
            match_snippet: None,
            editor_display_name: None,
            edited_label: None,
            edited_at: None,
            badges: Vec::new(),
        },
        visited_at_unix_millis,
    }
}

mod cache;
mod mutation;
mod refresh;
