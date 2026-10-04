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
fn query_pagination_counts_server_rows_without_hiding_prepended_local_matches() {
    let (mut state, server_results) = query_state_with_first_page();
    assert_expanded_query_page(&mut state, server_results);
    assert_repeated_query_uses_first_page_seed(&mut state);
}

fn query_state_with_first_page() -> (NotionSearchState, Vec<PageShellSearchResult>) {
    let local_results = (0..3)
        .map(|index| {
            recent_result(
                search_result(&format!("local-{index}"), "Matching local", None),
                100 - index,
            )
        })
        .collect();
    let server_results = (0..21)
        .map(|index| search_result(&format!("server-{index}"), "Server result", None))
        .collect::<Vec<_>>();
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(
        Some("workspace-a".to_string()),
        None,
        None,
        Some(local_results),
    );
    state.prepare_local_search_cache(indexed_local_search_cache(
        "matching",
        (0..3)
            .map(|index| search_result(&format!("local-{index}"), "Matching local", None))
            .collect(),
    ));
    state.query = "matching".to_string();
    let first_token = state.begin_request();
    assert_eq!(state.local_result_ids_for_query(first_token).len(), 3);
    assert!(state.commit_query_results(first_token, 21, 20, server_results[..20].to_vec()));
    let NotionSearchResultsState::Loaded {
        total,
        has_more,
        results,
    } = &state.results
    else {
        panic!("first query page should load");
    };
    assert_eq!(*total, 21, "the header keeps Notion's post-exclusion total");
    assert!(*has_more);
    assert_eq!(results.len(), 23);
    assert_eq!(
        results[..3]
            .iter()
            .map(|result| result.block_id.as_str())
            .collect::<Vec<_>>(),
        vec!["local-0", "local-1", "local-2"]
    );

    (state, server_results)
}

fn assert_expanded_query_page(
    state: &mut NotionSearchState,
    server_results: Vec<PageShellSearchResult>,
) {
    state.result_limit = 40;
    let final_token = state.begin_request();
    assert_eq!(state.local_result_ids_for_query(final_token).len(), 3);
    assert!(state.commit_query_results(final_token, 21, 21, server_results));
    let NotionSearchResultsState::Loaded {
        total,
        has_more,
        results,
    } = &state.results
    else {
        panic!("final query page should load");
    };
    assert_eq!(*total, 21);
    assert!(!has_more);
    assert_eq!(results.len(), 24);
}

fn assert_repeated_query_uses_first_page_seed(state: &mut NotionSearchState) {
    state.begin_session("repeat-query".to_string());
    state.query = "matching".to_string();
    assert!(state.show_query_seed());
    let NotionSearchResultsState::Loaded {
        has_more, results, ..
    } = &state.results
    else {
        panic!("expanded cached query should seed the repeated query");
    };
    assert!(*has_more);
    assert_eq!(results.len(), 23);
}

#[test]
fn pagination_uses_raw_consumed_rows_when_shaping_skips_unavailable_records() {
    let visible = search_result("visible", "Visible", None);
    let mut exhausted = NotionSearchState::default();
    exhausted.query = "visible".to_string();
    let exhausted_token = exhausted.begin_request();
    assert!(exhausted.commit_query_results(exhausted_token, 3, 3, vec![visible.clone()]));
    let NotionSearchResultsState::Loaded {
        total,
        has_more,
        results,
    } = &exhausted.results
    else {
        panic!("exhausted shaped response should load");
    };
    assert_eq!(
        *total, 2,
        "the exact-title row promoted into the local tier must keep the first and repeated headers stable"
    );
    assert!(!has_more);
    assert_eq!(results.as_ref(), &[visible.clone()]);

    let mut partial = NotionSearchState::default();
    partial.query = "visible".to_string();
    let partial_token = partial.begin_request();
    assert!(partial.commit_query_results(partial_token, 3, 2, vec![visible]));
    let NotionSearchResultsState::Loaded { has_more, .. } = &partial.results else {
        panic!("partial shaped response should load");
    };
    assert!(has_more);
}

