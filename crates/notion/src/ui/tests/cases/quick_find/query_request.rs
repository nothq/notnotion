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
fn query_sends_ranked_local_exclusions_and_fifty_exact_boosts_then_merges_by_relevance(
    cx: &mut TestAppContext,
) {
    let fixture = ranked_query_fixture();
    let surface = run_ranked_query(&fixture, cx);
    assert_ranked_query_request(&fixture);
    assert_ranked_query_results(&fixture, &surface, cx);
}

struct RankedQueryFixture {
    api: Arc<RecordingQuickFindApi>,
    recents: Vec<RecentPageResult>,
    local_pages: Vec<PageShellSearchResult>,
    remote_pages: Vec<PageShellSearchResult>,
}

fn ranked_query_fixture() -> RankedQueryFixture {
    let recents = (0..52)
        .map(|index| {
            let title = if index == 0 {
                "Payroll scratch".to_string()
            } else if index == 1 {
                "Payroll".to_string()
            } else {
                format!("Other recent {index}")
            };
            recent_result(
                search_result(&format!("local-{index:02}"), &title, None),
                10_000 - index,
            )
        })
        .collect::<Vec<_>>();
    let local_pages = vec![recents[1].page.clone(), recents[0].page.clone()];
    let remote_pages = (0..25)
        .map(|index| {
            search_result(
                &format!("remote-{index:02}"),
                &format!("Remote result {index}"),
                None,
            )
        })
        .collect::<Vec<_>>();
    let mut server_results = vec![
        search_result("local-01", "Server duplicate B", None),
        remote_pages[0].clone(),
        search_result("local-00", "Server duplicate A", None),
    ];
    server_results.extend(remote_pages.iter().skip(1).cloned());
    let api = RecordingQuickFindApi::new(
        Vec::new(),
        vec![SearchWorkspaceResult {
            total: server_results.len() as u32,
            consumed_result_count: server_results.len() as u32,
            results: server_results,
        }],
        &[],
        HashMap::new(),
    );
    api.set_persisted_recents(Some("workspace-a"), recents.clone());
    api.set_local_search_cache(indexed_local_search_cache("payroll", local_pages.clone()));
    api.set_recent_responses(vec![recents.clone()]);
    RankedQueryFixture {
        api,
        recents,
        local_pages,
        remote_pages,
    }
}

fn run_ranked_query(fixture: &RankedQueryFixture, cx: &mut TestAppContext) -> Entity<SurfaceState> {
    let (_window, surface) = open_quick_find_app(cx, fixture.api.clone());

    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.run_until_parked();
    surface.update(cx, |surface, _| {
        surface
            .notion_search
            .replace_local_search_cache_for_test(indexed_local_search_cache(
                "payroll",
                fixture.local_pages.clone(),
            ));
    });
    surface.update(cx, |surface, cx| {
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("payroll".to_string()), cx)
    });
    cx.executor().advance_clock(Duration::from_millis(225));
    cx.run_until_parked();
    surface
}

fn assert_ranked_query_request(fixture: &RankedQueryFixture) {
    let requests = fixture.api.search_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].excluded_block_ids,
        vec!["local-01".to_string(), "local-00".to_string()]
    );
    assert_eq!(
        requests[0].recent_pages_for_boosting,
        fixture
            .recents
            .iter()
            .take(50)
            .map(RecentPageResult::recent_page_visit)
            .collect::<Vec<RecentPageVisit>>()
    );
}

fn assert_ranked_query_results(
    fixture: &RankedQueryFixture,
    surface: &Entity<SurfaceState>,
    cx: &TestAppContext,
) {
    let mut expected = fixture.local_pages.clone();
    expected.extend(fixture.remote_pages.iter().take(18).cloned());
    cx.read_entity(surface, |surface, _| {
        assert_loaded_results(surface, 27, &expected);
        assert_eq!(surface.notion_search.list_row_index_for_result(0), Some(1));
        assert_eq!(
            surface.notion_search.list_row_index_for_result(19),
            Some(20)
        );
        assert_eq!(surface.notion_search.list_row_index_for_result(20), None);
    });
}
