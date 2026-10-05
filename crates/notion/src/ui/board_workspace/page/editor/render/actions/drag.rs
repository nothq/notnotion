use std::sync::Arc;

use gpui::{Bounds, Context, Pixels, Point, Window};

use crate::ui::{LoadedCardPageData, PageBlockDragScrollTarget, SurfaceState};

use super::super::super::{PageBlockDragObservation, PageBlockDragPayload, PageColumnResizeDrag};

pub(in crate::ui::board_workspace) struct PageLinearRowsObservation {
    pub(in crate::ui::board_workspace) scroll_target: PageBlockDragScrollTarget,
    pub(in crate::ui::board_workspace) data: Arc<LoadedCardPageData>,
    pub(in crate::ui::board_workspace) document_unit_indices: Vec<usize>,
    pub(in crate::ui::board_workspace) row_bounds: Vec<Bounds<Pixels>>,
    pub(in crate::ui::board_workspace) generation: u64,
    pub(in crate::ui::board_workspace) active_drag: bool,
}

pub(in crate::ui::board_workspace) enum PageDragRenderAction {
    BeginBlockHandle,
    FinishBlockHandle(String),
    UpdateBlock {
        data: Arc<LoadedCardPageData>,
        observation: PageBlockDragObservation,
    },
    FinishBlock {
        scroll_target: PageBlockDragScrollTarget,
        payload: PageBlockDragPayload,
        pointer: Point<Pixels>,
    },
    BeginColumnResize {
        drag: PageColumnResizeDrag,
        pointer_x: f32,
    },
    UpdateColumnResize {
        drag: PageColumnResizeDrag,
        pointer_x: f32,
    },
    FinishColumnResize {
        drag: PageColumnResizeDrag,
        pointer_x: f32,
    },
    ObserveLinearRows(PageLinearRowsObservation),
}

pub(super) fn handle(
    surface: &mut SurfaceState,
    action: PageDragRenderAction,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageDragRenderAction::BeginBlockHandle => {
            surface.page_editor.drag.borrow_mut().cancelled = false
        }
        PageDragRenderAction::FinishBlockHandle(block_id) => {
            surface.finish_page_block_handle_interaction(block_id, window, cx)
        }
        PageDragRenderAction::UpdateBlock { data, observation } => {
            surface.update_page_block_drag_from_observation(&data, observation, cx)
        }
        PageDragRenderAction::FinishBlock {
            scroll_target,
            payload,
            pointer,
        } => surface.finish_page_block_drag(scroll_target, &payload, pointer, cx),
        PageDragRenderAction::BeginColumnResize { drag, pointer_x } => {
            surface.begin_page_column_resize(&drag, pointer_x, cx);
        }
        PageDragRenderAction::UpdateColumnResize { drag, pointer_x } => {
            surface.update_page_column_resize_from_pointer(&drag, pointer_x, cx)
        }
        PageDragRenderAction::FinishColumnResize { drag, pointer_x } => {
            surface.finish_page_column_resize(&drag, pointer_x, cx)
        }
        PageDragRenderAction::ObserveLinearRows(observation) => {
            observe_linear_rows(surface, observation, cx)
        }
    }
}

fn observe_linear_rows(
    surface: &mut SurfaceState,
    observation: PageLinearRowsObservation,
    cx: &mut Context<SurfaceState>,
) {
    surface.page_editor.observe_page_block_drag_rows(
        super::super::super::drag::PageBlockDragRowsObservation {
            scroll_target: observation.scroll_target,
            data: &observation.data,
            document_unit_indices: observation.document_unit_indices,
            row_bounds: observation.row_bounds,
            generation: observation.generation,
            active_drag: observation.active_drag,
        },
        cx,
    )
}
