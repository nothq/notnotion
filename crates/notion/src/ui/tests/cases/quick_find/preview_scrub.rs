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
fn a_b_a_preview_scrub_does_not_request_the_abandoned_result(cx: &mut TestAppContext) {
    let page_a = search_result("preview-a", "Preview A", None);
    let page_b = search_result("preview-b", "Preview B", None);
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &[], HashMap::new());
    let recents = vec![
        recent_result(page_a.clone(), 20),
        recent_result(page_b.clone(), 10),
    ];
    api.set_persisted_recents(Some("workspace-a"), recents.clone());
    api.set_recent_responses(vec![recents]);
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| {
        surface.activate_notion_search(cx);
        surface.apply_quick_find_action(QuickFindAction::SelectResult(1), cx);
        surface.apply_quick_find_action(QuickFindAction::SelectResult(0), cx);
    });
    assert_selected_index(&surface, 0, cx);

    cx.run_until_parked();
    cx.read_entity(&surface, |surface, _| {
        match &surface.notion_search.preview {
            NotionSearchPreviewState::Loaded { block_id, page } => {
                assert_eq!(block_id.as_ref(), page_a.block_id.as_str());
                assert_eq!(page.data.page.block_id, page_a.block_id);
            }
            _ => panic!("only the final A preview request may become visible"),
        }
    });
    assert_eq!(api.page_requests(), vec!["preview-a".to_string()]);
}

#[gpui::test]
fn preview_scrubbing_only_loads_the_result_that_remains_selected(cx: &mut TestAppContext) {
    let pages = (0..4)
        .map(|index| {
            search_result(
                &format!("preview-{index}"),
                &format!("Preview {index}"),
                None,
            )
        })
        .collect::<Vec<_>>();
    let recents = pages
        .iter()
        .enumerate()
        .map(|(index, page)| recent_result(page.clone(), 100 - index as u64))
        .collect::<Vec<_>>();
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &[], HashMap::new());
    api.set_persisted_recents(Some("workspace-a"), recents.clone());
    api.set_recent_responses(vec![recents]);
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.run_until_parked();
    assert_eq!(api.page_requests(), vec!["preview-0".to_string()]);
    surface.update(cx, |surface, cx| {
        surface.apply_quick_find_action(QuickFindAction::SelectResult(1), cx);
        surface.apply_quick_find_action(QuickFindAction::SelectResult(2), cx);
        surface.apply_quick_find_action(QuickFindAction::SelectResult(3), cx);
    });
    cx.executor().advance_clock(Duration::from_millis(74));
    cx.run_until_parked();
    assert_eq!(api.page_requests(), vec!["preview-0".to_string()]);

    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(
        api.page_requests(),
        vec!["preview-0".to_string(), "preview-3".to_string()]
    );
}

#[gpui::test]
fn same_provisional_result_across_query_keystrokes_reuses_its_preview(cx: &mut TestAppContext) {
    let page = search_result("payroll-preview", "Payroll guide", None);
    let api = RecordingQuickFindApi::new(vec![vec![page.clone()]], Vec::new(), &[], HashMap::new());
    api.set_persisted_recents(Some("workspace-a"), vec![recent_result(page.clone(), 20)]);
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| {
        surface.activate_notion_search(cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("p".to_string()), cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("pa".to_string()), cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("pay".to_string()), cx);
    });
    assert!(
        api.page_requests().is_empty(),
        "the preview worker must remain asynchronous"
    );

    cx.executor().advance_clock(Duration::from_millis(75));
    cx.run_until_parked();
    assert_eq!(api.page_requests(), vec![page.block_id.clone()]);
    cx.read_entity(&surface, |surface, _| {
        assert!(matches!(
            &surface.notion_search.preview,
            NotionSearchPreviewState::Loaded { block_id, .. }
                if block_id.as_ref() == page.block_id.as_str()
        ));
    });

    surface.update(cx, |surface, cx| {
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("payr".to_string()), cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("payro".to_string()), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        api.page_requests(),
        vec![page.block_id],
        "typing must reuse the loaded preview while the first provisional row is unchanged"
    );
}

#[gpui::test]
fn rapid_queries_only_load_the_final_distinct_provisional_preview(cx: &mut TestAppContext) {
    let general = search_result("general", "General", None);
    let page_a = search_result("query-a", "a", None);
    let page_ab = search_result("query-ab", "ab", None);
    let page_abc = search_result("query-abc", "abc", None);
    let recents = vec![
        recent_result(general, 400),
        recent_result(page_a, 300),
        recent_result(page_ab, 200),
        recent_result(page_abc.clone(), 100),
    ];
    let api = RecordingQuickFindApi::new(Vec::new(), Vec::new(), &[], HashMap::new());
    api.set_persisted_recents(Some("workspace-a"), recents.clone());
    api.set_recent_responses(vec![recents]);
    let (_window, surface) = open_quick_find_app(cx, api.clone());

    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.run_until_parked();
    assert_eq!(api.page_requests(), vec!["general".to_string()]);
    surface.update(cx, |surface, cx| {
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("a".to_string()), cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("ab".to_string()), cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("abc".to_string()), cx);
    });
    cx.executor().advance_clock(Duration::from_millis(74));
    cx.run_until_parked();
    assert_eq!(api.page_requests(), vec!["general".to_string()]);

    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert_eq!(
        api.page_requests(),
        vec!["general".to_string(), page_abc.block_id]
    );
}