#[test]
fn pagination_is_single_flight_and_preserves_the_list_scroll_anchor() {
    let first_page = (0..20)
        .map(|index| search_result(&format!("server-{index}"), "Matching server", None))
        .collect::<Vec<_>>();
    let expanded_page = (0..40)
        .map(|index| search_result(&format!("server-{index}"), "Matching server", None))
        .collect::<Vec<_>>();
    let mut state = NotionSearchState::default();
    state.query = "matching".to_string();
    let first_token = state.begin_request();
    assert!(state.commit_query_results(first_token, 40, 20, first_page));
    state.list_state.scroll_to(ListOffset {
        item_ix: 12,
        offset_in_item: px(7.0),
    });

    let pagination_token = state
        .begin_pagination_request(20)
        .expect("begin the first pagination request");
    assert_eq!(state.result_limit, 40);
    assert!(state.pagination_is_loading());
    assert_eq!(
        state.begin_pagination_request(20),
        None,
        "Show more must ignore a duplicate click while the request is in flight"
    );
    assert!(state.commit_query_results(pagination_token, 40, 40, expanded_page));

    assert!(!state.pagination_is_loading());
    let preserved_scroll = state.list_state.logical_scroll_top();
    assert_eq!(preserved_scroll.item_ix, 12);
    assert_eq!(preserved_scroll.offset_in_item.as_f32(), 7.0);

    let failed_pagination_token = state
        .begin_pagination_request(20)
        .expect("begin a retryable pagination request");
    assert_eq!(state.result_limit, 60);
    state.finish_query_failure(failed_pagination_token);
    assert!(!state.pagination_is_loading());
    assert_eq!(
        state.result_limit, 40,
        "a failed request must restore the last successful result limit"
    );
}

#[test]
fn local_query_relevance_ranks_an_older_exact_title_before_the_limit_cut() {
    let mut recents = (0..25)
        .map(|index| {
            recent_result(
                search_result(
                    &format!("weak-{index:02}"),
                    &format!("Acme scratch {index:02}"),
                    None,
                ),
                10_000 - index,
            )
        })
        .collect::<Vec<_>>();
    recents.push(recent_result(search_result("exact", "Acme", None), 1));
    let mut state = NotionSearchState::default();
    state.prepare_recents_cache(Some("workspace-a".to_string()), None, None, Some(recents));
    state.prepare_local_search_cache(indexed_local_search_cache(
        "acme",
        (0..25)
            .map(|index| {
                search_result(
                    &format!("weak-{index:02}"),
                    &format!("Acme scratch {index:02}"),
                    None,
                )
            })
            .chain(std::iter::once(search_result("exact", "Acme", None)))
            .collect(),
    ));
    state.query = "acme".to_string();

    let token = state.begin_request();
    let excluded = state.local_result_ids_for_query(token);
    assert_eq!(excluded.len(), 3);
    assert_eq!(excluded[0], "exact");
    assert!(!excluded.contains(&"weak-02".to_string()));
    assert!(state.show_query_seed());
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("ranked local query should seed results");
    };
    assert_eq!(results[0].block_id, "exact");
}

#[test]
fn indexed_query_ties_keep_authoritative_order_after_other_queries_reorder_pages() {
    let alpha = search_result("alpha", "Acme Alpha", None);
    let bravo = search_result("bravo", "Acme Bravo", None);
    let mut cache = QuickFindLocalSearchCache::default();
    cache.record_complete_query_results("all_content|acme", [alpha.clone(), bravo.clone()]);
    cache.record_complete_query_results("all_content|other", [bravo.clone(), alpha.clone()]);
    assert_eq!(
        cache
            .pages
            .iter()
            .map(|page| page.block_id.as_str())
            .collect::<Vec<_>>(),
        vec!["bravo", "alpha"],
        "the second query must exercise global page-cache reordering"
    );

    let mut state = NotionSearchState::default();
    state.replace_local_search_cache_for_test(cache);
    state.query = "acme".to_string();
    assert!(state.show_query_seed());

    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("the indexed query should seed results");
    };
    assert_eq!(
        results
            .iter()
            .map(|result| result.block_id.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "bravo"]
    );
}
