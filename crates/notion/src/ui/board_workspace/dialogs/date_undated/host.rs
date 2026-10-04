use gpui::{AnyElement, Context, IntoElement, Window};

use super::super::super::InlineDatabaseToolbarTarget;
use super::{
    Arc, DateUndatedAction, DateUndatedDialogCommand, DateUndatedDialogSource, DateViewContext,
    DateViewEffect,
};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{KeyDownEvent, SurfaceState};

mod query;
mod render;
mod state;

impl SurfaceState {
    pub(crate) fn toggle_date_undated_dialog_from_toolbar(
        &mut self,
        inline_target: Option<InlineDatabaseToolbarTarget>,
        cx: &mut Context<Self>,
    ) {
        let anchor = self
            .date_view
            .undated_badge_bounds
            .expect("Notion no-date badge must be laid out before activation");
        let source = match inline_target.as_ref() {
            Some(_) => DateUndatedDialogSource::Inline {
                session: self.date_view.query_session.clone(),
                surface: cx.entity().downgrade(),
            },
            None => DateUndatedDialogSource::Full {
                session: self.date_view.query_session.clone(),
            },
        };
        let effect =
            DateViewEffect::ToggleUndatedDialog(DateUndatedDialogCommand { anchor, source });
        if let Some(target) = inline_target {
            let parent_surface = target.parent_surface;
            cx.defer(move |cx| {
                parent_surface
                    .update(cx, move |parent, cx| {
                        parent.execute_date_view_effects(vec![effect], cx);
                    })
                    .expect("inline no-date dialog requires its parent surface");
            });
        } else {
            self.execute_date_view_effects(vec![effect], cx);
        }
    }

    pub(crate) fn date_undated_dialog_open_for_source(
        &self,
        inline_target: Option<&InlineDatabaseToolbarTarget>,
        cx: &Context<Self>,
    ) -> bool {
        let session = &self.date_view.query_session;
        match inline_target {
            Some(target) => target.parent_surface.upgrade().is_some_and(|parent| {
                parent
                    .read(cx)
                    .notion_chrome
                    .date_undated_dialog
                    .as_ref()
                    .is_some_and(|state| state.source.matches_session(session))
            }),
            None => self
                .notion_chrome
                .date_undated_dialog
                .as_ref()
                .is_some_and(|state| state.source.matches_session(session)),
        }
    }

    pub(crate) fn handle_date_undated_dialog_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.keystroke.key != "escape" || self.notion_chrome.date_undated_dialog.is_none() {
            return false;
        }
        self.execute_date_view_effects(vec![DateViewEffect::CloseHostedUndatedDialog], cx);
        true
    }

    pub(crate) fn render_date_undated_dialog_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .notion_chrome
            .date_undated_dialog
            .as_ref()
            .expect("Notion no-date overlay requires dialog state")
            .clone();
        let viewport_width = self.viewport.app_width();
        let viewport_height = self.viewport.app_height();
        let geometry = render::DateUndatedOverlayGeometry {
            anchor: state.anchor,
            viewport_width,
            viewport_height,
        };
        let rendered = match &state.source {
            DateUndatedDialogSource::Full { session }
                if Arc::ptr_eq(session, &self.date_view.query_session)
                    && self.date_view.undated.open =>
            {
                Some(
                    render::DateUndatedRenderSource {
                        state: &self.date_view.undated,
                        theme: self.theme,
                        appearance_mode: self.appearance_mode,
                        page_icons: self.page_shell_icon_renderer(cx),
                        active_view_kind: self.board.active_date_view().map(|view| view.kind),
                        workspace_available: self.notion_startup.workspace_api().is_some(),
                    }
                    .render(session.clone(), geometry, cx),
                )
            }
            DateUndatedDialogSource::Inline { session, surface } => {
                render::resolve_inline_date_undated_dialog(geometry, session, surface, cx)
            }
            _ => None,
        };
        rendered.unwrap_or_else(|| {
            render::defer_clear_stale_date_undated_dialog(state.source.session().clone(), cx);
            gpui::Empty.into_any_element()
        })
    }
}

fn handle_date_undated_action(
    surface: &mut SurfaceState,
    action: DateUndatedAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let workspace_available = surface.notion_startup.workspace_api().is_some();
    let context = DateViewContext::from_board(&surface.board, workspace_available);
    let effect = surface.date_view.undated.reduce(action);
    let effects = surface.date_view.resolve_undated_effect(effect, &context);
    surface.execute_date_view_effects(effects, cx);
}

fn date_undated_action_sink(
    cx: &Context<SurfaceState>,
    session: Arc<()>,
) -> ViewActionSink<DateUndatedAction> {
    ViewActionSink::new(cx, move |surface, action, window, cx| {
        if !Arc::ptr_eq(&surface.date_view.query_session, &session)
            || !surface.date_view.undated.open
        {
            return;
        }
        handle_date_undated_action(surface, action, window, cx);
    })
}
