use crate::ui::search::action::QuickFindCompletion;

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

#[test]
fn fresh_authoritative_recents_ignore_stale_same_space_runtime_snapshots() {
    let stale = search_result("stale", "Stale", None);
    let fresh = search_result("fresh", "Fresh", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(vec![recent_result(stale.clone(), 100)]),
    );
    let refresh_token = state
        .begin_recents_refresh(false)
        .0
        .expect("start authoritative refresh");
    assert_eq!(
        state.commit_recents_refresh(
            refresh_token,
            vec![recent_result(fresh.clone(), 200)],
            false,
        ),
        NotionSearchRecentsCommit::Ignored
    );
    state.expire_recents_freshness_for_test();

    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(vec![recent_result(stale.clone(), 100)]),
    );
    state.prepare_local_search_cache(indexed_local_search_cache("stale", vec![stale]));
    state.begin_session("after-runtime-swap".to_string());
    let (_, showing_cached_recents) = state.begin_recents_refresh(true);
    assert!(showing_cached_recents);
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("fresh authoritative recents should remain loaded");
    };
    assert_eq!(results.as_ref(), &[fresh]);

    state.query = "stale".to_string();
    let query_token = state.begin_request();
    assert!(state.local_result_ids_for_query(query_token).is_empty());
}

#[gpui::test]
fn accepted_recents_revocation_requeries_the_visible_query(cx: &mut TestAppContext) {
    let stale = search_result("stale", "Payroll", None);
    let api = RecordingQuickFindApi::new(
        Vec::new(),
        vec![SearchWorkspaceResult {
            total: 0,
            consumed_result_count: 0,
            results: Vec::new(),
        }],
        &[],
        HashMap::new(),
    );
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| {
        surface.notion_chrome.notion_search_open = true;
        surface.notion_search.prepare_recents_cache(
            Some("revocation-test".to_string()),
            None,
            None,
            Some(vec![recent_result(stale, 100)]),
        );
        let refresh_token = surface
            .notion_search
            .begin_recents_refresh(true)
            .0
            .expect("start authoritative refresh");
        surface.notion_search.query = "payroll".to_string();
        let old_query_token = surface.notion_search.begin_request();
        assert!(surface
            .notion_search
            .commit_query_results(old_query_token, 0, 0, Vec::new()));

        surface.finish_quick_find_completion(
            QuickFindCompletion::recents(
                refresh_token,
                Ok(LoadRecentPagesResult {
                    results: Vec::new(),
                }),
            ),
            cx,
        );
        let NotionSearchResultsState::Loaded { results, .. } = &surface.notion_search.results
        else {
            panic!("the accepted empty query cache should remain visible during revalidation");
        };
        assert!(results.is_empty());
    });
    assert!(api.search_requests().is_empty());

    cx.run_until_parked();
    assert_eq!(api.search_requests().len(), 1);
    cx.read_entity(&surface, |surface, _| {
        let NotionSearchResultsState::Loaded { results, .. } = &surface.notion_search.results
        else {
            panic!("replacement query should complete");
        };
        assert!(results.is_empty());
    });
}
