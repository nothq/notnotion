use gpui::{AnyElement, Bounds, Context, Pixels, WeakEntity};

use super::super::{
    AppearanceMode, Arc, DateUndatedDialogView, DateUndatedItemsState, PageShellIconRenderer, Theme,
};
use crate::ui::{SurfaceState, ViewTabKind};

pub(super) struct DateUndatedRenderSource<'a> {
    pub(super) state: &'a DateUndatedItemsState,
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) active_view_kind: Option<ViewTabKind>,
    pub(super) workspace_available: bool,
}

#[derive(Clone, Copy)]
pub(super) struct DateUndatedOverlayGeometry {
    pub(super) anchor: Bounds<Pixels>,
    pub(super) viewport_width: f32,
    pub(super) viewport_height: f32,
}

impl DateUndatedRenderSource<'_> {
    pub(super) fn render(
        self,
        session: Arc<()>,
        geometry: DateUndatedOverlayGeometry,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let instruction = match self.active_view_kind {
            Some(ViewTabKind::Calendar) if self.workspace_available => {
                "Click or drag to calendar to add date."
            }
            Some(ViewTabKind::Timeline) => "Click to add to the timeline",
            _ => "Click to add date.",
        };
        DateUndatedDialogView {
            state: self.state,
            anchor: geometry.anchor,
            viewport_width: geometry.viewport_width,
            viewport_height: geometry.viewport_height,
            instruction,
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            page_icons: self.page_icons,
            drag_enabled: self.active_view_kind == Some(ViewTabKind::Calendar)
                && self.workspace_available,
            actions: super::date_undated_action_sink(cx, session),
        }
        .render(cx)
    }
}

pub(super) fn resolve_inline_date_undated_dialog(
    geometry: DateUndatedOverlayGeometry,
    session: &Arc<()>,
    source_surface: &WeakEntity<SurfaceState>,
    cx: &mut Context<SurfaceState>,
) -> Option<AnyElement> {
    let source_surface = source_surface.upgrade()?;
    let source_is_current = {
        let source = source_surface.read(cx);
        Arc::ptr_eq(session, &source.date_view.query_session) && source.date_view.undated.open
    };
    if !source_is_current {
        return None;
    }
    Some(source_surface.update(cx, |source, cx| {
        DateUndatedRenderSource {
            state: &source.date_view.undated,
            theme: source.theme,
            appearance_mode: source.appearance_mode,
            page_icons: source.page_shell_icon_renderer(cx),
            active_view_kind: source.board.active_date_view().map(|view| view.kind),
            workspace_available: source.notion_startup.workspace_api().is_some(),
        }
        .render(session.clone(), geometry, cx)
    }))
}

pub(super) fn defer_clear_stale_date_undated_dialog(
    session: Arc<()>,
    cx: &mut Context<SurfaceState>,
) {
    let host = cx.entity().downgrade();
    cx.defer(move |cx| {
        let _ = host.update(cx, |host, cx| {
            if host
                .notion_chrome
                .clear_date_undated_dialog_for_source_session(&session)
            {
                cx.notify();
            }
        });
    });
}
