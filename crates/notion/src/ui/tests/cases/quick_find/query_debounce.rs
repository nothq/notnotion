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
fn query_debounce_title_scope_and_results_follow_the_workspace_contract(cx: &mut TestAppContext) {
    let all_content = vec![
        search_result("body-b", "Second from server", Some("payroll body match")),
        search_result("body-a", "First by title", None),
    ];
    let title_only = vec![
        search_result("title-c", "Payroll calendar", None),
        search_result("body-a", "First by title", Some("Payroll")),
    ];
    let api = query_contract_api(&all_content, &title_only);
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| {
        surface.activate_notion_search(cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("pay".to_string()), cx);
        surface
            .apply_quick_find_action(QuickFindAction::QueryChanged("  payroll  ".to_string()), cx);
    });
    cx.run_until_parked();
    assert!(api.search_requests().is_empty());

    cx.executor().advance_clock(Duration::from_millis(224));
    cx.run_until_parked();
    assert!(api.search_requests().is_empty());

    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    let search_session_id = assert_all_content_request(&api, &surface, &all_content, cx);

    surface.update(cx, |surface, cx| {
        surface.apply_quick_find_action(QuickFindAction::ToggleTitleOnly, cx);
    });
    assert_eq!(
        api.search_requests().len(),
        1,
        "the title-only request remains asynchronous"
    );
    cx.run_until_parked();

    assert_title_only_request(&api, &surface, &title_only, &search_session_id, cx);
}

fn query_contract_api(
    all_content: &[PageShellSearchResult],
    title_only: &[PageShellSearchResult],
) -> Arc<RecordingQuickFindApi> {
    let api = RecordingQuickFindApi::new(
        vec![Vec::new()],
        vec![
            SearchWorkspaceResult {
                total: 9,
                consumed_result_count: all_content.len() as u32,
                results: all_content.to_vec(),
            },
            SearchWorkspaceResult {
                total: 2,
                consumed_result_count: title_only.len() as u32,
                results: title_only.to_vec(),
            },
        ],
        &[],
        HashMap::new(),
    );
    let mut local_search = QuickFindLocalSearchCache::default();
    for scope in [
        SearchWorkspaceScope::AllContent,
        SearchWorkspaceScope::TitleOnly,
    ] {
        local_search.record_complete_query_results(
            &crate::model::quick_find_local_query_key(scope, "payroll"),
            Vec::new(),
        );
    }
    api.set_local_search_cache(local_search);
    api
}

fn assert_all_content_request(
    api: &RecordingQuickFindApi,
    surface: &Entity<SurfaceState>,
    expected: &[PageShellSearchResult],
    cx: &TestAppContext,
) -> String {
    let requests = api.search_requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.current_board_url, TEST_BOARD_URL);
    assert_eq!(request.query, "payroll");
    assert_eq!(request.scope, SearchWorkspaceScope::AllContent);
    assert_eq!(request.limit, 20);
    assert!(!request.search_session_id.is_empty());
    assert_eq!(request.flow_number, 1);
    assert!(request.recent_pages_for_boosting.is_empty());
    assert!(request.excluded_block_ids.is_empty());
    cx.read_entity(surface, |surface, _| {
        assert_loaded_results(surface, 9, expected)
    });
    request.search_session_id.clone()
}

fn assert_title_only_request(
    api: &RecordingQuickFindApi,
    surface: &Entity<SurfaceState>,
    expected: &[PageShellSearchResult],
    search_session_id: &str,
    cx: &TestAppContext,
) {
    let requests = api.search_requests();
    assert_eq!(requests.len(), 2);
    let request = &requests[1];
    assert_eq!(request.current_board_url, TEST_BOARD_URL);
    assert_eq!(request.query, "payroll");
    assert_eq!(request.scope, SearchWorkspaceScope::TitleOnly);
    assert_eq!(request.limit, 20);
    assert_eq!(request.search_session_id, search_session_id);
    assert_eq!(request.flow_number, 2);
    cx.read_entity(surface, |surface, _| {
        assert_loaded_results(surface, 2, expected)
    });
}
