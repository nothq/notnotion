use std::collections::HashMap;

use app_model::{AppearanceMode, Viewport};
use gpui::{
    point, px, size, AppContext, Bounds, ListState, Modifiers, Pixels, ScrollDelta,
    ScrollWheelEvent, TestAppContext, TouchPhase, VisualTestContext,
};

use crate::model::{BoardColumn, BoardSnapshot, CardPage, CardSummary};

use super::{CardPeekState, LoadedCardPage, SurfaceState};

mod fixtures;

pub(crate) use fixtures::{regression_board, regression_page, regression_viewport};

const REGRESSION_WINDOW_WIDTH: u32 = 1_200;
const REGRESSION_WINDOW_HEIGHT: u32 = 760;
const REGRESSION_PAGE_BLOCK_COUNT: usize = 640;
const REGRESSION_WHEEL_DELTA_Y: f32 = -240.0;
const REGRESSION_WHEEL_EVENT_COUNT: usize = 20;
const REGRESSION_SETTLE_FRAME_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotionPageScrollSurface {
    StandaloneDocument,
    SelectedPageOverlay,
}

impl NotionPageScrollSurface {
    const fn scroll_host_id(self) -> &'static str {
        match self {
            Self::StandaloneDocument => "notion-page-scroll",
            Self::SelectedPageOverlay => "notion-selected-page-scroll",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NotionPageScrollOffset {
    pub item_index: usize,
    pub offset_in_item: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotionPageScrollOutcome {
    pub surface: NotionPageScrollSurface,
    pub scroll_host_id: &'static str,
    pub document_block_count: usize,
    pub virtualized_item_count: usize,
    pub scroll_host_width: f32,
    pub scroll_host_height: f32,
    pub list_viewport_height: f32,
    pub initial_offset: NotionPageScrollOffset,
    pub final_offset: NotionPageScrollOffset,
    pub wheel_delta_y: f32,
}

pub fn drive_standalone_document_wheel_scroll(cx: &mut TestAppContext) -> NotionPageScrollOutcome {
    drive_page_wheel_scroll(cx, NotionPageScrollSurface::StandaloneDocument)
}

pub fn drive_selected_page_overlay_wheel_scroll(
    cx: &mut TestAppContext,
) -> NotionPageScrollOutcome {
    drive_page_wheel_scroll(cx, NotionPageScrollSurface::SelectedPageOverlay)
}

fn drive_page_wheel_scroll(
    cx: &mut TestAppContext,
    surface: NotionPageScrollSurface,
) -> NotionPageScrollOutcome {
    let window = cx.open_window(regression_window_size(), move |_, _| {
        regression_surface(surface)
    });
    cx.run_until_parked();
    let root = window
        .root(cx)
        .expect("access Notion page-scroll regression root");
    let mut visual_cx = VisualTestContext::from_window(window.into(), cx);

    draw_surface(&mut visual_cx);
    let scroll_host_id = surface.scroll_host_id();
    let scroll_host_bounds = visual_cx
        .debug_bounds(scroll_host_id)
        .unwrap_or_else(|| panic!("painted Notion scroll host `{scroll_host_id}` is missing"));
    assert_bounded_scroll_host(scroll_host_id, scroll_host_bounds);

    let list_state = visual_cx.read_entity(&root, |surface_state, _| {
        page_list_state(surface_state, surface).clone()
    });
    let list_viewport = list_state.viewport_bounds();
    assert_painted_list_viewport(scroll_host_id, scroll_host_bounds, list_viewport);

    let document_block_count = visual_cx.read_entity(&root, |surface_state, _| {
        page_list_state_and_block_count(surface_state, surface).1
    });
    let virtualized_item_count = list_state.item_count();
    assert!(
        virtualized_item_count > 2,
        "long Notion fixture did not produce a multi-item virtualized document"
    );

    let initial_offset = page_scroll_offset(&list_state);

    let final_offset = drive_wheel_events(
        &mut visual_cx,
        scroll_host_bounds,
        &list_state,
        initial_offset,
    );
    NotionPageScrollOutcome {
        surface,
        scroll_host_id,
        document_block_count,
        virtualized_item_count,
        scroll_host_width: scroll_host_bounds.size.width.as_f32(),
        scroll_host_height: scroll_host_bounds.size.height.as_f32(),
        list_viewport_height: list_viewport.size.height.as_f32(),
        initial_offset,
        final_offset,
        wheel_delta_y: REGRESSION_WHEEL_DELTA_Y,
    }
}

fn drive_wheel_events(
    visual_cx: &mut VisualTestContext,
    scroll_host_bounds: Bounds<Pixels>,
    list_state: &ListState,
    initial_offset: NotionPageScrollOffset,
) -> NotionPageScrollOffset {
    let mut previous_offset = initial_offset;
    for event_index in 0..REGRESSION_WHEEL_EVENT_COUNT {
        visual_cx.simulate_event(ScrollWheelEvent {
            position: scroll_host_bounds.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(REGRESSION_WHEEL_DELTA_Y))),
            modifiers: Modifiers::none(),
            touch_phase: if event_index == 0 {
                TouchPhase::Started
            } else {
                TouchPhase::Moved
            },
        });
        let event_offset = page_scroll_offset(list_state);
        assert!(
            page_scroll_offset_is_after(event_offset, previous_offset),
            "Notion page scroll did not advance after wheel event {event_index}: previous={previous_offset:?}, event={event_offset:?}"
        );
        let mut settled_offset = event_offset;
        for settle_frame in 0..REGRESSION_SETTLE_FRAME_COUNT {
            draw_pending_surface_frame(visual_cx);
            let next_offset = page_scroll_offset(list_state);
            assert!(
                !page_scroll_offset_is_after(settled_offset, next_offset),
                "Notion page scroll regressed after wheel event {event_index}, settle frame {settle_frame}: previous={settled_offset:?}, next={next_offset:?}"
            );
            settled_offset = next_offset;
        }
        previous_offset = settled_offset;
    }
    visual_cx.simulate_event(ScrollWheelEvent {
        position: scroll_host_bounds.center(),
        delta: ScrollDelta::Pixels(point(px(0.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Ended,
    });
    for settle_frame in 0..REGRESSION_SETTLE_FRAME_COUNT {
        draw_pending_surface_frame(visual_cx);
        let ended_offset = page_scroll_offset(list_state);
        assert!(
            !page_scroll_offset_is_after(previous_offset, ended_offset),
            "Notion page scroll regressed after wheel gesture ended, settle frame {settle_frame}: previous={previous_offset:?}, settled={ended_offset:?}"
        );
        previous_offset = ended_offset;
    }
    previous_offset
}

fn draw_pending_surface_frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
    cx.run_until_parked();
}

fn page_scroll_offset_is_after(
    candidate: NotionPageScrollOffset,
    previous: NotionPageScrollOffset,
) -> bool {
    candidate.item_index > previous.item_index
        || (candidate.item_index == previous.item_index
            && candidate.offset_in_item > previous.offset_in_item)
}

fn draw_surface(cx: &mut VisualTestContext) {
    cx.refresh()
        .expect("draw Notion page-scroll regression surface");
    cx.run_until_parked();
}

fn assert_bounded_scroll_host(scroll_host_id: &str, bounds: Bounds<Pixels>) {
    assert!(
        bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
        "painted Notion scroll host `{scroll_host_id}` has empty bounds: {bounds:?}"
    );
    assert!(
        bounds.left() >= px(0.0)
            && bounds.top() >= px(0.0)
            && bounds.right() <= px(REGRESSION_WINDOW_WIDTH as f32)
            && bounds.bottom() <= px(REGRESSION_WINDOW_HEIGHT as f32),
        "painted Notion scroll host `{scroll_host_id}` escapes its bounded window: {bounds:?}"
    );
}

fn assert_painted_list_viewport(
    scroll_host_id: &str,
    scroll_host_bounds: Bounds<Pixels>,
    list_viewport: Bounds<Pixels>,
) {
    assert!(
        list_viewport.size.width > px(0.0) && list_viewport.size.height > px(0.0),
        "Notion list inside `{scroll_host_id}` was not laid out: {list_viewport:?}"
    );
    assert!(
        scroll_host_bounds.contains(&list_viewport.center()),
        "Notion list viewport is not inside painted host `{scroll_host_id}`: host={scroll_host_bounds:?}, list={list_viewport:?}"
    );
    assert!(
        list_viewport.size.height <= scroll_host_bounds.size.height,
        "Notion list viewport is not bounded by painted host `{scroll_host_id}`: host={scroll_host_bounds:?}, list={list_viewport:?}"
    );
}

pub(crate) fn page_list_state(
    surface: &SurfaceState,
    target: NotionPageScrollSurface,
) -> &ListState {
    page_list_state_and_block_count(surface, target).0
}

fn page_list_state_and_block_count(
    surface: &SurfaceState,
    target: NotionPageScrollSurface,
) -> (&ListState, usize) {
    let page = match target {
        NotionPageScrollSurface::StandaloneDocument => surface
            .page_documents
            .standalone
            .as_ref()
            .expect("standalone Notion regression page is missing"),
        NotionPageScrollSurface::SelectedPageOverlay => {
            let selected_page = surface
                .page_documents
                .selected_page
                .as_ref()
                .expect("selected-page Notion regression state is missing");
            let CardPeekState::Loaded(page) = selected_page else {
                panic!("selected-page Notion regression fixture is not loaded");
            };
            page
        }
    };
    (&page.list_state, page.data.page.blocks.len())
}

fn page_scroll_offset(list_state: &ListState) -> NotionPageScrollOffset {
    let offset = list_state.logical_scroll_top();
    NotionPageScrollOffset {
        item_index: offset.item_ix,
        offset_in_item: offset.offset_in_item.as_f32(),
    }
}

fn regression_surface(surface: NotionPageScrollSurface) -> SurfaceState {
    regression_surface_with(
        surface,
        regression_board(),
        regression_page(),
        regression_viewport(),
    )
}

pub(crate) fn regression_surface_with(
    surface: NotionPageScrollSurface,
    mut board: BoardSnapshot,
    page: CardPage,
    viewport: Viewport,
) -> SurfaceState {
    match surface {
        NotionPageScrollSurface::StandaloneDocument => {
            board.page_content = Some(page);
            SurfaceState::fixture_snapshot(board, HashMap::new(), AppearanceMode::Dark, viewport)
        }
        NotionPageScrollSurface::SelectedPageOverlay => {
            board.columns.push(BoardColumn {
                title: "In progress".to_string(),
                option_color: None,
                cards: vec![CardSummary {
                    block_id: page.block_id.clone(),
                    title: page.title.clone(),
                    height: 72.0,
                    has_content: true,
                    icon: None,
                }],
            });
            let mut surface = SurfaceState::fixture_snapshot(
                board,
                HashMap::new(),
                AppearanceMode::Dark,
                viewport,
            );
            surface.page_documents.selected_page =
                Some(CardPeekState::Loaded(LoadedCardPage::new(page)));
            surface
        }
    }
}

fn regression_window_size() -> gpui::Size<Pixels> {
    size(
        px(REGRESSION_WINDOW_WIDTH as f32),
        px(REGRESSION_WINDOW_HEIGHT as f32),
    )
}
