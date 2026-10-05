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

struct ColdQueryPages {
    exact: PageShellSearchResult,
    august_15: PageShellSearchResult,
    playground: PageShellSearchResult,
    fixture: PageShellSearchResult,
    august_6: PageShellSearchResult,
    top_of_mind: PageShellSearchResult,
    robot: PageShellSearchResult,
    machine_identifier: PageShellSearchResult,
    acmemate: PageShellSearchResult,
    trailing_space: PageShellSearchResult,
}

fn cold_query_pages() -> ColdQueryPages {
    let mut top_of_mind = search_result(
        "top-of-mind",
        "Top of mind",
        Some("Get acme working on Sam's laptop"),
    );
    top_of_mind.match_snippet = Some("Get acme working on Sam's laptop".to_string());
    ColdQueryPages {
        exact: search_result("exact", "Acme", None),
        august_15: search_result("august-15", "Acme Notion parity scratch 2026-08-15", None),
        playground: search_result("playground", "Acme Playground", None),
        fixture: search_result("fixture", "Acme parity fixture - delete me", None),
        august_6: search_result("august-6", "Acme parity scratch 2026-08-06", None),
        top_of_mind,
        robot: search_result(
            "robot",
            "Weekly Metrics Tracking",
            Some("outputs for acme-ops"),
        ),
        machine_identifier: search_result(
            "machine",
            "ACME_PARITY_TEMP_20260805_AACME_NET_TEMP_20260805_BACME_PARITY",
            None,
        ),
        acmemate: search_result("acmemate", "Acmemate", None),
        trailing_space: search_result("trailing-space", "Acme ", None),
    }
}

fn cold_query_recent_pages(pages: &ColdQueryPages) -> Vec<RecentPageResult> {
    let mut recents = (0..3)
        .map(|index| {
            recent_result(
                search_result(&format!("unrelated-{index}"), "Unrelated", None),
                11_000 - index,
            )
        })
        .collect::<Vec<_>>();
    recents.push(recent_result(pages.august_15.clone(), 10_000));
    recents.extend((4..14).map(unrelated_recent));
    recents.push(recent_result(pages.fixture.clone(), 9_000));
    recents.push(recent_result(pages.august_6.clone(), 8_000));
    recents.extend((16..19).map(unrelated_recent));
    recents.push(recent_result(
        search_result("playground", "Symphony", None),
        7_000,
    ));
    recents.extend((20..26).map(unrelated_recent));
    recents.push(recent_result(pages.machine_identifier.clone(), 6_000));
    recents
}

fn unrelated_recent(index: u64) -> RecentPageResult {
    recent_result(
        search_result(&format!("unrelated-{index}"), "Unrelated", None),
        10_000 - index,
    )
}

fn cold_query_state(pages: &ColdQueryPages) -> NotionSearchState {
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(cold_query_recent_pages(pages)),
    );
    let mut local_search_cache = QuickFindLocalSearchCache::default();
    local_search_cache.add_provisional_pages([pages.exact.clone()]);
    state.prepare_local_search_cache(local_search_cache);
    state.query = "acme".to_string();
    assert!(state.show_query_seed());
    state
}

#[test]
fn cold_query_bootstraps_notions_three_local_rows_then_preserves_server_order() {
    let pages = cold_query_pages();
    let mut state = cold_query_state(&pages);

    let mut exact_server = pages.exact.clone();
    exact_server.match_snippet = Some("Server body mentioning acme".to_string());
    let mut august_15_server = pages.august_15.clone();
    august_15_server.match_snippet = Some("Another server body mentioning acme".to_string());
    let token = state.begin_request();
    assert!(
        state.local_result_ids_for_query(token).is_empty(),
        "a partial bootstrap/recents index must not perturb the first server query"
    );
    assert!(state.commit_query_results(
        token,
        63,
        10,
        vec![
            pages.playground.clone(),
            august_15_server,
            pages.fixture.clone(),
            pages.august_6.clone(),
            pages.top_of_mind.clone(),
            exact_server,
            pages.robot.clone(),
            pages.machine_identifier.clone(),
            pages.acmemate.clone(),
            pages.trailing_space.clone(),
        ],
    ));
    let expected_ids = cold_query_expected_ids(&pages);
    assert_loaded_result_ids(&state, &expected_ids);
    let NotionSearchResultsState::Loaded { total, results, .. } = &state.results else {
        panic!("cold blended query should load");
    };
    assert_eq!(
        *total, 60,
        "an unindexed response reconciles the three verified local rows to Notion's post-exclusion count"
    );
    assert_eq!(results[0].match_snippet, None);
    assert_eq!(results[1].match_snippet, None);
    assert_eq!(
        results[5].match_snippet.as_deref(),
        Some("Get acme working on Sam's laptop")
    );

    state.begin_session("repeat-ranked-query".to_string());
    state.query = "acme".to_string();
    assert!(state.show_query_seed());
    assert_loaded_result_ids(&state, &expected_ids);
    let repeat_token = state.begin_request();
    assert_eq!(
        state.local_result_ids_for_query(repeat_token),
        vec!["exact", "august-15", "playground"]
    );
}

fn cold_query_expected_ids(pages: &ColdQueryPages) -> Vec<&str> {
    vec![
        pages.exact.block_id.as_str(),
        pages.august_15.block_id.as_str(),
        pages.playground.block_id.as_str(),
        pages.fixture.block_id.as_str(),
        pages.august_6.block_id.as_str(),
        pages.top_of_mind.block_id.as_str(),
        pages.robot.block_id.as_str(),
        pages.machine_identifier.block_id.as_str(),
        pages.acmemate.block_id.as_str(),
        pages.trailing_space.block_id.as_str(),
    ]
}
