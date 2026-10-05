use gpui::{Bounds, Context, Pixels, Point};

use super::{Arc, CardPeekState, LoadedCardPageData, PageBlockDragScrollTarget, SurfaceState};
use crate::ui::surface::PageEditorState;

mod drop_runtime;
mod geometry;
mod layout;
mod movement;
mod preview;
mod source;
mod types;

pub(in crate::ui::board_workspace::page::editor) use drop_runtime::{
    PageBlockDropRequest, PageBlockDropRuntime,
};
pub(super) use types::{
    PageBlockDragGeometryMode, PageBlockDragLayout, PageBlockDragObservation,
    PageBlockDragPreviewRows, PageBlockDragPreviewWidth, PageBlockDragSelection,
    PageBlockDragSource, PageBlockDragWash, ResolvedPageBlockDragPayload,
};
pub(crate) use types::{
    PageBlockDragLayouts, PageBlockDragPayload, PageBlockDragPreviewRow, PageBlockDragTarget,
};

use geometry::{page_flow_surface, resolve_page_block_drag_target, resolve_page_flow_drag_target};
use layout::merge_page_block_drag_rows;
use source::resolve_page_block_drag_payload;

pub(super) struct PageBlockDragRowsObservation<'a> {
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) data: &'a LoadedCardPageData,
    pub(super) document_unit_indices: Vec<usize>,
    pub(super) row_bounds: Vec<Bounds<Pixels>>,
    pub(super) generation: u64,
    pub(super) active_drag: bool,
}

impl SurfaceState {
    pub(crate) fn finish_page_block_handle_interaction(
        &mut self,
        block_id: String,
        window: &mut super::Window,
        cx: &mut Context<Self>,
    ) {
        if std::mem::take(&mut self.page_editor.drag.borrow_mut().cancelled) {
            cx.notify();
            return;
        }
        self.toggle_page_block_context_menu(block_id, window, cx);
    }

    pub(in crate::ui::board_workspace::page::editor) fn update_page_block_drag_from_observation(
        &mut self,
        data: &Arc<LoadedCardPageData>,
        observation: PageBlockDragObservation,
        cx: &mut Context<Self>,
    ) {
        self.update_page_block_drag_auto_scroll(&observation, cx);
        let scroll_target = observation.scroll_target;
        {
            let mut drag = self.page_editor.drag.borrow_mut();
            let layout = drag.layouts.get_mut(scroll_target);
            layout.container_bounds = Some(observation.container_bounds);
            layout.observation = Some(observation);
        }
        let next_target = self.page_editor.drag.resolve_current_target(
            &self.page_editor.flow,
            scroll_target,
            data,
        );
        self.page_editor.set_page_block_drag_target(next_target, cx);
    }

    pub(crate) fn update_page_block_hover(
        &mut self,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let scroll_target = match self.page_documents.selected_page.as_ref() {
            Some(CardPeekState::Loaded(_)) => Some(PageBlockDragScrollTarget::SelectedPage),
            Some(_) => None,
            None if self.board.page_shell.is_some() => Some(PageBlockDragScrollTarget::Standalone),
            None => None,
        };
        let next = scroll_target.and_then(|scroll_target| {
            self.page_editor
                .drag
                .borrow()
                .layouts
                .hovered_block_id(scroll_target, position)
        });
        if self.page_editor.hovered_page_block != next {
            self.page_editor.hovered_page_block = next;
            cx.notify();
        }
    }

    pub(crate) fn finish_page_block_drag(
        &mut self,
        scroll_target: PageBlockDragScrollTarget,
        dragged: &PageBlockDragPayload,
        pointer: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let target = self.finish_page_block_drag_target(scroll_target, dragged, pointer);
        self.page_editor.drag.borrow_mut().layouts = PageBlockDragLayouts::default();
        self.page_editor.stop_page_block_drag_auto_scroll();
        let Some(target) = target else {
            cx.notify();
            return;
        };
        if self
            .page_documents
            .active_page_data_with_id(&target.page_id)
            .is_none()
        {
            cx.notify();
            return;
        }
        self.move_page_block_subtrees(
            dragged.root_block_ids(),
            &target.target_parent_block_id,
            target.before_block_id.as_deref(),
            cx,
        );
    }

