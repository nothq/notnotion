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
fn long_quick_find_preview_scrolls_inside_the_clipped_card(cx: &mut TestAppContext) {
    let result = search_result("long-preview", "Long preview", None);
    let api = long_preview_api(&result);
    let (window, surface) = open_quick_find_app(cx, api);

    surface.update(cx, |surface, cx| surface.activate_notion_search(cx));
    cx.run_until_parked();
    let mut visual_cx = VisualTestContext::from_window(window.into(), cx);
    visual_cx
        .refresh()
        .expect("draw the long Quick Find preview");
    visual_cx.run_until_parked();
    let scroll_position = visual_cx
        .debug_bounds("notion-search-preview-scroll")
        .expect("long Quick Find preview should expose a scroll host")
        .center();
    assert_first_preview_scroll(&mut visual_cx, &surface, scroll_position);
    assert_preview_tail_reachable(&mut visual_cx, &surface, scroll_position);
}

fn long_preview_api(result: &PageShellSearchResult) -> Arc<RecordingQuickFindApi> {
    let api = RecordingQuickFindApi::new(
        vec![vec![result.clone()]],
        Vec::new(),
        std::slice::from_ref(&result),
        HashMap::new(),
    );
    api.set_page(CardPage {
        block_id: result.block_id.clone(),
        title: result.title.clone(),
        status: None,
        properties: Vec::new(),
        blocks: (0..40)
            .map(|index| {
                CardPageBlock::editable(
                    format!("long-preview-{index}"),
                    result.block_id.clone(),
                    0,
                    CardPageBlockKind::Text,
                    format!("Preview line {index} with enough content to remain visible"),
                )
            })
            .collect(),
        discussions: Vec::new(),
        comments_writable: false,
        format: Default::default(),
    });
    api
}

fn assert_first_preview_scroll(
    visual_cx: &mut VisualTestContext,
    surface: &Entity<SurfaceState>,
    scroll_position: gpui::Point<gpui::Pixels>,
) {
    let initial_header_bounds = visual_cx
        .debug_bounds("notion-search-preview-header")
        .expect("long Quick Find preview should expose its header");
    let initial_title_bounds = visual_cx
        .debug_bounds("notion-search-preview-title")
        .expect("long Quick Find preview should expose its title");
    let initial_offset = visual_cx.read_entity(&surface, |surface, _| {
        let NotionSearchPreviewState::Loaded { page, .. } = &surface.notion_search.preview else {
            panic!("long Quick Find preview should be loaded");
        };
        assert_eq!(
            page.list_state.item_count(),
            41,
            "the virtual preview must include one intro row and every visible block"
        );
        page.list_state.logical_scroll_top()
    });
    visual_cx.simulate_event(ScrollWheelEvent {
        position: scroll_position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(-40.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Started,
    });
    visual_cx.run_until_parked();
    let final_offset = visual_cx.read_entity(&surface, |surface, _| {
        let NotionSearchPreviewState::Loaded { page, .. } = &surface.notion_search.preview else {
            panic!("long Quick Find preview should remain loaded");
        };
        page.list_state.logical_scroll_top()
    });
    assert_ne!(
        (final_offset.item_ix, final_offset.offset_in_item),
        (initial_offset.item_ix, initial_offset.offset_in_item),
        "wheel input inside the preview must reveal later content: initial={initial_offset:?}, final={final_offset:?}"
    );
    visual_cx
        .refresh()
        .expect("draw the scrolled Quick Find preview chrome");
    let final_header_bounds = visual_cx
        .debug_bounds("notion-search-preview-header")
        .expect("the partially visible preview header should remain inspectable");
    let final_title_bounds = visual_cx
        .debug_bounds("notion-search-preview-title")
        .expect("the partially visible preview title should remain inspectable");
    assert!(
        final_header_bounds.top() < initial_header_bounds.top()
            && final_title_bounds.top() < initial_title_bounds.top(),
        "Notion scroll parity requires the whole preview card to move, including header and title"
    );
}

fn assert_preview_tail_reachable(
    visual_cx: &mut VisualTestContext,
    surface: &Entity<SurfaceState>,
    scroll_position: gpui::Point<gpui::Pixels>,
) {
    let mut tail_visible = false;
    for _ in 0..20 {
        visual_cx.simulate_event(ScrollWheelEvent {
            position: scroll_position,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-400.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
        });
        visual_cx.run_until_parked();
        visual_cx
            .refresh()
            .expect("draw another virtualized Quick Find preview segment");
        if visual_cx
            .debug_bounds("notion-search-preview-block-39-text")
            .is_some()
        {
            tail_visible = true;
            break;
        }
    }
    let (tail_offset, tail_viewport) = visual_cx.read_entity(&surface, |surface, _| {
        let NotionSearchPreviewState::Loaded { page, .. } = &surface.notion_search.preview else {
            panic!("long Quick Find preview should remain loaded at the tail");
        };
        (
            page.list_state.logical_scroll_top(),
            page.list_state.viewport_bounds(),
        )
    });
    assert!(
        tail_visible,
        "the scroll host must make preview content beyond the former 27-block cap reachable; offset={tail_offset:?}, viewport={tail_viewport:?}"
    );
}
