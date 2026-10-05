//! Wheel-driven headless scroll profiler for the Notion page document.
//!
//! Each frame mirrors the live interaction: one platform scroll-wheel event is
//! dispatched at the painted document, then the window is drawn with the real
//! macOS text system. The workspace enables gpui's `test-support` feature, so
//! the dispatch itself already draws the dirty window synchronously; the
//! `event` series therefore carries the real frame cost, the explicit draw that
//! follows is a clean-window redraw, and the settle series is a second redraw
//! with no new input.

use crate::ui::regression_test_support::{
    page_list_state, regression_board, regression_page, regression_surface_with,
    NotionPageScrollSurface,
};
use crate::ui::tests::*;
use gpui::{InputEvent, ListState, Modifiers, ScrollDelta, ScrollWheelEvent, TouchPhase};

mod fixtures;

use fixtures::linear_page;

const SCROLL_PROFILE_EVENT_COUNT: usize = 160;
const SCROLL_PROFILE_LONG_EVENT_COUNT: usize = 4000;
const SCROLL_PROFILE_WARMUP_EVENTS: usize = 4;
const SCROLL_PROFILE_FLIP_EVERY: usize = 60;
const SCROLL_PROFILE_WHEEL_DELTA_Y: f32 = 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollProfileFixture {
    Columns,
    Linear,
}

#[derive(Clone, Copy)]
struct ScrollProfileCase {
    surface: NotionPageScrollSurface,
    fixture: ScrollProfileFixture,
    ai_panel_open: bool,
    event_count: usize,
}

#[gpui::test]
#[ignore]
fn profile_standalone_linear_page_wheel_scroll() {
    profile_wheel_scroll(ScrollProfileCase::new(
        NotionPageScrollSurface::StandaloneDocument,
        ScrollProfileFixture::Linear,
    ));
}

/// Long enough to CPU-sample with `sample <pid> 10` while it runs.
#[gpui::test]
#[ignore]
fn profile_standalone_linear_page_wheel_scroll_long() {
    profile_wheel_scroll(
        ScrollProfileCase::new(
            NotionPageScrollSurface::StandaloneDocument,
            ScrollProfileFixture::Linear,
        )
        .with_event_count(SCROLL_PROFILE_LONG_EVENT_COUNT),
    );
}

#[gpui::test]
#[ignore]
fn profile_standalone_linear_page_wheel_scroll_with_ai_panel() {
    profile_wheel_scroll(
        ScrollProfileCase::new(
            NotionPageScrollSurface::StandaloneDocument,
            ScrollProfileFixture::Linear,
        )
        .with_ai_panel(),
    );
}

#[gpui::test]
#[ignore]
fn profile_standalone_columns_page_wheel_scroll() {
    profile_wheel_scroll(ScrollProfileCase::new(
        NotionPageScrollSurface::StandaloneDocument,
        ScrollProfileFixture::Columns,
    ));
}

#[gpui::test]
#[ignore]
fn profile_selected_page_linear_wheel_scroll() {
    profile_wheel_scroll(ScrollProfileCase::new(
        NotionPageScrollSurface::SelectedPageOverlay,
        ScrollProfileFixture::Linear,
    ));
}

#[gpui::test]
#[ignore]
fn profile_selected_page_columns_wheel_scroll() {
    profile_wheel_scroll(ScrollProfileCase::new(
        NotionPageScrollSurface::SelectedPageOverlay,
        ScrollProfileFixture::Columns,
    ));
}

impl ScrollProfileCase {
    const fn new(surface: NotionPageScrollSurface, fixture: ScrollProfileFixture) -> Self {
        Self {
            surface,
            fixture,
            ai_panel_open: false,
            event_count: SCROLL_PROFILE_EVENT_COUNT,
        }
    }

    const fn with_ai_panel(mut self) -> Self {
        self.ai_panel_open = true;
        self
    }

    const fn with_event_count(mut self, event_count: usize) -> Self {
        self.event_count = event_count;
        self
    }

    fn label(&self) -> String {
        let chrome = if self.ai_panel_open { "/ai-panel" } else { "" };
        format!("{:?}/{:?}{chrome}", self.surface, self.fixture).to_ascii_lowercase()
    }

    fn page(&self) -> CardPage {
        match self.fixture {
            ScrollProfileFixture::Columns => regression_page(),
            ScrollProfileFixture::Linear => linear_page(),
        }
    }

    fn board(&self) -> BoardSnapshot {
        match self.surface {
            NotionPageScrollSurface::StandaloneDocument => {
                let mut board = notion_page_shell_test_board();
                clear_board_views(&mut board);
                board
            }
            NotionPageScrollSurface::SelectedPageOverlay => regression_board(),
        }
    }
}

