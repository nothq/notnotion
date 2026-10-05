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
fn cached_current_page_uses_the_exact_active_view_url() {
    let current_id = "00000000000000000000000000000004";
    let current_url =
        "https://www.notion.so/acme/Current-00000000000000000000000000000004?v=view-1";
    let current = search_result(current_id, "Current", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        Some("00000000-0000-0000-0000-000000000004".to_string()),
        Some(current_url.to_string()),
        Some(vec![recent_result(current, 100)]),
    );

    let (_, showing_cached_recents) = state.begin_recents_refresh(true);
    assert!(showing_cached_recents);
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("cached current page should render");
    };
    assert_eq!(results[0].target_board_url, current_url);
    assert_eq!(results[0].badges, vec![PageShellSearchBadge::CurrentPage]);
}

#[gpui::test]
fn stale_recents_neither_replace_query_results_nor_warm_the_cache(cx: &mut TestAppContext) {
    let current = vec![search_result(
        "current-query",
        "Current query result",
        Some("Current highlight"),
    )];
    let stale = vec![search_result("stale-recent", "Stale recent", None)];
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &current, HashMap::new());
    let (_window, surface) = open_quick_find_app(cx, api);

    surface.update(cx, |surface, cx| {
        let mut harness = QuickFindSurfaceHarness::new(surface, cx);
        complete_current_query_then_stale_recents(&mut harness, &current, &stale);
    });
    cx.run_until_parked();
}

fn complete_current_query_then_stale_recents(
    harness: &mut QuickFindSurfaceHarness<'_, '_, '_>,
    current: &[PageShellSearchResult],
    stale: &[PageShellSearchResult],
) {
    let surface = &mut harness.surface;
    surface.notion_chrome.notion_search_open = true;
    surface
        .notion_search
        .begin_session("recents-session".to_string());
    surface.notion_search.prepare_recents_cache(
        Some("stale-workspace".to_string()),
        None,
        None,
        None,
    );
    let stale_token = surface
        .notion_search
        .begin_recents_refresh(true)
        .0
        .expect("start stale recents request");
    surface.notion_search.prepare_recents_cache(
        Some("current-workspace".to_string()),
        None,
        None,
        None,
    );
    surface.notion_search.query = "current query".to_string();
    let current_token = surface.notion_search.begin_request();
    harness.complete_query(current_token, current.to_vec());
    harness.complete_recents(stale_token, recent_page_results(stale.to_vec()));
    let surface = &mut harness.surface;
    assert_loaded_results(surface, current.len() as u32, current);
    assert_eq!(surface.notion_search.query, "current query");
    surface
        .notion_search
        .begin_session("next-session".to_string());
    let (_, showing_cached_recents) = surface.notion_search.begin_recents_refresh(true);
    assert!(
        !showing_cached_recents,
        "a stale completion must not populate the recents cache"
    );
}

#[gpui::test]
fn some_to_none_cache_scope_transition_clears_recents_query_and_preview_state() {
    let recent = search_result("recent-a", "Recent A", None);
    let query_result = search_result("query-a", "Query A", None);
    let preview_result = search_result("preview-a", "Preview A", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(vec![recent_result(recent, 10)]),
    );
    state.query = "query-only".to_string();
    let query_token = state.begin_request();
    assert!(state.commit_query_results(query_token, 1, 1, vec![query_result]));
    state.cache_preview(
        preview_result.block_id.clone(),
        LoadedCardPage::new(card_page_for_result(&preview_result)),
    );

    state.prepare_recents_cache(None, None, None, None);
    state.query = "query-only".to_string();

    assert!(!state.show_query_seed());
    assert!(state.cached_preview(&preview_result.block_id).is_none());
    state.query.clear();
    let (refresh_token, showing_cached_recents) = state.begin_recents_refresh(true);
    assert!(refresh_token.is_some());
    assert!(!showing_cached_recents);
}
