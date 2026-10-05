use crate::ui::surface::{PageDocumentDragRuntime, PageDocumentFlowRuntime};
use crate::ui::view_actions::ViewNotifier;
use gpui::App;

use super::geometry::{
    page_flow_surface, resolve_page_block_drag_target, resolve_page_flow_drag_target,
};
use super::{
    Arc, LoadedCardPageData, PageBlockDragGeometryMode, PageBlockDragObservation,
    PageBlockDragPayload, PageBlockDragScrollTarget, PageBlockDragTarget, Pixels, Point,
};

pub(in crate::ui::board_workspace::page::editor) struct PageBlockDropRequest<'a> {
    pub(in crate::ui::board_workspace::page::editor) scroll_target: PageBlockDragScrollTarget,
    pub(in crate::ui::board_workspace::page::editor) data: &'a Arc<LoadedCardPageData>,
    pub(in crate::ui::board_workspace::page::editor) pointer: Point<Pixels>,
    pub(in crate::ui::board_workspace::page::editor) payload: &'a PageBlockDragPayload,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageBlockDropRuntime {
    drag: PageDocumentDragRuntime,
    flow: PageDocumentFlowRuntime,
    notifier: ViewNotifier,
}

struct PageBlockDropUpdate {
    can_drop: bool,
    target_changed: bool,
}

impl PageBlockDropRuntime {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        drag: PageDocumentDragRuntime,
        flow: PageDocumentFlowRuntime,
        notifier: ViewNotifier,
    ) -> Self {
        Self {
            drag,
            flow,
            notifier,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn can_drop(
        &self,
        request: PageBlockDropRequest<'_>,
        cx: &mut App,
    ) -> bool {
        let update = self.drag.update_target_from_pointer(&self.flow, request);
        if update.target_changed {
            self.notifier.notify(cx);
        }
        update.can_drop
    }
}

impl PageDocumentDragRuntime {
    fn update_target_from_pointer(
        &self,
        flow: &PageDocumentFlowRuntime,
        request: PageBlockDropRequest<'_>,
    ) -> PageBlockDropUpdate {
        let container_bounds = self
            .borrow()
            .layouts
            .get(request.scroll_target)
            .container_bounds;
        let Some(container_bounds) = container_bounds else {
            return PageBlockDropUpdate {
                can_drop: false,
                target_changed: self.replace_target(None),
            };
        };
        let observation = PageBlockDragObservation {
            scroll_target: request.scroll_target,
            pointer: request.pointer,
            container_bounds,
            payload: request.payload.clone(),
        };
        self.borrow_mut()
            .layouts
            .get_mut(request.scroll_target)
            .observation = Some(observation);
        let next_target = self.resolve_current_target(flow, request.scroll_target, request.data);
        let can_drop = next_target
            .as_ref()
            .is_some_and(|target| target.matches(request.scroll_target, request.payload));
        PageBlockDropUpdate {
            can_drop,
            target_changed: self.replace_target(next_target),
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn resolve_current_target(
        &self,
        flow: &PageDocumentFlowRuntime,
        scroll_target: PageBlockDragScrollTarget,
        data: &Arc<LoadedCardPageData>,
    ) -> Option<PageBlockDragTarget> {
        let drag = self.borrow();
        let layout = drag.layouts.get(scroll_target);
        let observation = layout.observation.as_ref()?;
        let mode = PageBlockDragGeometryMode::for_data(data);
        if mode != PageBlockDragGeometryMode::for_data(&observation.payload.source.data) {
            return None;
        }
        match mode {
            PageBlockDragGeometryMode::ExistingLinear => {
                resolve_page_block_drag_target(data, layout)
            }
            PageBlockDragGeometryMode::RecursiveFlow => {
                let observations = flow.state().observations.borrow();
                let authority =
                    observations.drag_authority(page_flow_surface(scroll_target), data)?;
                resolve_page_flow_drag_target(data, &authority, observation)
            }
        }
    }

    fn replace_target(&self, target: Option<PageBlockDragTarget>) -> bool {
        let mut drag = self.borrow_mut();
        if drag.target == target {
            return false;
        }
        drag.target = target;
        true
    }
}