    fn finish_page_block_drag_target(
        &mut self,
        scroll_target: PageBlockDragScrollTarget,
        dragged: &PageBlockDragPayload,
        pointer: Point<Pixels>,
    ) -> Option<PageBlockDragTarget> {
        let stored = self.page_editor.drag.borrow_mut().target.take();
        let page_id = &dragged.source.data.page.block_id;
        let data = self.page_documents.active_page_data_with_id(page_id)?;
        let mode = PageBlockDragGeometryMode::for_data(&data);
        if mode != PageBlockDragGeometryMode::for_data(&dragged.source.data) {
            return None;
        }
        if mode == PageBlockDragGeometryMode::ExistingLinear {
            return stored.filter(|target| target.matches(scroll_target, dragged));
        }
        let container_bounds = self
            .page_editor
            .drag
            .borrow()
            .layouts
            .get(scroll_target)
            .container_bounds?;
        let observation = PageBlockDragObservation {
            scroll_target,
            pointer,
            container_bounds,
            payload: dragged.clone(),
        };
        let observations = self.page_editor.flow.state().observations.borrow();
        let authority = observations.drag_authority(page_flow_surface(scroll_target), &data)?;
        resolve_page_flow_drag_target(&data, &authority, &observation)
            .filter(|target| target.matches(scroll_target, dragged))
    }
}

impl PageEditorState {
    pub(super) fn observe_page_block_drag_rows(
        &mut self,
        observation: PageBlockDragRowsObservation<'_>,
        cx: &mut Context<SurfaceState>,
    ) {
        let PageBlockDragRowsObservation {
            scroll_target,
            data,
            document_unit_indices,
            row_bounds,
            generation,
            active_drag,
        } = observation;
        let mut drag = self.drag.borrow_mut();
        let layout = drag.layouts.get_mut(scroll_target);
        merge_page_block_drag_rows(layout, data, document_unit_indices, row_bounds, generation);
        if !active_drag {
            layout.observation = None;
            drop(drag);
            self.set_page_block_drag_target(None, cx);
            return;
        }
        let next_target = {
            layout
                .observation
                .as_ref()
                .map(|_| resolve_page_block_drag_target(data, layout))
        };
        drop(drag);
        if let Some(next_target) = next_target {
            self.set_page_block_drag_target(next_target, cx);
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn set_page_block_drag_target(
        &mut self,
        target: Option<PageBlockDragTarget>,
        cx: &mut Context<SurfaceState>,
    ) {
        let mut drag = self.drag.borrow_mut();
        if drag.target == target {
            return;
        }
        drag.target = target;
        cx.notify();
    }
}

impl PageBlockDragTarget {
    pub(super) fn matches(
        &self,
        scroll_target: PageBlockDragScrollTarget,
        dragged: &PageBlockDragPayload,
    ) -> bool {
        self.scroll_target == scroll_target
            && self.dragged_root_block_ids == dragged.root_block_ids()
    }
}

impl PageBlockDragPayload {
    pub(super) fn resolve(&self) -> &ResolvedPageBlockDragPayload {
        self.resolved
            .get_or_init(|| resolve_page_block_drag_payload(&self.source))
    }

    fn root_block_ids(&self) -> &[String] {
        &self.resolve().root_block_ids
    }

    fn subtree_contains(&self, block_id: &str) -> bool {
        self.resolve().subtree_membership.contains(block_id)
    }

    fn source_allocation_matches(&self, data: &Arc<LoadedCardPageData>) -> bool {
        self.resolve().source_count_proof.matches(data)
    }

    fn source_counts_allow_target(
        &self,
        data: &Arc<LoadedCardPageData>,
        target_parent_id: &str,
    ) -> bool {
        self.resolve()
            .source_count_proof
            .allows_target(data, target_parent_id)
    }
}
