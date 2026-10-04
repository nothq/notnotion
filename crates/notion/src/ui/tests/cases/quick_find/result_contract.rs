use crate::ui::search::QuickFindAction;

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use gpui::{
    point, px, AppContext as _, Bounds, Entity, Keystroke, ListOffset, Modifiers, Pixels,
    ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase, VisualTestContext, WindowHandle,
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
fn query_headers_metadata_badges_and_snippets_keep_the_56_and_76_pixel_row_contract(
    cx: &mut TestAppContext,
) {
    let (api, board) = query_result_contract_fixture();
    let (window, surface) = open_quick_find_app_with_board(cx, api, board);
    surface.update(cx, |surface, cx| {
        surface.activate_notion_search(cx);
        surface.apply_quick_find_action(QuickFindAction::QueryChanged("payroll".to_string()), cx);
    });
    cx.executor().advance_clock(Duration::from_millis(225));
    cx.run_until_parked();
    cx.read_entity(&surface, |surface, _| {
        assert_query_row_mapping(&surface.notion_search)
    });

    let mut visual_cx = VisualTestContext::from_window(window.into(), cx);
    visual_cx.refresh().expect("draw Quick Find query rows");
    visual_cx.run_until_parked();
    assert_query_row_geometry(&query_row_bounds(&mut visual_cx));
    assert_query_row_metadata(&mut visual_cx);
}

fn query_result_contract_fixture() -> (Arc<RecordingQuickFindApi>, BoardSnapshot) {
    let mut detailed = search_result(
        "detailed-result",
        "Payroll handbook",
        Some("General / … / Team"),
    );
    detailed.match_snippet = Some("The payroll matching sentence".to_string());
    detailed.editor_display_name = Some("Ada Lovelace".to_string());
    detailed.edited_label = Some("Edited Jan 1".to_string());
    detailed.badges = vec![
        PageShellSearchBadge::CurrentPage,
        PageShellSearchBadge::Database,
    ];
    let compact = search_result(
        "compact-result",
        "Top of mind",
        Some("Operations / Calendar"),
    );
    let mut long = search_result(
        "long-result",
        "Payroll handbook for every international operating company and every regional team",
        Some("Operations / People"),
    );
    long.icon = PageShellIcon::named("database");
    long.badges = vec![PageShellSearchBadge::Database];
    let api = RecordingQuickFindApi::new(
        vec![Vec::new()],
        vec![SearchWorkspaceResult {
            total: 3,
            consumed_result_count: 3,
            results: vec![detailed, long, compact],
        }],
        &[],
        HashMap::new(),
    );
    api.set_local_search_cache(indexed_local_search_cache("payroll", Vec::new()));
    let mut board = interaction_test_board();
    board.share_target_id = Some(
        "detailed-result"
            .parse()
            .expect("set current Quick Find page ID"),
    );
    (api, board)
}

fn assert_query_row_mapping(search: &NotionSearchState) {
    assert!(search.list_row_is_query_header(0));
    for (list_row, result_index) in [(1, 0), (2, 1), (3, 2)] {
        assert_eq!(
            search.query_result_index_for_list_row(list_row),
            Some(result_index)
        );
        assert_eq!(
            search.list_row_index_for_result(result_index),
            Some(list_row)
        );
    }
}

struct QueryRowBounds {
    detailed: Bounds<Pixels>,
    compact: Bounds<Pixels>,
    header: Bounds<Pixels>,
    title_text: Bounds<Pixels>,
    title_inline: Bounds<Pixels>,
    title_line: Bounds<Pixels>,
    compact_title_text: Bounds<Pixels>,
    long_title_line: Bounds<Pixels>,
    long_icon_slot: Bounds<Pixels>,
    long_title_text: Bounds<Pixels>,
    current_page_badge: Bounds<Pixels>,
    long_database_badge: Bounds<Pixels>,
}

fn query_row_bounds(visual_cx: &mut VisualTestContext) -> QueryRowBounds {
    let mut bounds = |id, message| visual_cx.debug_bounds(id).expect(message);
    QueryRowBounds {
        detailed: bounds("notion-search-result-0", "detailed query row should render"),
        compact: bounds("notion-search-result-2", "compact query row should render"),
        header: bounds("notion-search-query-header", "query header should render"),
        title_text: bounds(
            "notion-search-result-0-title-text",
            "query title text should render",
        ),
        title_inline: bounds(
            "notion-search-result-0-title-inline",
            "query inline title group should render",
        ),
        title_line: bounds(
            "notion-search-result-0-title",
            "query title line should render",
        ),
        compact_title_text: bounds(
            "notion-search-result-2-title-text",
            "compact query title text should render",
        ),
        long_title_line: bounds(
            "notion-search-result-1-title",
            "long query title line should render",
        ),
        long_icon_slot: bounds(
            "notion-search-result-1-icon-slot",
            "database query result icon slot should render",
        ),
        long_title_text: bounds(
            "notion-search-result-1-title-text",
            "long query title text should render",
        ),
        current_page_badge: bounds(
            "notion-search-result-0-badge-current-page",
            "current-page badge should render",
        ),
        long_database_badge: bounds(
            "notion-search-result-1-badge-database",
            "long-title database badge should render",
        ),
    }
}

fn assert_query_row_geometry(bounds: &QueryRowBounds) {
    assert_eq!(bounds.header.size.height.as_f32(), 33.0);
    assert_eq!(bounds.detailed.size.height.as_f32(), 76.0);
    assert_eq!(bounds.compact.size.height.as_f32(), 56.0);
    assert_eq!(bounds.long_icon_slot.size.width.as_f32(), 20.0);
    assert_eq!(bounds.long_title_line.size.width.as_f32(), 528.0);
    assert!(
        bounds.title_text.size.width.as_f32() > 60.0,
        "title text should keep its natural width: text={:?}, inline={:?}, line={:?}",
        bounds.title_text,
        bounds.title_inline,
        bounds.title_line
    );
    assert!(bounds.compact_title_text.size.width.as_f32() > 60.0);
    assert_eq!(
        bounds.compact_title_text.size.width.as_f32().fract(),
        0.0,
        "a fractional shaped width must round up so a complete short title does not paint a false ellipsis"
    );
    assert_badge_spacing(bounds);
}

fn assert_badge_spacing(bounds: &QueryRowBounds) {
    assert_eq!(
        bounds.current_page_badge.origin.x.as_f32()
            - bounds.title_text.origin.x.as_f32()
            - bounds.title_text.size.width.as_f32(),
        6.0,
        "badges should sit inline after the title instead of at the row edge"
    );
    assert_eq!(
        bounds.long_database_badge.origin.x.as_f32()
            - bounds.long_title_text.origin.x.as_f32()
            - bounds.long_title_text.size.width.as_f32(),
        6.0,
        "a long title should truncate exactly six pixels before its badge"
    );
    assert!(
        bounds.long_database_badge.origin.x.as_f32()
            + bounds.long_database_badge.size.width.as_f32()
            <= bounds.long_title_line.origin.x.as_f32()
                + bounds.long_title_line.size.width.as_f32(),
        "the preserved badge must stay inside the title line: title={:?}, badge={:?}, line={:?}",
        bounds.long_title_text,
        bounds.long_database_badge,
        bounds.long_title_line
    );
}

fn assert_query_row_metadata(visual_cx: &mut VisualTestContext) {
    for id in [
        "notion-search-result-0-metadata",
        "notion-search-result-0-snippet",
        "notion-search-result-0-badge-current-page",
        "notion-search-result-0-badge-database",
    ] {
        assert!(visual_cx.debug_bounds(id).is_some(), "missing `{id}`");
    }
}