fn profile_wheel_scroll(case: ScrollProfileCase) {
    let _guard = acquire_headless_test_lock();
    let label = case.label();
    let mut cx = headless_test_context();
    let (window, list_state) = open_scroll_profile_window(&mut cx, case);
    let viewport = list_state.viewport_bounds();
    eprintln!(
        "profiling {label}: list viewport {viewport:?}, {} virtualized items",
        list_state.item_count()
    );

    let mut driver = WheelDriver {
        cx: &mut cx,
        window,
        position: viewport.center(),
        direction: -1.0,
    };
    for _ in 0..SCROLL_PROFILE_WARMUP_EVENTS {
        driver.event(TouchPhase::Moved);
    }
    let initial_offset = list_state.logical_scroll_top();
    let series = driver.run(case.event_count);
    let final_offset = list_state.logical_scroll_top();
    assert_ne!(
        (initial_offset.item_ix, initial_offset.offset_in_item),
        (final_offset.item_ix, final_offset.offset_in_item),
        "wheel events did not move the Notion document"
    );
    series.report(&label);
}

fn open_scroll_profile_window(
    cx: &mut HeadlessAppContext,
    case: ScrollProfileCase,
) -> (gpui::WindowHandle<SurfaceState>, ListState) {
    let page = case.page();
    let block_count = page.blocks.len();
    let mut state = regression_surface_with(case.surface, case.board(), page, Viewport::default());
    state.notion_chrome.notion_ai_open = case.ai_panel_open;
    let window = cx
        .open_window(app_size(), move |_, cx| {
            let surface = cx.new(move |_| state);
            surface.update(cx, {
                let surface = surface.clone();
                move |state, cx| state.presentation.initialize_cached_regions(surface, cx)
            });
            surface
        })
        .expect("open scroll profile window");
    cx.run_until_parked();
    draw_frame(cx, window);
    cx.run_until_parked();

    let root = window.root(cx).expect("access scroll profile root");
    let list_state = cx.read_entity(&root, |state, _| {
        page_list_state(state, case.surface).clone()
    });
    let viewport = list_state.viewport_bounds();
    assert!(
        viewport.size.height > px(0.0) && viewport.size.width > px(0.0),
        "the Notion document list must be laid out before profiling: {viewport:?}"
    );
    assert!(
        list_state.item_count() > 2,
        "fixture with {block_count} blocks did not virtualize"
    );
    (window, list_state)
}

struct ProfileSeries {
    dispatch: Vec<Duration>,
    frame: Vec<Duration>,
    settle: Vec<Duration>,
    total: Duration,
}

impl ProfileSeries {
    fn with_capacity(event_count: usize) -> Self {
        Self {
            dispatch: Vec::with_capacity(event_count),
            frame: Vec::with_capacity(event_count),
            settle: Vec::with_capacity(event_count),
            total: Duration::ZERO,
        }
    }

    fn report(&self, label: &str) {
        let report = |name: &str, durations: &[Duration]| {
            eprintln!(
                "{}",
                super::profile::summarize_profile_frames(
                    &format!("{label} {name}"),
                    durations,
                    self.total,
                )
            );
        };
        report("event+draw", &self.frame);
        report("event (dispatch incl. synchronous draw)", &self.dispatch);
        report("settle-draw", &self.settle);
    }
}

struct WheelDriver<'a> {
    cx: &'a mut HeadlessAppContext,
    window: gpui::WindowHandle<SurfaceState>,
    position: gpui::Point<Pixels>,
    direction: f32,
}

impl WheelDriver<'_> {
    fn run(&mut self, event_count: usize) -> ProfileSeries {
        let mut series = ProfileSeries::with_capacity(event_count);
        let start = Instant::now();
        for index in 0..event_count {
            let starts_gesture = index % SCROLL_PROFILE_FLIP_EVERY == 0;
            if starts_gesture && index > 0 {
                self.end_gesture();
                self.direction = -self.direction;
            }
            let phase = if starts_gesture {
                TouchPhase::Started
            } else {
                TouchPhase::Moved
            };
            let (dispatch, frame) = self.event(phase);
            series.dispatch.push(dispatch);
            series.frame.push(dispatch + frame);
            series.settle.push(self.settle());
        }
        self.end_gesture();
        series.total = start.elapsed();
        series
    }

    fn event(&mut self, touch_phase: TouchPhase) -> (Duration, Duration) {
        let delta = SCROLL_PROFILE_WHEEL_DELTA_Y * self.direction;
        let dispatch_start = Instant::now();
        self.dispatch(ScrollWheelEvent {
            position: self.position,
            delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
            modifiers: Modifiers::none(),
            touch_phase,
        });
        let dispatch = dispatch_start.elapsed();
        let frame_start = Instant::now();
        draw_frame(self.cx, self.window);
        self.cx.run_until_parked();
        (dispatch, frame_start.elapsed())
    }

    fn settle(&mut self) -> Duration {
        let start = Instant::now();
        draw_frame(self.cx, self.window);
        self.cx.run_until_parked();
        start.elapsed()
    }

    fn end_gesture(&mut self) {
        self.dispatch(ScrollWheelEvent {
            position: self.position,
            delta: ScrollDelta::Pixels(point(px(0.0), px(0.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Ended,
        });
        self.settle();
    }

    fn dispatch(&mut self, event: ScrollWheelEvent) {
        self.cx
            .update_window(self.window.into(), |_, window, cx| {
                window.dispatch_event(event.to_platform_input(), cx);
            })
            .expect("dispatch scroll wheel event");
    }
}

fn draw_frame(cx: &mut HeadlessAppContext, window: gpui::WindowHandle<SurfaceState>) {
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear())
        .expect("draw scroll profile frame");
}
