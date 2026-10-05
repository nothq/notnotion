use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::PageShellIcon;

mod local_search;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellChatThread {
    pub thread_id: String,
    pub title: String,
    pub created_time: u64,
    pub sort_time: u64,
    pub unread: bool,
    pub pinned: bool,
}

#[derive(Clone, Debug)]
pub struct PageShellChatCursor(String);

impl PageShellChatCursor {
    pub(crate) fn from_serialized(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn serialized(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct LoadSidebarChatsRequest {
    pub current_board_url: String,
    pub cursor: Option<PageShellChatCursor>,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarChatsResult {
    pub threads: Vec<PageShellChatThread>,
    pub next_cursor: Option<PageShellChatCursor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellInboxActor {
    pub actor_id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellInboxItem {
    pub notification_id: String,
    pub activity_id: Option<String>,
    pub notification_type: String,
    pub actor: Option<PageShellInboxActor>,
    pub title: String,
    pub body: Option<String>,
    pub event_time_ms: Option<u64>,
    pub read: bool,
    pub archived: bool,
    pub target_board_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NotionSidebarInboxFilter {
    #[default]
    All,
    Unread,
    Archived,
    WorkspaceUpdates,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarInboxRequest {
    pub current_board_url: String,
    pub filter: NotionSidebarInboxFilter,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct LoadSidebarInboxResult {
    pub items: Vec<PageShellInboxItem>,
    pub has_more: bool,
}

#[derive(Clone, Debug)]
pub enum MutateSidebarInboxAction {
    SetRead {
        notification_ids: Vec<String>,
        read: bool,
    },
    SetArchived {
        notification_ids: Vec<String>,
        archived: bool,
    },
    MarkAllRead,
    ArchiveAll {
        filter: NotionSidebarInboxFilter,
        read_only: bool,
    },
}

#[derive(Clone, Debug)]
pub struct MutateSidebarInboxRequest {
    pub current_board_url: String,
    pub action: MutateSidebarInboxAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageShellSearchResult {
    pub block_id: String,
    pub title: String,
    pub icon: PageShellIcon,
    pub target_board_url: String,
    pub highlight: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<PageShellEditedAt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub badges: Vec<PageShellSearchBadge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageShellEditedAt {
    pub unix_millis: u64,
    pub date_label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageShellSearchBadge {
    CurrentPage,
    Database,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentPageVisit {
    pub page_id: String,
    pub visited_at_unix_millis: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentPageResult {
    pub page: PageShellSearchResult,
    pub visited_at_unix_millis: u64,
}

/// The bounded, encrypted client-side page index used by Quick Find.
///
/// `indexed_queries` records page membership that an authoritative query response has verified.
/// Complete responses replace a query's membership while partial responses extend its verified
/// subset. A page being present by itself is not sufficient: sidebar/bootstrap records are useful
/// provisional results, but excluding them would perturb Notion's authoritative server ranking.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickFindLocalSearchCache {
    #[serde(default)]
    pub pages: Vec<PageShellSearchResult>,
    #[serde(default)]
    pub indexed_queries: Vec<QuickFindIndexedQuery>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickFindIndexedQuery {
    pub query: String,
    pub result_ids: Vec<String>,
}

fn normalized_local_search_query(query: &str) -> String {
    query.trim().to_lowercase()
}

pub(crate) fn quick_find_local_query_key(scope: SearchWorkspaceScope, query: &str) -> String {
    let scope = match scope {
        SearchWorkspaceScope::AllContent => "all_content",
        SearchWorkspaceScope::TitleOnly => "title_only",
    };
    format!("{scope}|{}", normalized_local_search_query(query))
}

impl RecentPageResult {
    pub fn recent_page_visit(&self) -> RecentPageVisit {
        RecentPageVisit {
            page_id: self.page.block_id.clone(),
            visited_at_unix_millis: self.visited_at_unix_millis,
        }
    }

    pub fn into_page(self) -> PageShellSearchResult {
        self.page
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchWorkspaceScope {
    #[default]
    AllContent,
    TitleOnly,
}

#[derive(Clone, Debug)]
pub struct LoadRecentPagesRequest {
    pub current_board_url: String,
    pub limit: u8,
}

#[derive(Clone, Debug)]
pub struct LoadRecentPagesResult {
    pub results: Vec<RecentPageResult>,
}

impl LoadRecentPagesResult {
    pub const MAX_CACHED_RESULTS: usize = 1_000;

    pub fn recent_pages_for_boosting(&self) -> Vec<RecentPageVisit> {
        self.results
            .iter()
            .take(SearchWorkspaceRequest::MAX_RECENT_PAGES_FOR_BOOSTING)
            .map(RecentPageResult::recent_page_visit)
            .collect()
    }

    pub fn into_pages(self) -> Vec<PageShellSearchResult> {
        self.results
            .into_iter()
            .map(RecentPageResult::into_page)
            .collect()
    }

    pub(crate) fn merge_results(
        cached: Vec<RecentPageResult>,
        incoming: Vec<RecentPageResult>,
    ) -> Vec<RecentPageResult> {
        let mut by_page_id: HashMap<String, RecentPageResult> =
            HashMap::with_capacity(cached.len() + incoming.len());
        for page in cached.into_iter().chain(incoming) {
            match by_page_id.entry(notion_page_identity_key(&page.page.block_id)) {
                std::collections::hash_map::Entry::Occupied(mut entry)
                    if page.visited_at_unix_millis >= entry.get().visited_at_unix_millis =>
                {
                    entry.insert(page);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(page);
                }
                std::collections::hash_map::Entry::Occupied(_) => {}
            }
        }
        let mut merged = by_page_id.into_values().collect::<Vec<_>>();
        merged.sort_by(|left, right| {
            right
                .visited_at_unix_millis
                .cmp(&left.visited_at_unix_millis)
                .then_with(|| left.page.block_id.cmp(&right.page.block_id))
        });
        merged.truncate(Self::MAX_CACHED_RESULTS);
        merged
    }
}

pub(crate) fn notion_page_identity_key(page_id: &str) -> String {
    uuid::Uuid::parse_str(page_id)
        .map(|page_id| page_id.simple().to_string())
        .unwrap_or_else(|_| page_id.to_string())
}

#[derive(Clone, Debug)]
pub struct SearchWorkspaceRequest {
    pub current_board_url: String,
    pub query: String,
    pub scope: SearchWorkspaceScope,
    pub limit: u32,
    pub search_session_id: String,
    pub flow_number: u32,
    pub recent_pages_for_boosting: Vec<RecentPageVisit>,
    pub excluded_block_ids: Vec<String>,
}

impl SearchWorkspaceRequest {
    pub const MAX_RECENT_PAGES_FOR_BOOSTING: usize = 50;
}

#[derive(Clone, Debug)]
pub struct SearchWorkspaceResult {
    pub total: u32,
    pub consumed_result_count: u32,
    pub results: Vec<PageShellSearchResult>,
}
