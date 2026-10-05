use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use super::{
    quick_find::{
        commit_quick_find_recents_refresh_state, merge_authoritative_quick_find_refresh,
        quick_find_response_is_complete, QuickFindRecentsCommitLocks,
    },
    QuickFindLocalSearchState, QuickFindRecentsRefresh, QuickFindRecentsState,
};
use crate::model::{
    notion_page_identity_key, LoadRecentPagesResult, PageShellIcon, PageShellSearchResult,
    QuickFindLocalSearchCache, RecentPageResult, SearchWorkspaceRequest, SearchWorkspaceResult,
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

fn search_request(session_id: &str, flow_number: u32) -> SearchWorkspaceRequest {
    SearchWorkspaceRequest {
        current_board_url: "https://www.notion.so/page".to_string(),
        query: "payroll".to_string(),
        scope: crate::model::SearchWorkspaceScope::AllContent,
        limit: 20,
        search_session_id: session_id.to_string(),
        flow_number,
        recent_pages_for_boosting: Vec::new(),
        excluded_block_ids: Vec::new(),
    }
}

#[test]
fn quick_find_recents_keep_newest_visit_and_have_deterministic_order() {
    let merged = LoadRecentPagesResult::merge_results(
        vec![
            recent_page("page-b", "Cached B", 200),
            recent_page("page-a", "Cached A", 100),
            recent_page("page-a", "Duplicate A", 90),
        ],
        vec![
            recent_page("page-a", "Fresh A", 300),
            recent_page("page-c", "Fresh C", 200),
            recent_page("page-b", "Older B", 150),
        ],
    );

    assert_eq!(
        merged
            .iter()
            .map(|page| (page.page.block_id.as_str(), page.page.title.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("page-a", "Fresh A"),
            ("page-b", "Cached B"),
            ("page-c", "Fresh C"),
        ]
    );
}

#[test]
fn authoritative_refresh_preserves_only_explicitly_concurrent_local_visits() {
    let future_remote = recent_page("future-remote", "Future remote", u64::MAX);
    let stale = recent_page("stale", "Stale", 100);
    let concurrent = recent_page("concurrent", "Concurrent", 200);
    let authoritative = recent_page("authoritative", "Authoritative", 150);
    let dirty_page_ids = HashSet::from([notion_page_identity_key(&concurrent.page.block_id)]);

    let (refreshed, concurrent_visits) = merge_authoritative_quick_find_refresh(
        vec![future_remote, stale, concurrent.clone()],
        vec![authoritative.clone()],
        &dirty_page_ids,
    );

    assert_eq!(concurrent_visits, vec![concurrent.clone()]);
    assert_eq!(refreshed, vec![concurrent, authoritative]);
}

#[test]
fn overlapping_recents_commits_publish_reset_in_token_order() {
    let commit_lock = Mutex::new(());
    let runtime_generation = Mutex::new(1);
    let recents = Mutex::new(QuickFindRecentsState {
        results: None,
        next_refresh_token: 3,
        last_committed_refresh_token: 0,
        dirty_page_ids_by_refresh: HashMap::from([(1, HashSet::new()), (2, HashSet::new())]),
    });
    let local_search = Mutex::new(QuickFindLocalSearchState::new(
        QuickFindLocalSearchCache::default(),
    ));
    let first = commit_quick_find_recents_refresh_state(
        QuickFindRecentsCommitLocks {
            commit_lock: &commit_lock,
            quick_find_runtime_generation: &runtime_generation,
            quick_find_recents: &recents,
            quick_find_local_search: &local_search,
        },
        vec![recent_page("first", "First", 100)],
        QuickFindRecentsRefresh {
            token: 1,
            started_at_unix_millis: 100,
            runtime_generation: 1,
        },
    )
    .expect("first refresh should commit");
    let second = commit_quick_find_recents_refresh_state(
        QuickFindRecentsCommitLocks {
            commit_lock: &commit_lock,
            quick_find_runtime_generation: &runtime_generation,
            quick_find_recents: &recents,
            quick_find_local_search: &local_search,
        },
        vec![recent_page("second", "Second", 200)],
        QuickFindRecentsRefresh {
            token: 2,
            started_at_unix_millis: 200,
            runtime_generation: 1,
        },
    )
    .expect("second refresh should commit");
    assert!(second.authority_version > first.authority_version);
    let recents = recents.lock().expect("final recents lock");
    assert_eq!(recents.last_committed_refresh_token, 2);
    assert_eq!(
        recents.results.as_ref().expect("final recents").results[0]
            .page
            .block_id,
        "second"
    );
    drop(recents);
    assert_eq!(
        local_search
            .lock()
            .expect("final local-search lock")
            .cache
            .pages[0]
            .block_id,
        "second"
    );
}

#[test]
fn superseded_runtime_refresh_cannot_invalidate_newer_query_authority() {
    let current_query_page = recent_page("current-query", "Current query", 300).page;
    let mut cache = QuickFindLocalSearchCache::default();
    cache.record_complete_query_results("all_content|payroll", [current_query_page.clone()]);
    let local_search = Mutex::new(QuickFindLocalSearchState::new(cache));
    let authority_before = local_search
        .lock()
        .expect("local-search lock")
        .authority_version;
    let recents = Mutex::new(QuickFindRecentsState {
        results: None,
        next_refresh_token: 2,
        last_committed_refresh_token: 0,
        dirty_page_ids_by_refresh: HashMap::from([(1, HashSet::new())]),
    });
    let cache_write = commit_quick_find_recents_refresh_state(
        QuickFindRecentsCommitLocks {
            commit_lock: &Mutex::new(()),
            quick_find_runtime_generation: &Mutex::new(2),
            quick_find_recents: &recents,
            quick_find_local_search: &local_search,
        },
        vec![recent_page("stale-refresh", "Stale refresh", 100)],
        QuickFindRecentsRefresh {
            token: 1,
            started_at_unix_millis: 100,
            runtime_generation: 1,
        },
    );

    assert!(cache_write.is_none());
    let local_search = local_search.lock().expect("final local-search lock");
    assert_eq!(local_search.authority_version, authority_before);
    assert!(local_search.cache.query_is_indexed("all_content|payroll"));
    assert_eq!(local_search.cache.pages, vec![current_query_page]);
    let recents = recents.lock().expect("final recents lock");
    assert!(recents.results.is_none());
    assert!(!recents.dirty_page_ids_by_refresh.contains_key(&1));
}

#[test]
fn authority_reset_rejects_in_flight_query_cache_updates() {
    let stale = recent_page("stale", "Payroll stale", 100).page;
    let fresh = recent_page("fresh", "Fresh", 200).page;
    let request = search_request("session-a", 1);
    let mut state = QuickFindLocalSearchState::new(QuickFindLocalSearchCache::default());
    state.begin_query(&request);
    state.reset_authority([fresh.clone()]);

    assert!(state
        .accept_query_response(&request, vec![stale], true, 300)
        .is_none());
    assert_eq!(state.cache.pages, vec![fresh]);
    assert!(!state.cache.query_is_indexed("all_content|payroll"));

    let current_request = search_request("session-b", 1);
    let current = recent_page("current", "Payroll current", 400).page;
    state.begin_query(&current_request);
    let write = state
        .accept_query_response(&current_request, vec![current.clone()], true, 500)
        .expect("post-refresh query should update the cache");
    assert_eq!(write.query_key, "all_content|payroll");
    assert_eq!(write.results, vec![current]);
    assert!(write.response_is_complete);
    assert!(state.cache.query_is_indexed("all_content|payroll"));
}

#[test]
fn partial_query_responses_authorize_only_the_rows_the_server_verified() {
    let request = search_request("session", 1);
    let verified = recent_page("verified", "Payroll verified", 100).page;
    let provisional = recent_page("provisional", "Payroll provisional", 90).page;
    let mut cache = QuickFindLocalSearchCache::default();
    cache.add_provisional_pages([provisional]);
    let mut state = QuickFindLocalSearchState::new(cache);
    state.begin_query(&request);

    let write = state
        .accept_query_response(&request, vec![verified.clone()], false, 200)
        .expect("current partial response should be accepted");

    assert!(!write.response_is_complete);
    assert_eq!(
        state.cache.indexed_result_ids("all_content|payroll"),
        Some([verified.block_id].as_slice())
    );
    assert!(state
        .cache
        .pages
        .iter()
        .any(|page| page.block_id == "provisional"));
}

#[test]
fn only_exhausted_unexcluded_search_responses_replace_query_membership() {
    let request = search_request("session", 1);
    let result = SearchWorkspaceResult {
        total: 2,
        consumed_result_count: 1,
        results: vec![recent_page("page-a", "Payroll", 100).page],
    };

    assert!(!quick_find_response_is_complete(&request, &result));
    let complete = SearchWorkspaceResult {
        total: 1,
        consumed_result_count: 1,
        results: result.results.clone(),
    };
    assert!(quick_find_response_is_complete(&request, &complete));
    let excluded = SearchWorkspaceRequest {
        excluded_block_ids: vec!["page-a".to_string()],
        ..request
    };
    assert!(!quick_find_response_is_complete(&excluded, &complete));
}
