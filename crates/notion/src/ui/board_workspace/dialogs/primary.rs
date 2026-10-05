use std::sync::Arc;

use gpui::{Context, Pixels, SharedString, Window};

use super::super::database_view_controls::{
    database_view_controls_renderer, DatabaseViewControlPlacement,
};
use super::super::{
    alpha, div, point, px, rgb, rgba, AnyElement, AppearanceMode, BoxShadow, Div, IntoElement,
    Styled, SurfaceState, Theme, ToolbarDialogKind,
};
use super::filter::{database_filter_renderer, DatabaseFilterDialogPlacement, DatabaseFilterHost};
use crate::ui::{view_actions::ViewActionSink, IconSet};

#[derive(Clone, Copy)]
pub(super) enum ToolbarDialogAction {
    Close,
}

pub(super) struct ToolbarDialogRenderer {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) database_title: SharedString,
    pub(super) actions: ViewActionSink<ToolbarDialogAction>,
}

impl SurfaceState {
    pub(crate) fn render_toolbar_dialog(
        &self,
        dialog: ToolbarDialogKind,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let renderer = ToolbarDialogRenderer {
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            icons: self.icons.clone(),
            database_title: self.board.database_title.clone().into(),
            actions: ViewActionSink::new(cx, handle_toolbar_dialog_action),
        };
        match dialog {
            ToolbarDialogKind::Actions => renderer.render_actions_dialog(cx),
            ToolbarDialogKind::Filter => {
                database_filter_renderer(self, DatabaseFilterHost::FullPage, cx)
                    .render_database_filter_dialog(DatabaseFilterDialogPlacement::FullPage, cx)
            }
            ToolbarDialogKind::Sort => database_view_controls_renderer(self, cx)
                .render_database_sort_controls(DatabaseViewControlPlacement::FullPage {
                    top: 168.0,
                    right: 27.0,
                }),
            ToolbarDialogKind::Automations => renderer.render_automation_dialog(cx),
            ToolbarDialogKind::Templates => renderer.render_templates_dialog(cx),
            ToolbarDialogKind::Properties => database_view_controls_renderer(self, cx)
                .render_database_properties_controls(DatabaseViewControlPlacement::FullPage {
                    top: 168.0,
                    right: 27.0,
                }),
        }
        .into_any_element()
    }
}

impl ToolbarDialogRenderer {
    pub(super) fn render_dialog_surface(&self, width: Pixels, top: f32, right: f32) -> Div {
        div()
            .absolute()
            .top(px(top))
            .right(px(right))
            .w(width)
            .rounded(px(12.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(vec![BoxShadow {
                color: alpha(
                    0x000000,
                    if self.appearance_mode == AppearanceMode::Light {
                        0.12
                    } else {
                        0.28
                    },
                ),
                offset: point(px(0.0), px(22.0)),
                blur_radius: px(40.0),
                spread_radius: px(-14.0),
                inset: false,
            }])
    }
}

fn handle_toolbar_dialog_action(
    surface: &mut SurfaceState,
    action: ToolbarDialogAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let ToolbarDialogAction::Close = action;
    surface.notion_chrome.close_toolbar_dialog(cx);
}
