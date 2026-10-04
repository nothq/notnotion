pub(super) use super::super::{
    alpha, div, point, px, rgb, AnyElement, AppearanceMode, BoxShadow, Context, Div, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState, Theme, ToolbarDialogKind,
};
pub(super) use super::filter::DatabaseFilterDialogPlacement;
pub(super) use gpui::{Bounds, Pixels, Role, SharedString};

const INLINE_DIALOG_MARGIN: f32 = 12.0;

mod automation;
mod lifecycle;
mod renderer;
mod surface;
mod templates;

use crate::ui::view_actions::ViewActionSink;
use renderer::InlineToolbarRenderer;

#[derive(Clone, Copy)]
enum InlineToolbarAction {
    Dismiss,
}

fn inline_toolbar_renderer(
    surface: &SurfaceState,
    actions: ViewActionSink<InlineToolbarAction>,
) -> InlineToolbarRenderer {
    InlineToolbarRenderer {
        theme: surface.theme,
        appearance_mode: surface.appearance_mode,
        database_title: surface.board.database_title.clone().into(),
        actions,
    }
}

fn inline_toolbar_shadow(appearance_mode: AppearanceMode) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(
                0x000000,
                if appearance_mode == AppearanceMode::Light {
                    0.12
                } else {
                    0.28
                },
            ),
            offset: point(px(0.0), px(14.0)),
            blur_radius: px(28.0),
            spread_radius: px(-6.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x000000, 0.12),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(4.0),
            spread_radius: px(-1.0),
            inset: false,
        },
    ]
}
