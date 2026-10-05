use super::{
    column_style, AppearanceMode, BoardSnapshot, BoardViewState, CardPage, Context,
    DatabaseFilterUiState, DatabaseSearchState, HashMap, IconSet, LoadedCardPage,
    NotionChromeState, NotionDateViewState, NotionSidebarState, NotionSidebarTab, NotionStartup,
    PageEditorState, ScrollHandle, SurfacePresentationState, SurfaceState, Theme, Viewport, Window,
};
#[cfg(any(test, feature = "test-support"))]
use super::{NotionFixtureInput, NotionFixtureSource};
use crate::ui::{civil_date_month_start, local_today_civil_date};

mod filter_projection;
mod initialization;
mod keyboard;
mod route;
mod startup_construction;

use initialization::{
    board_presence_images, empty_board_snapshot, notion_sidebar_should_be_visible,
};

impl SurfaceState {
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_snapshot(
        board: BoardSnapshot,
        snapshot_pages: HashMap<String, CardPage>,
        appearance_mode: AppearanceMode,
        viewport: Viewport,
    ) -> Self {
        Self::from_startup(
            NotionStartup::Fixture(NotionFixtureInput {
                board,
                source: NotionFixtureSource::SnapshotPages(snapshot_pages),
            }),
            appearance_mode,
            viewport,
            true,
            super::NotionSurfaceResources::fixture_without_icon_io(),
        )
    }

    pub(crate) fn refresh_appearance_mode(&mut self, cx: &mut Context<Self>) {
        let appearance_mode = AppearanceMode::current(cx);
        if self.appearance_mode == appearance_mode {
            return;
        }
        self.appearance_mode = appearance_mode;
        self.theme = Theme::for_appearance_mode(appearance_mode);
        self.icons = std::sync::Arc::new(IconSet::new(appearance_mode));
        self.refresh_column_styles();
        cx.notify();
    }

    pub fn set_viewport(&mut self, viewport: Viewport, cx: &mut Context<Self>) {
        let viewport_size_changed = self.viewport.logical_width != viewport.logical_width
            || self.viewport.logical_height != viewport.logical_height
            || (self.viewport.scale_factor - viewport.scale_factor).abs() > f64::EPSILON;
        let viewport_position_changed = self.viewport.viewport_x != viewport.viewport_x
            || self.viewport.viewport_y != viewport.viewport_y;
        if !viewport_size_changed && !viewport_position_changed {
            return;
        }
        self.viewport = viewport;
        if viewport_size_changed {
            self.notion_chrome.notion_sidebar_width = self.page_layout().sidebar_width();
            cx.notify();
        }
    }

    pub(crate) fn set_preview_width(&mut self, preview_width: f32, cx: &mut Context<Self>) {
        let preview_width = preview_width.max(0.0);
        if (self.preview_width - preview_width).abs() < f32::EPSILON {
            return;
        }
        self.preview_width = preview_width;
        self.notion_chrome.notion_sidebar_width = self.page_layout().sidebar_width();
        cx.notify();
    }

    pub(crate) fn refresh_column_styles(&mut self) {
        for (column, board_column) in self.columns.iter_mut().zip(self.board.columns.iter()) {
            column.style = column_style(
                &board_column.title,
                board_column.option_color.as_deref(),
                self.appearance_mode,
            );
        }
    }

    pub(crate) fn spawn_background_task<Request, Result, Work, Apply>(
        &mut self,
        request: Request,
        cx: &mut Context<Self>,
        work: Work,
        apply: Apply,
    ) where
        Request: Send + 'static,
        Result: Send + 'static,
        Work: FnOnce(Request) -> Result + Send + 'static,
        Apply: FnOnce(&mut Self, Result, &mut Context<Self>) + 'static,
    {
        gpui_components::spawn_background_task_for_entity(request, cx, work, apply);
    }

    pub(crate) fn spawn_timer_task<Token, Apply>(
        &mut self,
        token: Token,
        delay: std::time::Duration,
        cx: &mut Context<Self>,
        apply: Apply,
    ) where
        Token: Send + 'static,
        Apply: FnOnce(&mut Self, Token, &mut Context<Self>) + 'static,
    {
        gpui_components::spawn_timer_task_for_entity(token, delay, cx, apply);
    }
}
