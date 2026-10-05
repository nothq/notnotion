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
fn stale_query_session_and_post_close_completions_preserve_current_results(
    cx: &mut TestAppContext,
) {
    let current = vec![search_result(
        "current-result",
        "Current result",
        Some("Current body match"),
    )];
    let stale_query = vec![search_result("stale-query", "Stale query", None)];
    let stale_session = vec![search_result("stale-session", "Stale session", None)];
    let post_close = vec![search_result("post-close", "Post close", None)];
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &current, HashMap::new());
    let (_window, surface) = open_quick_find_app(cx, api);

    surface.update(cx, |surface, cx| {
        let mut harness = QuickFindSurfaceHarness::new(surface, cx);
        let current_token =
            complete_queries_out_of_order(&mut harness, &current, stale_query, stale_session);
        complete_query_after_close(&mut harness, current_token, post_close, &current);
    });
    cx.run_until_parked();
}

fn complete_queries_out_of_order(
    harness: &mut QuickFindSurfaceHarness<'_, '_, '_>,
    current: &[PageShellSearchResult],
    stale_query: Vec<PageShellSearchResult>,
    stale_session: Vec<PageShellSearchResult>,
) -> NotionSearchRequestToken {
    let surface = &mut harness.surface;
    surface.notion_chrome.notion_search_open = true;
    surface
        .notion_search
        .begin_session("old-session".to_string());
    surface.notion_search.query = "old session query".to_string();
    let stale_session_token = surface.notion_search.begin_request();
    surface
        .notion_search
        .begin_session("current-session".to_string());
    surface.notion_search.query = "older query".to_string();
    let stale_query_token = surface.notion_search.begin_request();
    surface.notion_search.query = "current query".to_string();
    let current_token = surface.notion_search.begin_request();
    harness.complete_query(current_token, current.to_vec());
    harness.complete_query(stale_query_token, stale_query);
    harness.complete_query(stale_session_token, stale_session);
    assert_loaded_results(harness.surface, current.len() as u32, current);
    current_token
}

fn complete_query_after_close(
    harness: &mut QuickFindSurfaceHarness<'_, '_, '_>,
    current_token: NotionSearchRequestToken,
    post_close: Vec<PageShellSearchResult>,
    current: &[PageShellSearchResult],
) {
    harness
        .surface
        .apply_quick_find_action(QuickFindAction::Close, harness.cx);
    harness.complete_query(current_token, post_close);
    assert!(!harness.surface.notion_chrome.notion_search_open);
    assert_loaded_results(harness.surface, current.len() as u32, current);
}
