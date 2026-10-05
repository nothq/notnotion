use crate::ui::board_workspace::PageMutationAction;
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
fn page_mutation_requeries_without_stale_exclusion_and_accepts_authoritative_removal() {
    let (mut state, page) = state_with_active_page_mutation();
    assert_dirty_page_stays_hidden(&mut state, &page);
    assert_mutation_finish_accepts_authoritative_removal(&mut state);
}

fn state_with_active_page_mutation() -> (NotionSearchState, PageShellSearchResult) {
    let compact = "00000000000000000000000000000004";
    let dashed = "00000000-0000-0000-0000-000000000004";
    let page = search_result(compact, "Payroll", Some("Payroll body match"));
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(vec![recent_result(page.clone(), 100)]),
    );
    let recents_token = state
        .begin_recents_refresh(false)
        .0
        .expect("start a pre-mutation recents refresh");
    state.prepare_local_search_cache(indexed_local_search_cache("payroll", vec![page.clone()]));
    state.query = "payroll".to_string();
    assert!(state.show_query_seed());
    let pre_mutation_query_token = state.begin_request();
    assert_eq!(
        state.local_result_ids_for_query(pre_mutation_query_token),
        vec![compact]
    );
    state.cache_preview(
        compact.to_string(),
        LoadedCardPage::new(card_page_for_result(&page)),
    );
    let preview_token = state.begin_preview_request(compact.to_string());
    state.preview = NotionSearchPreviewState::Loading {
        block_id: compact.into(),
        request_token: preview_token,
    };

    assert!(state.begin_page_mutation("page-lane", dashed));
    assert!(
        !state.begin_page_mutation("page-lane", compact),
        "normalized aliases must make mutation begin idempotent"
    );

    assert!(!state.request_is_current(pre_mutation_query_token));
    assert!(!state.recents_refresh_is_current(recents_token));
    assert_eq!(state.pending_preview_request(compact), None);
    assert!(state.cached_preview(compact).is_none());
    assert!(matches!(state.preview, NotionSearchPreviewState::Idle));

    (state, page)
}

fn assert_dirty_page_stays_hidden(state: &mut NotionSearchState, page: &PageShellSearchResult) {
    let compact = "00000000000000000000000000000004";
    let during_mutation_query_token = state.begin_request();
    assert!(state
        .local_result_ids_for_query(during_mutation_query_token)
        .is_empty());
    assert!(state.commit_query_results(during_mutation_query_token, 1, 1, vec![page.clone()]));
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("the response received during the mutation should settle");
    };
    assert!(
        results.is_empty(),
        "a response received while P is dirty must not restore it"
    );
    let during_mutation_recents_token = state
        .begin_recents_refresh(false)
        .0
        .expect("start a recents request during the mutation");
    assert_eq!(
        state.commit_recents_refresh(
            during_mutation_recents_token,
            vec![recent_result(page.clone(), 200)],
            false,
        ),
        NotionSearchRecentsCommit::Ignored
    );
    state.query.clear();
    let (_, showing_recents) = state.begin_recents_refresh(true);
    assert!(showing_recents);
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("filtered recents should settle as an empty list");
    };
    assert!(
        results.is_empty(),
        "a recents response received while P is dirty must not restore it"
    );
    state.cache_preview(
        compact.to_string(),
        LoadedCardPage::new(card_page_for_result(&page)),
    );
    assert!(
        state.cached_preview(compact).is_none(),
        "a preview received while P is dirty must not become visible"
    );
}

fn assert_mutation_finish_accepts_authoritative_removal(state: &mut NotionSearchState) {
    let compact = "00000000000000000000000000000004";
    assert_eq!(state.finish_page_mutation_lane("page-lane"), vec![compact]);
    assert!(!state.page_mutation_is_dirty(compact));
    state.query = "payroll".to_string();
    let same_query_token = state.begin_request();
    assert!(
        state
            .local_result_ids_for_query(same_query_token)
            .is_empty(),
        "the mutated page must go back to the server instead of remaining a local exclusion"
    );
    assert!(state.commit_query_results(same_query_token, 0, 0, Vec::new()));
    let NotionSearchResultsState::Loaded { total, results, .. } = &state.results else {
        panic!("the empty authoritative response should be accepted");
    };
    assert_eq!(*total, 0);
    assert!(
        results.is_empty(),
        "the authoritative response must remove P"
    );
}

#[test]
fn dirty_page_identities_are_scoped_to_their_mutation_lane() {
    let compact = "00000000000000000000000000000004";
    let dashed = "00000000-0000-0000-0000-000000000004";
    let mut state = NotionSearchState::default();

    assert!(state.begin_page_mutation("container-a", "container-a"));
    assert!(state.begin_page_mutation("container-a", dashed));
    assert!(state.begin_page_mutation("container-b", compact));
    assert!(state.page_mutation_is_dirty(compact));
    state.begin_session("reopened-while-dirty".to_string());
    assert!(state.page_mutation_is_dirty(dashed));

    let mut completed = state.finish_page_mutation_lane("container-a");
    completed.sort();
    assert_eq!(completed, vec![compact, "container-a"]);
    assert!(
        state.page_mutation_is_dirty(dashed),
        "another active lane must keep the same normalized page hidden"
    );

    assert_eq!(
        state.finish_page_mutation_lane("container-b"),
        vec![compact]
    );
    assert!(!state.page_mutation_is_dirty(compact));
}

