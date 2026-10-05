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
fn recency_headers_do_not_enter_selection_or_break_result_to_list_row_mapping(
    cx: &mut TestAppContext,
) {
    let now = unix_millis_now();
    let today = search_result("today", "Today", None);
    let older = search_result("older", "Older", None);
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &[], HashMap::new());
    let recents = vec![
        recent_result(today, now),
        recent_result(older, now.saturating_sub(40 * 86_400_000)),
    ];
    api.set_persisted_recents(Some("workspace-a"), recents.clone());
    api.set_recent_responses(vec![recents]);
    let (window, surface) = open_quick_find_app(cx, api);
    open_search_and_focus_input(&window, &surface, cx);

    cx.read_entity(&surface, |surface, _| {
        assert_eq!(
            surface.notion_search.list_row_recency_header(0),
            Some(("Today", "today"))
        );
        assert_eq!(
            surface.notion_search.recent_result_index_for_list_row(1),
            Some(0)
        );
        assert_eq!(
            surface.notion_search.list_row_recency_header(2),
            Some(("Older", "older"))
        );
        assert_eq!(
            surface.notion_search.recent_result_index_for_list_row(3),
            Some(1)
        );
        assert_eq!(surface.notion_search.list_row_index_for_result(0), Some(1));
        assert_eq!(surface.notion_search.list_row_index_for_result(1), Some(3));
    });
    dispatch_keystroke(&window, "down", cx);
    assert_selected_index(&surface, 1, cx);

    let mut visual_cx = VisualTestContext::from_window(window.into(), cx);
    visual_cx.refresh().expect("draw Quick Find recent rows");
    visual_cx.run_until_parked();
    let today_header_bounds = visual_cx
        .debug_bounds("notion-search-recency-today")
        .expect("Today recency header should render");
    assert_eq!(today_header_bounds.size.height.as_f32(), 34.0);
}

#[gpui::test]
fn arrow_selection_clamps_and_enter_opens_the_selected_result(cx: &mut TestAppContext) {
    let results = navigation_results();
    let target = results[2].target_board_url.clone();
    let api = RecordingQuickFindApi::new(
        vec![results.clone()],
        Vec::new(),
        &[],
        HashMap::from([(target.clone(), interaction_test_board())]),
    );
    let (window, surface) = open_quick_find_app(cx, api.clone());
    open_search_and_focus_input(&window, &surface, cx);

    dispatch_keystroke(&window, "up", cx);
    assert_selected_index(&surface, 0, cx);
    dispatch_keystroke(&window, "down", cx);
    dispatch_keystroke(&window, "down", cx);
    dispatch_keystroke(&window, "down", cx);
    assert_selected_index(&surface, 2, cx);

    dispatch_keystroke(&window, "enter", cx);
    cx.read_entity(&surface, |surface, _| {
        assert!(!surface.notion_chrome.notion_search_open);
    });
    assert!(api.workspace_requests().is_empty());
    cx.run_until_parked();
    assert_eq!(api.workspace_requests(), vec![target]);
    assert_eq!(cx.opened_url(), None);
}

#[gpui::test]
fn cmd_enter_routes_through_the_root_records_the_visit_and_reopens_from_local_cache(
    cx: &mut TestAppContext,
) {
    let mut results = navigation_results();
    results[1].match_snippet = Some("Transient matching body".to_string());
    results[1].editor_display_name = Some("Ada Lovelace".to_string());
    results[1].edited_label = Some("Edited Jan 1".to_string());
    results[1].badges = vec![
        PageShellSearchBadge::CurrentPage,
        PageShellSearchBadge::Database,
    ];
    let target = results[1].target_board_url.clone();
    let selected = results[1].clone();
    let api = RecordingQuickFindApi::new(vec![results], Vec::new(), &[], HashMap::new());
    let (window, surface) = open_quick_find_app(cx, api.clone());
    open_search_and_focus_input(&window, &surface, cx);

    dispatch_keystroke(&window, "down", cx);
    assert_selected_index(&surface, 1, cx);
    dispatch_keystroke(&window, "cmd-enter", cx);

    cx.read_entity(&surface, |surface, _| {
        assert!(!surface.notion_chrome.notion_search_open);
    });
    assert_eq!(cx.opened_url().as_deref(), Some(target.as_str()));
    assert!(api.workspace_requests().is_empty());

    cx.run_until_parked();
    let recorded = api.recorded_recent_visits();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].page.block_id, selected.block_id);
    assert_eq!(recorded[0].page.match_snippet, None);
    assert_eq!(recorded[0].page.editor_display_name, None);
    assert_eq!(recorded[0].page.edited_label, None);
    assert_eq!(
        recorded[0].page.badges,
        vec![PageShellSearchBadge::Database]
    );
    assert!(recorded[0].visited_at_unix_millis > 0);

    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.read_entity(&surface, |surface, _| {
        let NotionSearchResultsState::Loaded { results, .. } = &surface.notion_search.results
        else {
            panic!("recorded recents should reopen synchronously");
        };
        assert_eq!(results[0].block_id, selected.block_id);
    });
    assert_eq!(
        api.recent_requests().len(),
        1,
        "the local visit must retain the completed cache's freshness"
    );
}
