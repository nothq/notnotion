use crate::ui::search::QuickFindAction;

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpui::{
    point, px, AppContext as _, Entity, Keystroke, ListOffset, Modifiers, ScrollDelta,
    ScrollWheelEvent, TestAppContext, TouchPhase, VisualTestContext, WindowHandle,
};

use crate::{
    model::{
        BoardSnapshot, CardPage, CardPageBlock, CardPageBlockKind, LoadRecentPagesRequest,
        LoadRecentPagesResult, MoveCardRequest, NotionLaunchRoute, NotionWorkspaceApi,
        NotionWorkspaceLoad, NotionWorkspaceResult, PageShellIcon, PageShellSearchBadge,
        PageShellSearchResult, QuickFindLocalSearchCache, RecentPageResult, RecentPageVisit,
        SearchWorkspaceRequest, SearchWorkspaceResult, SearchWorkspaceScope,
    },
    ui::tests::cases::*,
};

use crate::ui::tests::cases::quick_find::{api::*, support::*};

#[gpui::test]
fn closed_recents_completion_warms_reopen_without_a_second_fresh_request(cx: &mut TestAppContext) {
    let cached = vec![
        search_result("cached-a", "Cached A", Some("First cached highlight")),
        search_result("cached-b", "Cached B", None),
    ];
    let api = RecordingQuickFindApi::new(vec![cached.clone()], Vec::new(), &[], HashMap::new());
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| {
        surface.activate_notion_search(cx);
        assert!(matches!(
            surface.notion_search.results,
            NotionSearchResultsState::Loading
        ));
        surface.apply_quick_find_action(QuickFindAction::Close, cx);
        assert!(!surface.notion_chrome.notion_search_open);
    });
    cx.run_until_parked();
    assert_eq!(api.recent_requests().len(), 1);

    surface.update(cx, |surface, cx| {
        surface.activate_notion_search(cx);
        assert!(surface.notion_chrome.notion_search_open);
        assert_loaded_results(surface, cached.len() as u32, &cached);
    });
    assert_eq!(
        api.recent_requests().len(),
        1,
        "a completed cache younger than 60 seconds must suppress refresh"
    );

    cx.run_until_parked();
    cx.read_entity(&surface, |surface, _| {
        assert_loaded_results(surface, cached.len() as u32, &cached);
    });
    let requests = api.recent_requests();
    assert_eq!(requests.len(), 1);
    for request in requests {
        assert_eq!(request.current_board_url, TEST_BOARD_URL);
        assert_eq!(request.limit, 50);
    }
}

#[gpui::test]
fn stale_persisted_recents_render_immediately_then_yield_to_the_authoritative_refresh(
    cx: &mut TestAppContext,
) {
    let cached_a = search_result("cached-a", "Cached A", None);
    let cached_b = search_result("cached-b", "Cached B", None);
    let cached = vec![
        recent_result(cached_a.clone(), 100),
        recent_result(cached_b.clone(), 300),
    ];
    let refreshed_a = search_result("cached-a", "Cached A refreshed", None);
    let refreshed_c = search_result("fresh-c", "Fresh C", None);
    let refreshed = vec![
        recent_result(refreshed_c.clone(), 400),
        recent_result(refreshed_a.clone(), 500),
    ];
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &[], HashMap::new());
    api.set_persisted_recents(Some("workspace-a"), cached);
    api.set_recent_responses(vec![refreshed]);
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.read_entity(&surface, |surface, _| {
        assert_loaded_results(surface, 2, &[cached_b.clone(), cached_a.clone()]);
    });
    assert!(
        api.recent_requests().is_empty(),
        "the persisted rows render before the background worker is polled"
    );

    cx.run_until_parked();
    cx.read_entity(&surface, |surface, _| {
        assert_loaded_results(surface, 2, &[refreshed_a.clone(), refreshed_c.clone()]);
    });
    assert_eq!(api.recent_requests().len(), 1);
}

#[gpui::test]
fn authoritative_recents_refresh_keeps_only_visits_recorded_while_it_was_in_flight() {
    let stale = search_result("stale", "Stale", None);
    let refreshed = search_result("refreshed", "Refreshed", None);
    let concurrent = search_result("concurrent", "Concurrent", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(vec![recent_result(stale, 100)]),
    );
    let refresh_token = state
        .begin_recents_refresh(true)
        .0
        .expect("start authoritative recents refresh");
    state.record_recent_page(recent_result(concurrent.clone(), 300));
    state.selected_index = 5;

    assert_eq!(
        state.commit_recents_refresh(
            refresh_token,
            vec![recent_result(refreshed.clone(), 200)],
            true,
        ),
        NotionSearchRecentsCommit::ShowingRecents
    );
    let NotionSearchResultsState::Loaded { total, results, .. } = &state.results else {
        panic!("authoritative recents refresh should load results");
    };
    assert_eq!(*total, 2);
    assert_eq!(results.as_ref(), &[concurrent, refreshed]);
    assert_eq!(state.selected_index, 1);
}