#[gpui::test]
fn final_page_mutation_boundary_reinvalidates_nested_page_and_requeries(cx: &mut TestAppContext) {
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
        surface
            .notion_search
            .begin_session("mutation-refresh".to_string());
        surface.notion_search.query = "payroll".to_string();

        for block_id in ["container", "nested-page", "nested-page"] {
            surface.dispatch_page_mutation_action(
                PageMutationAction::BeginSearchMutation {
                    lane_page_id: "container".to_string(),
                    block_id: block_id.to_string(),
                },
                cx,
            );
        }
        surface.dispatch_page_mutation_action(
            PageMutationAction::PersistSearchMutation {
                lane_page_id: "container".to_string(),
                workspace_api: api.clone(),
            },
            cx,
        );
        surface.dispatch_page_mutation_action(
            PageMutationAction::FinishSearchMutation {
                lane_page_id: "container".to_string(),
            },
            cx,
        );

        assert!(!surface.notion_search.page_mutation_is_dirty("container"));
        assert!(!surface.notion_search.page_mutation_is_dirty("nested-page"));
    });
    cx.run_until_parked();

    assert_mutation_refresh_requests(api.as_ref());
    cx.read_entity(&surface, |surface, _| {
        assert_empty_mutation_refresh(&surface.notion_search.results)
    });
}

#[test]
fn authoritative_same_index_replacement_resets_preview_scroll() {
    let previous = search_result("previous", "Previous result", None);
    let replacement = search_result("replacement", "Replacement result", None);
    let mut state = NotionSearchState::default();
    state.query = "result".to_string();

    let previous_token = state.begin_request();
    assert!(state.commit_query_results(previous_token, 1, 1, vec![previous]));
    state
        .preview_scroll_handle
        .set_offset(point(px(0.0), px(-64.0)));
    assert_eq!(
        state.preview_scroll_handle.offset(),
        point(px(0.0), px(-64.0))
    );

    let replacement_token = state.begin_request();
    assert!(state.commit_query_results(replacement_token, 1, 1, vec![replacement]));
    assert_eq!(
        state.preview_scroll_handle.offset(),
        point(px(0.0), px(0.0)),
        "a different selected page at the same row must start at the top"
    );
}

#[test]
fn complete_query_membership_omits_stale_partial_overlap_from_repeat_exclusions() {
    let stale = search_result("stale", "Payroll stale", None);
    let authoritative = search_result("authoritative", "Payroll current", None);
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(Vec::new()),
    );
    state.query = "payroll".to_string();

    let partial_token = state.begin_request();
    assert!(state.local_result_ids_for_query(partial_token).is_empty());
    assert!(state.commit_query_results(partial_token, 2, 1, vec![stale]));

    let refresh_token = state
        .begin_recents_refresh(true)
        .0
        .expect("start query-authority refresh");
    assert_eq!(
        state.commit_recents_refresh(refresh_token, Vec::new(), true),
        NotionSearchRecentsCommit::RequeryImmediately
    );

    state.begin_session("complete-query".to_string());
    state.query = "payroll".to_string();
    let complete_token = state.begin_request();
    assert!(state.local_result_ids_for_query(complete_token).is_empty());
    assert!(state.commit_query_results(complete_token, 1, 1, vec![authoritative]));

    state.begin_session("repeat-query".to_string());
    state.query = "payroll".to_string();
    assert!(state.show_query_seed());
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("the authoritative repeat seed should load");
    };
    assert_eq!(
        results
            .iter()
            .map(|result| result.block_id.as_str())
            .collect::<Vec<_>>(),
        vec!["authoritative"]
    );
    let repeat_token = state.begin_request();
    assert_eq!(
        state.local_result_ids_for_query(repeat_token),
        vec!["authoritative"]
    );
}

fn assert_empty_mutation_refresh(results: &NotionSearchResultsState) {
    let NotionSearchResultsState::Loaded { total, results, .. } = results else {
        panic!("the post-mutation query should complete");
    };
    assert_eq!(*total, 0);
    assert!(results.is_empty());
}

fn assert_mutation_refresh_requests(api: &RecordingQuickFindApi) {
    let (mut in_memory, mut persisted) = api.page_mutation_invalidations();
    in_memory.sort();
    persisted.sort();
    assert_eq!(in_memory, vec!["container", "nested-page"]);
    assert_eq!(
        persisted,
        vec!["container", "container", "nested-page", "nested-page"]
    );
    let requests = api.search_requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].excluded_block_ids.is_empty());
}
