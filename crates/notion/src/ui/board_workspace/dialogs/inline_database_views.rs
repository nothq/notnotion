pub(super) use std::rc::Rc;

pub(super) use gpui::{AppContext, Bounds, Pixels, Role};
pub(super) use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

pub(super) use super::super::{
    alpha, div, img, point, px, rgb, AnyElement, AppearanceMode, BoxShadow, Context, FluentBuilder,
    FontWeight, IconAsset, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, SurfaceState, ViewTab,
    Window,
};
pub(super) use crate::ui::surface::{InlineDatabaseViewMenuSelection, InlineDatabaseViewMenuState};

const VIEW_MENU_WIDTH: f32 = 290.0;
const VIEW_MENU_INPUT_SECTION_HEIGHT: f32 = 48.0;
const VIEW_MENU_ROW_HEIGHT: f32 = 28.0;
const VIEW_MENU_LIST_PADDING: f32 = 4.0;
const VIEW_MENU_ACTION_SEPARATOR_HEIGHT: f32 = 9.0;
const VIEW_MENU_ACTION_GAP: f32 = 1.0;
const VIEW_MENU_MARGIN: f32 = 12.0;
const NEW_VIEW_PICKER_WIDTH: f32 = 390.0;
const NEW_VIEW_PICKER_HEIGHT: f32 = 234.0;

mod lifecycle;
mod picker;
mod render;
mod renderer;
mod selection;

#[derive(Clone, Copy)]
struct InlineDatabaseViewMenuLayout {
    left: f32,
    top: f32,
    height: f32,
    list_height: f32,
}

fn inline_database_view_menu_layout(
    state: &InlineDatabaseViewMenuState,
    viewport_width: f32,
    viewport_height: f32,
) -> InlineDatabaseViewMenuLayout {
    let filtered_count = filtered_inline_database_view_tabs(state).len();
    let desired_list_height = VIEW_MENU_LIST_PADDING * 2.0
        + filtered_count as f32 * VIEW_MENU_ROW_HEIGHT
        + VIEW_MENU_ACTION_SEPARATOR_HEIGHT
        + VIEW_MENU_ROW_HEIGHT * 2.0
        + VIEW_MENU_ACTION_GAP;
    let desired_height = VIEW_MENU_INPUT_SECTION_HEIGHT + desired_list_height;
    let height = desired_height.min((viewport_height - VIEW_MENU_MARGIN * 2.0).max(0.0));
    let list_height = (height - VIEW_MENU_INPUT_SECTION_HEIGHT).max(0.0);
    let left = state.anchor.left().as_f32().clamp(
        VIEW_MENU_MARGIN,
        (viewport_width - VIEW_MENU_WIDTH - VIEW_MENU_MARGIN).max(VIEW_MENU_MARGIN),
    );
    let below = state.anchor.bottom().as_f32() + 4.0;
    let above = state.anchor.top().as_f32() - height - 4.0;
    let top = if below + height <= viewport_height - VIEW_MENU_MARGIN {
        below
    } else {
        above.max(VIEW_MENU_MARGIN)
    };
    InlineDatabaseViewMenuLayout {
        left,
        top,
        height,
        list_height,
    }
}

fn filtered_inline_database_view_tabs(state: &InlineDatabaseViewMenuState) -> Vec<&ViewTab> {
    let query = state.query.to_lowercase();
    state
        .view_tabs
        .iter()
        .filter(|tab| query.is_empty() || tab.label.to_lowercase().contains(&query))
        .collect()
}

fn inline_database_view_menu_options(
    state: &InlineDatabaseViewMenuState,
) -> Vec<InlineDatabaseViewMenuSelection> {
    let mut options = filtered_inline_database_view_tabs(state)
        .into_iter()
        .map(|tab| InlineDatabaseViewMenuSelection::View(tab.provider_view_id.clone()))
        .collect::<Vec<_>>();
    options.push(InlineDatabaseViewMenuSelection::NewView);
    options.push(InlineDatabaseViewMenuSelection::NewDataSource);
    options
}

fn inline_database_view_menu_shadow(appearance_mode: AppearanceMode) -> Vec<BoxShadow> {
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