#[test]
fn authoritative_recents_refresh_invalidates_exclusions_then_revalidates_cached_rows() {
    let stale = search_result("stale", "Payroll", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(vec![recent_result(stale.clone(), 100)]),
    );
    state.prepare_local_search_cache(indexed_local_search_cache("payroll", vec![stale.clone()]));
    let refresh_token = state
        .begin_recents_refresh(true)
        .0
        .expect("start authoritative recents refresh");
    state.query = "payroll".to_string();
    let stale_query_token = state.begin_request();
    assert_eq!(
        state.local_result_ids_for_query(stale_query_token),
        vec!["stale"]
    );
    state.mark_query_request_dispatched(stale_query_token);

    assert_eq!(
        state.commit_recents_refresh(refresh_token, Vec::new(), true),
        NotionSearchRecentsCommit::RequeryImmediately
    );
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("the cached row should remain visible while its exclusion authority is stale");
    };
    assert_eq!(results.as_ref(), std::slice::from_ref(&stale));
    let authoritative_query_token = state.begin_request();
    assert!(state
        .local_result_ids_for_query(authoritative_query_token)
        .is_empty());
    assert!(state.commit_query_results(authoritative_query_token, 0, 0, Vec::new()));
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("authoritative empty query should replace the revoked local row");
    };
    assert!(results.is_empty());

    state.begin_session("reopen-after-refresh".to_string());
    state.query = "payroll".to_string();
    assert!(
        state.show_query_seed(),
        "the accepted empty response should remain an authoritative query-cache hit"
    );
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("the authoritative empty response should remain loaded");
    };
    assert!(results.is_empty());
    let reopened_query_token = state.begin_request();
    assert!(state
        .local_result_ids_for_query(reopened_query_token)
        .is_empty());
}

#[test]
fn recents_refresh_preserves_the_instant_query_seed_while_expiring_exclusions() {
    let mut cached = search_result("cached", "Payroll", None);
    cached.match_snippet = Some("Payroll body match".to_string());
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(Vec::new()),
    );
    state.query = "payroll".to_string();
    let initial_token = state.begin_request();
    assert!(state.local_result_ids_for_query(initial_token).is_empty());
    assert!(state.commit_query_results(initial_token, 2, 1, vec![cached.clone()]));

    let refresh_token = state
        .begin_recents_refresh(true)
        .0
        .expect("start recents refresh");
    let pre_refresh_query_token = state.begin_request();
    assert_eq!(
        state.local_result_ids_for_query(pre_refresh_query_token),
        vec!["cached"]
    );
    state.mark_query_request_dispatched(pre_refresh_query_token);
    assert_eq!(
        state.commit_recents_refresh(refresh_token, Vec::new(), true),
        NotionSearchRecentsCommit::RequeryImmediately
    );
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("the prior query should remain visible while it is revalidated");
    };
    assert_eq!(results.as_ref(), std::slice::from_ref(&cached));
    assert_eq!(
        results[0].match_snippet.as_deref(),
        Some("Payroll body match"),
        "the preserved query seed must retain response-only snippet metadata"
    );

    let revalidation_token = state.begin_request();
    assert!(state
        .local_result_ids_for_query(revalidation_token)
        .is_empty());
}

#[test]
fn a_partial_response_makes_its_verified_rows_safe_for_local_exclusions() {
    let result = search_result("matching", "Matching page", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(Vec::new()),
    );
    state.query = "matching".to_string();

    let partial_token = state.begin_request();
    assert!(state.local_result_ids_for_query(partial_token).is_empty());
    assert!(state.commit_query_results(partial_token, 2, 1, vec![result.clone()]));
    state.begin_session("after-partial".to_string());
    state.query = "matching".to_string();
    let verified_token = state.begin_request();
    assert_eq!(
        state.local_result_ids_for_query(verified_token),
        vec!["matching"]
    );
}

#[test]
fn authoritative_empty_query_can_learn_new_results_without_an_exclusion() {
    let new_result = search_result("new", "Matching new", None);
    let mut state = NotionSearchState::default();
    state.query = "matching".to_string();
    let empty_token = state.begin_request();
    assert!(state.local_result_ids_for_query(empty_token).is_empty());
    assert!(state.commit_query_results(empty_token, 0, 0, Vec::new()));

    state.begin_session("after-empty".to_string());
    state.query = "matching".to_string();
    let changed_token = state.begin_request();
    assert!(state.local_result_ids_for_query(changed_token).is_empty());
    assert!(state.commit_query_results(changed_token, 1, 1, vec![new_result]));

    state.begin_session("after-change".to_string());
    state.query = "matching".to_string();
    let repeat_token = state.begin_request();
    assert_eq!(state.local_result_ids_for_query(repeat_token), vec!["new"]);
}
