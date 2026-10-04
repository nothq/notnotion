use std::sync::Arc;

use gpui::{Bounds, Context, Pixels, SharedString, Styled, Window};

use crate::model::{
    BoardSnapshot, DatabaseViewControlMutationRequest, DatabaseViewGroupState,
    NotionCollectionViewId, ViewTabKind,
};
use crate::ui::surface::{DatabaseViewControlPropertyRow, DatabaseViewControlsState, SurfaceState};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::Theme;

mod properties;
mod renderer;
mod sort;

use renderer::DatabaseViewControlsRenderer;

const CONTROL_DIALOG_WIDTH: f32 = 320.0;
const CONTROL_DIALOG_MAX_HEIGHT: f32 = 440.0;
const CONTROL_DIALOG_MARGIN: f32 = 12.0;
const CONTROL_DIALOG_HEADER_HEIGHT: f32 = 48.0;
const CONTROL_DIALOG_ROW_HEIGHT: f32 = 34.0;
const CONTROL_UPDATE_FAILURE: &str = "database view control update failed";

#[derive(Clone, Copy)]
pub(super) enum DatabaseViewControlPlacement {
    FullPage {
        top: f32,
        right: f32,
    },
    Inline {
        anchor: Bounds<Pixels>,
        viewport_width: f32,
        viewport_height: f32,
    },
}

#[derive(Clone, Copy)]
pub(super) enum DatabaseViewControlHost {
    FullPage,
    Inline,
}

#[derive(Clone)]
pub(super) enum DatabaseViewControlAction {
    Submit {
        request: DatabaseViewControlMutationRequest,
        host: DatabaseViewControlHost,
    },
}

#[derive(Clone)]
pub(super) struct DatabaseViewControlsSnapshot {
    property_rows: Arc<[DatabaseViewControlPropertyRow]>,
    active_view_id: Option<NotionCollectionViewId>,
    active_view_group: DatabaseViewGroupState,
    active_view_kind: ViewTabKind,
    sort_order: Arc<[SharedString]>,
    can_mutate: bool,
}

pub(super) fn database_view_controls_renderer(
    surface: &SurfaceState,
    cx: &Context<SurfaceState>,
) -> DatabaseViewControlsRenderer {
    let active_view_id = active_view_id(&surface.board);
    let sort_order = surface
        .board
        .active_view_sorts
        .as_slice()
        .iter()
        .map(|sort| SharedString::from(sort.property_id().as_str().to_owned()))
        .collect::<Vec<_>>()
        .into();
    DatabaseViewControlsRenderer::new(
        DatabaseViewControlsSnapshot {
            property_rows: surface.database_view_controls.property_rows.clone(),
            active_view_id,
            active_view_group: surface.board.active_view_group.clone(),
            active_view_kind: surface.board.active_view_kind(),
            sort_order,
            can_mutate: controls_can_mutate(
                &surface.board,
                &surface.database_view_controls,
                surface.notion_startup.workspace_api().is_some(),
            ),
        },
        surface.theme,
        ViewActionSink::new(cx, handle_database_view_control_action),
    )
}

fn handle_database_view_control_action(
    surface: &mut SurfaceState,
    action: DatabaseViewControlAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let DatabaseViewControlAction::Submit { request, host } = action;
    if !controls_can_mutate(
        &surface.board,
        &surface.database_view_controls,
        surface.notion_startup.workspace_api().is_some(),
    ) {
        return;
    }
    let Some(view_id) = active_view_id(&surface.board) else {
        surface.print_notion_error("database controls require an active Notion view");
        return;
    };
    let Some(workspace_api) = surface.notion_startup.workspace_api() else {
        surface.print_notion_error("database controls require a live Notion workspace API");
        return;
    };
    let Some(board_url) = surface.notion_startup.board_url() else {
        surface.print_notion_error("database controls require a live Notion database route");
        return;
    };
    let inline_parent = matches!(host, DatabaseViewControlHost::Inline)
        .then(|| surface.presentation.page_host.clone())
        .flatten();
    surface.database_view_controls.mutation_in_flight = true;
    cx.notify();
    surface.spawn_background_task(
        (workspace_api, request, board_url),
        cx,
        |(workspace_api, request, board_url)| {
            workspace_api.mutate_database_view_control(request)?;
            workspace_api.load_notion_workspace(&board_url)
        },
        move |surface, result, cx| {
            surface.database_view_controls.mutation_in_flight = false;
            match result {
                Ok(load) => {
                    if let Err(error) = surface.replace_database_filter_projection(&view_id, load) {
                        surface.print_notion_error(error);
                    }
                }
                Err(error) => {
                    if surface.handle_notion_workspace_failure(CONTROL_UPDATE_FAILURE, error, cx) {
                        return;
                    }
                }
            }
            if let Some(parent) = inline_parent.as_ref() {
                let _ = parent.update(cx, |_, cx| cx.notify());
            }
            cx.notify();
        },
    );
}

fn active_view_id(board: &BoardSnapshot) -> Option<NotionCollectionViewId> {
    board
        .view_tabs
        .iter()
        .find(|view| view.active)
        .map(|view| view.provider_view_id.clone())
}

fn controls_can_mutate(
    board: &BoardSnapshot,
    state: &DatabaseViewControlsState,
    workspace_available: bool,
) -> bool {
    !board.is_locked && !state.mutation_in_flight && workspace_available
}

fn position_inline_database_control_dialog(
    dialog: gpui::Stateful<gpui::Div>,
    anchor: Bounds<Pixels>,
    viewport_width: f32,
    viewport_height: f32,
    desired_height: f32,
) -> gpui::Stateful<gpui::Div> {
    let left = anchor.left().as_f32().clamp(
        CONTROL_DIALOG_MARGIN,
        (viewport_width - CONTROL_DIALOG_WIDTH - CONTROL_DIALOG_MARGIN).max(CONTROL_DIALOG_MARGIN),
    );
    let below = anchor.bottom().as_f32() + 4.0;
    let above = anchor.top().as_f32() - desired_height - 4.0;
    let top = if below + desired_height <= viewport_height - CONTROL_DIALOG_MARGIN {
        below
    } else {
        above.max(CONTROL_DIALOG_MARGIN)
    };
    dialog.left(gpui::px(left)).top(gpui::px(top))
}

#[derive(Clone, Copy)]
struct DatabaseViewControlsResources {
    theme: Theme,
    host: DatabaseViewControlHost,
}
