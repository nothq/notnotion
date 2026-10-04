use crate::ui::search::action::QuickFindCompletion;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::{
    Context, Entity, Keystroke, Modifiers, TestAppContext, VisualTestContext, WindowHandle,
};

use crate::{
    model::{
        BoardSnapshot, CardPage, LoadRecentPagesResult, NotionLaunchRoute, PageShellSearchResult,
        RecentPageResult, SearchWorkspaceResult,
    },
    ui::tests::cases::*,
};

use crate::ui::tests::cases::quick_find::api::RecordingQuickFindApi;

pub(super) struct QuickFindSurfaceHarness<'a, 'b, 'c> {
    pub(super) surface: &'a mut SurfaceState,
    pub(super) cx: &'b mut Context<'c, SurfaceState>,
}

impl<'a, 'b, 'c> QuickFindSurfaceHarness<'a, 'b, 'c> {
    pub(super) fn new(
        surface: &'a mut SurfaceState,
        cx: &'b mut Context<'c, SurfaceState>,
    ) -> Self {
        Self { surface, cx }
    }

    pub(super) fn complete_query(
        &mut self,
        token: NotionSearchRequestToken,
        results: Vec<PageShellSearchResult>,
    ) {
        self.surface.finish_quick_find_completion(
            QuickFindCompletion::query_for_test(
                token,
                Ok(SearchWorkspaceResult {
                    total: results.len() as u32,
                    consumed_result_count: results.len() as u32,
                    results,
                }),
            ),
            self.cx,
        );
    }

    pub(super) fn complete_recents(
        &mut self,
        token: NotionSearchRequestToken,
        results: Vec<RecentPageResult>,
    ) {
        self.surface.finish_quick_find_completion(
            QuickFindCompletion::recents(token, Ok(LoadRecentPagesResult { results })),
            self.cx,
        );
    }
}

pub(super) fn open_quick_find_app(
    cx: &mut TestAppContext,
    api: Arc<RecordingQuickFindApi>,
) -> (WindowHandle<NotionTestApp>, Entity<SurfaceState>) {
    open_quick_find_app_with_board(cx, api, interaction_test_board())
}

pub(super) fn open_quick_find_app_with_board(
    cx: &mut TestAppContext,
    api: Arc<RecordingQuickFindApi>,
    board: BoardSnapshot,
) -> (WindowHandle<NotionTestApp>, Entity<SurfaceState>) {
    let workspace_api: Arc<dyn NotionWorkspaceApi> = api;
    cx.set_global(AppearanceMode::Dark);
    let window = cx.open_window(app_size(), move |_, _| {
        NotionTestApp::from_ready_workspace(
            notion_route(TEST_BOARD_URL),
            board,
            workspace_api,
            Viewport::default(),
        )
    });
    cx.run_until_parked();
    let root = window
        .root(cx)
        .expect("access Quick Find test application root");
    let surface = root.update(cx, |app, cx| app.root.surface_for_test(cx));
    (window, surface)
}

pub(super) fn open_search_and_focus_input(
    window: &WindowHandle<NotionTestApp>,
    surface: &Entity<SurfaceState>,
    cx: &mut TestAppContext,
) {
    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.run_until_parked();
    cx.update_window((*window).into(), |_, window, cx| window.draw(cx).clear())
        .expect("draw Quick Find input");
}

pub(super) fn dispatch_keystroke(
    window: &WindowHandle<NotionTestApp>,
    keystroke: &str,
    cx: &mut TestAppContext,
) {
    cx.dispatch_keystroke(
        (*window).into(),
        Keystroke::parse(keystroke).expect("Quick Find test keystroke should parse"),
    );
}

pub(super) fn assert_selected_index(
    surface: &Entity<SurfaceState>,
    expected: usize,
    cx: &TestAppContext,
) {
    cx.read_entity(surface, |surface, _| {
        assert_eq!(surface.notion_search.selected_index, expected);
    });
}

pub(super) fn assert_loaded_results(
    surface: &SurfaceState,
    expected_total: u32,
    expected_results: &[PageShellSearchResult],
) {
    let NotionSearchResultsState::Loaded { total, results, .. } = &surface.notion_search.results
    else {
        panic!("Quick Find results should be loaded");
    };
    assert_eq!(*total, expected_total);
    assert_eq!(results.as_ref(), expected_results);
}

pub(super) fn assert_loaded_result_ids(state: &NotionSearchState, expected_ids: &[&str]) {
    let NotionSearchResultsState::Loaded { results, .. } = &state.results else {
        panic!("authoritative query results should be loaded");
    };
    assert_eq!(
        results
            .iter()
            .map(|result| result.block_id.as_str())
            .collect::<Vec<_>>(),
        expected_ids,
        "exact title should win across merged local and server results while equal tiers remain stable"
    );
}

pub(super) fn search_result(
    block_id: &str,
    title: &str,
    highlight: Option<&str>,
) -> PageShellSearchResult {
    PageShellSearchResult {
        block_id: block_id.to_string(),
        title: title.to_string(),
        icon: PageShellIcon::named("page"),
        target_board_url: format!("https://www.notion.so/acme/{block_id}"),
        highlight: highlight.map(str::to_string),
        match_snippet: None,
        editor_display_name: None,
        edited_label: None,
        edited_at: None,
        badges: Vec::new(),
    }
}

pub(super) fn card_page_for_result(result: &PageShellSearchResult) -> CardPage {
    CardPage {
        block_id: result.block_id.clone(),
        title: result.title.clone(),
        status: None,
        properties: Vec::new(),
        blocks: Vec::new(),
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
    }
}

pub(super) fn recent_page_results(results: Vec<PageShellSearchResult>) -> Vec<RecentPageResult> {
    let result_count = results.len() as u64;
    results
        .into_iter()
        .enumerate()
        .map(|(index, page)| RecentPageResult {
            page,
            visited_at_unix_millis: result_count - index as u64,
        })
        .collect()
}

pub(super) fn recent_result(
    page: PageShellSearchResult,
    visited_at_unix_millis: u64,
) -> RecentPageResult {
    RecentPageResult {
        page,
        visited_at_unix_millis,
    }
}

pub(super) fn unix_millis_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Quick Find test clock must follow the Unix epoch")
        .as_millis()
        .try_into()
        .expect("Quick Find test timestamp must fit in a u64")
}

pub(super) fn navigation_results() -> Vec<PageShellSearchResult> {
    vec![
        search_result("navigation-a", "Navigation A", None),
        search_result("navigation-b", "Navigation B", None),
        search_result("navigation-c", "Navigation C", None),
    ]
}

pub(super) fn notion_route(board_url: &str) -> NotionLaunchRoute {
    NotionLaunchRoute::board(
        board_url
            .parse()
            .expect("Quick Find test board URL should be valid"),
    )
}
