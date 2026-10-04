use gpui::{size, AppContext, HeadlessAppContext, IntoElement, KeyDownEvent, Keystroke, Render};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};

use crate::ui::*;

const TEST_BOARD_URL: &str = "https://www.notion.so/acme/a1b2c3d4e5f647188293a4b5c6d7e8f0?v=b2c3d4e5f6a748299304b5c6d7e8f901";

mod cases;
mod helpers;
mod profile;
mod profile_page;
mod profile_scroll;
mod scroll_regression;

pub(crate) use helpers::*;

fn headless_test_context() -> HeadlessAppContext {
    let platform = current_platform(true);
    let mut cx = HeadlessAppContext::with_platform(platform.text_system(), Arc::new(()), || {
        current_headless_renderer()
    });
    cx.update(|cx| cx.set_global(AppearanceMode::Dark));
    cx
}

fn acquire_headless_test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn app_size() -> gpui::Size<Pixels> {
    let viewport = Viewport::default();
    size(px(viewport.app_width()), px(viewport.app_height()))
}

fn named_key_down_event(key: &str) -> KeyDownEvent {
    KeyDownEvent {
        keystroke: Keystroke::parse(key).expect("named keystroke should parse"),
        is_held: false,
        prefer_character_input: false,
    }
}

struct NotionTestApp {
    root: SurfaceRoot,
}

impl NotionTestApp {
    fn from_workspace(
        board: BoardSnapshot,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        viewport: Viewport,
    ) -> Self {
        Self {
            root: SurfaceRoot::fixture_workspace(board, workspace_api, viewport),
        }
    }

    fn from_ready_workspace(
        route: crate::model::NotionLaunchRoute,
        board: BoardSnapshot,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        viewport: Viewport,
    ) -> Self {
        Self {
            root: SurfaceRoot::fixture_ready_workspace(route, board, workspace_api, viewport),
        }
    }
}

impl Render for NotionTestApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.root.render_standalone(cx)
    }
}
