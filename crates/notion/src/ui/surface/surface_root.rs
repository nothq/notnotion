use super::{
    AnyElement, App, AppContext, AppearanceMode, BoardSnapshot, Context, IntoElement,
    NotionBootstrapApi, NotionPendingNavigation, NotionStartup, NotionSurfaceResources,
    NotionSurfaceServices, SurfaceFrame, SurfaceRoot, SurfaceState, Viewport, Window,
};
#[cfg(any(test, feature = "test-support"))]
use super::{NotionFixtureInput, NotionFixtureSource};
use app_model::SurfaceRoot as AppSurfaceRoot;

impl SurfaceRoot {
    pub(crate) fn production(
        bootstrap_api: crate::ui::Arc<dyn NotionBootstrapApi>,
        notion_services: NotionSurfaceServices,
    ) -> Self {
        let viewport = Viewport::default();
        Self {
            startup: NotionStartup::Loading {
                bootstrap_api,
                request_started: false,
                cache_request_started: false,
                cached: None,
                retry_attempt: 0,
            },
            notion_resources: NotionSurfaceResources::new(notion_services),
            viewport,
            preview_width: viewport.app_width(),
            active: false,
            pending_route: None,
            surface: None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(input: NotionFixtureInput, viewport: Viewport) -> Self {
        Self {
            startup: NotionStartup::Fixture(input),
            notion_resources: NotionSurfaceResources::fixture_without_icon_io(),
            viewport,
            preview_width: viewport.app_width(),
            active: false,
            pending_route: None,
            surface: None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_workspace(
        board: BoardSnapshot,
        workspace_api: crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>,
        viewport: Viewport,
    ) -> Self {
        Self::fixture(
            NotionFixtureInput {
                board,
                source: NotionFixtureSource::WorkspaceApi(workspace_api),
            },
            viewport,
        )
    }

    #[cfg(test)]
    pub(crate) fn fixture_ready_workspace(
        route: crate::model::NotionLaunchRoute,
        board: BoardSnapshot,
        workspace_api: crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>,
        viewport: Viewport,
    ) -> Self {
        Self {
            startup: NotionStartup::Ready(crate::model::NotionWorkspaceBootstrap {
                route,
                workspace: board,
                workspace_api,
                code_settings: crate::model::CardPageCodeSettingsCapability::memory(),
                previous_state_disposition:
                    crate::model::NotionPreviousStateDisposition::PreserveCompatible,
            }),
            notion_resources: NotionSurfaceResources::fixture_without_icon_io(),
            viewport,
            preview_width: viewport.app_width(),
            active: false,
            pending_route: None,
            surface: None,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_snapshot(
        board: BoardSnapshot,
        fixture_pages: std::collections::HashMap<String, crate::model::CardPage>,
        viewport: Viewport,
    ) -> Self {
        Self::fixture(
            NotionFixtureInput {
                board,
                source: NotionFixtureSource::SnapshotPages(fixture_pages),
            },
            viewport,
        )
    }

    pub fn restore_notion_route(
        &mut self,
        route: crate::model::NotionLaunchRoute,
    ) -> Result<(), String> {
        if self.surface.is_some() {
            return Err(
                "Notion launch route must be restored before surface activation".to_string(),
            );
        }
        self.pending_route = Some(route);
        Ok(())
    }

    pub fn set_viewport(&mut self, viewport: Viewport, cx: &mut App) {
        self.viewport = viewport;
        self.preview_width = viewport.app_width();
        if let Some(surface) = self.surface.as_ref() {
            surface.update(cx, |surface, cx| {
                surface.set_viewport(viewport, cx);
                surface.set_preview_width(viewport.app_width(), cx);
            });
        }
    }

    pub fn ensure_surface_state(&mut self, active: bool, cx: &mut App) {
        self.active = active;
        if !active {
            return;
        }
        let surface = self.ensure_surface(cx);
        let viewport = self.viewport;
        let preview_width = self.preview_width;
        surface.update(cx, |surface, cx| {
            surface.surface_active = true;
            surface.refresh_appearance_mode(cx);
            surface.set_viewport(viewport, cx);
            surface.set_preview_width(preview_width, cx);
            surface.ensure_notion_startup(cx);
        });
    }

    pub fn render_standalone(&mut self, cx: &mut App) -> AnyElement {
        self.ensure_surface_state(true, cx);
        self.ensure_surface(cx).into_any_element()
    }

    #[cfg(test)]
    pub(crate) fn surface_for_test<AppState: 'static>(
        &mut self,
        cx: &mut Context<AppState>,
    ) -> gpui::Entity<SurfaceState> {
        self.ensure_surface(cx)
    }

    /// Shows `route`, or reports why the surface cannot navigate yet.
    pub fn open_notion_workspace<AppState: 'static>(
        &mut self,
        route: crate::model::NotionLaunchRoute,
        cx: &mut Context<AppState>,
    ) -> Result<(), String> {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, cx| {
            let history_update = surface.notion_startup.push_history_update();
            surface
                .begin_notion_navigation(route, history_update, cx)
                .map(|_| ())
        })
    }

    #[doc(hidden)]
    pub fn set_sidebar_visible<AppState: 'static>(
        &mut self,
        visible: bool,
        cx: &mut Context<AppState>,
    ) {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, cx| {
            surface.notion_chrome.notion_sidebar_visible = visible;
            cx.notify();
        });
    }

    #[doc(hidden)]
    pub fn collapse_page_section<AppState: 'static>(
        &mut self,
        title: String,
        cx: &mut Context<AppState>,
    ) {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, cx| {
            let page_shell = surface
                .board
                .page_shell
                .as_ref()
                .expect("collapsible page section requires a page shell");
            let (index, section) = page_shell
                .sidebar_sections
                .iter()
                .enumerate()
                .find(|(_, section)| section.title == title)
                .expect("collapsible page section title must resolve");
            let key = crate::ui::NotionSidebarSectionKey::new(
                section.identity.as_ref(),
                index,
                &section.title,
            );
            let changed = surface.notion_sidebar.collapsed_sections.insert(key);
            if changed {
                surface
                    .notion_sidebar
                    .rebuild_rows(surface.board.page_shell.as_ref());
            }
            cx.notify();
        });
    }

    #[doc(hidden)]
    pub fn board<AppState: 'static>(&mut self, cx: &mut Context<AppState>) -> BoardSnapshot {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, _cx| surface.board.clone())
    }

    #[doc(hidden)]
    pub fn standalone_page_content_loaded<AppState: 'static>(
        &mut self,
        cx: &mut Context<AppState>,
    ) -> bool {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, _cx| {
            surface.page_documents.standalone.is_some()
        })
    }

    #[doc(hidden)]
    pub fn sidebar_visible<AppState: 'static>(&mut self, cx: &mut Context<AppState>) -> bool {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, _cx| {
            surface.notion_chrome.notion_sidebar_visible
        })
    }

    #[doc(hidden)]
    pub fn page_section_collapsed<AppState: 'static>(
        &mut self,
        title: &str,
        cx: &mut Context<AppState>,
    ) -> bool {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, _cx| {
            let page_shell = surface
                .board
                .page_shell
                .as_ref()
                .expect("collapsed page section requires a page shell");
            let (index, section) = page_shell
                .sidebar_sections
                .iter()
                .enumerate()
                .find(|(_, section)| section.title == title)
                .expect("collapsed page section title must resolve");
            let key = crate::ui::NotionSidebarSectionKey::new(
                section.identity.as_ref(),
                index,
                &section.title,
            );
            surface.notion_sidebar.collapsed_sections.contains(&key)
        })
    }

    #[doc(hidden)]
    pub fn pending_navigation<AppState: 'static>(
        &mut self,
        cx: &mut Context<AppState>,
    ) -> Option<NotionPendingNavigation> {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |surface, _cx| {
            surface.notion_chrome.notion_pending_navigation.clone()
        })
    }

    #[doc(hidden)]
    pub fn navigation_error<AppState: 'static>(
        &mut self,
        cx: &mut Context<AppState>,
    ) -> Option<String> {
        let surface = self.ensure_surface(cx);
        surface.update(cx, |_surface, _cx| None)
    }

    fn ensure_surface(&mut self, cx: &mut App) -> gpui::Entity<SurfaceState> {
        if let Some(surface) = self.surface.as_ref() {
            return surface.clone();
        }
        let route = self.pending_route.take();
        let appearance_mode = AppearanceMode::current(cx);
        let surface = cx.new(|_| {
            let mut surface = SurfaceState::from_startup(
                self.startup.clone(),
                appearance_mode,
                self.viewport,
                self.active,
                self.notion_resources.clone(),
            );
            if let Some(route) = route {
                let _ = surface.restore_notion_route(&route);
            }
            surface
        });
        surface.update(cx, {
            let surface = surface.clone();
            move |state, cx| state.presentation.initialize_cached_regions(surface, cx)
        });
        self.surface = Some(surface.clone());
        surface
    }
}

impl AppSurfaceRoot for SurfaceRoot {
    fn set_surface_frame(&mut self, frame: SurfaceFrame) {
        self.viewport = frame.viewport;
        self.preview_width = frame.preview_width;
    }

    fn activate_surface(&mut self, frame: SurfaceFrame, cx: &mut App) {
        self.set_surface_frame(frame);
        self.ensure_surface_state(frame.active, cx);
    }

    fn deactivate_surface(&mut self, cx: &mut App) {
        self.active = false;
        if let Some(surface) = self.surface.as_ref() {
            surface.update(cx, |surface, _cx| {
                surface.surface_active = false;
            });
        }
    }

    fn render_surface_preview(&mut self, frame: SurfaceFrame, cx: &mut App) -> Option<AnyElement> {
        self.activate_surface(frame, cx);
        frame.active.then(|| self.render_standalone(cx))
    }

    fn render_surface_standalone(
        &mut self,
        frame: SurfaceFrame,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.activate_surface(frame, cx);
        Some(self.render_standalone(cx))
    }

    fn render_surface(&mut self, _window: &mut Window, cx: &mut App) -> AnyElement {
        self.ensure_surface_state(true, cx);
        self.ensure_surface(cx).into_any_element()
    }
}
