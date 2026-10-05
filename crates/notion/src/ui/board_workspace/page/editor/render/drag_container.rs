use super::super::drag::{PageBlockDropRequest, PageBlockDropRuntime};
use super::super::{
    App, Arc, Div, InteractiveElement, LoadedCardPageData, PageBlockDragObservation,
    PageBlockDragPayload, PageBlockDragScrollTarget, PageColumnResizeDrag,
};
use super::{PageBlockRenderer, PageDragRenderAction, PageRenderAction};

impl PageBlockRenderer {
    pub(in crate::ui::board_workspace) fn observe_page_block_drag_container_bounds(
        &self,
        container: Div,
        scroll_target: PageBlockDragScrollTarget,
    ) -> Div {
        let drag = self.drag.clone();
        container.on_children_prepainted(move |bounds, _, _| {
            let Some(bounds) = bounds.first().copied() else {
                return;
            };
            drag.borrow_mut()
                .layouts
                .get_mut(scroll_target)
                .container_bounds = Some(bounds);
        })
    }

    pub(in crate::ui::board_workspace) fn wire_page_block_drag_container(
        &self,
        container: gpui::Stateful<Div>,
        data: Arc<LoadedCardPageData>,
        scroll_target: PageBlockDragScrollTarget,
        _cx: &mut App,
    ) -> gpui::Stateful<Div> {
        let container = self.wire_page_block_drop_handlers(container, data, scroll_target);
        self.wire_page_column_resize_handlers(container)
    }

    fn wire_page_block_drop_handlers(
        &self,
        container: gpui::Stateful<Div>,
        data: Arc<LoadedCardPageData>,
        scroll_target: PageBlockDragScrollTarget,
    ) -> gpui::Stateful<Div> {
        let drag_data = data.clone();
        let can_drop_data = data;
        let drop_runtime =
            PageBlockDropRuntime::new(self.drag.clone(), self.flow.clone(), self.notifier.clone());
        let move_actions = self.actions.clone();
        let block_drop_actions = self.actions.clone();
        container
            .can_drop(move |dragged, window, cx| {
                if dragged.downcast_ref::<PageColumnResizeDrag>().is_some() {
                    return true;
                }
                let Some(dragged) = dragged.downcast_ref::<PageBlockDragPayload>() else {
                    return false;
                };
                drop_runtime.can_drop(
                    PageBlockDropRequest {
                        scroll_target,
                        data: &can_drop_data,
                        pointer: window.mouse_position(),
                        payload: dragged,
                    },
                    cx,
                )
            })
            .on_drag_move::<PageBlockDragPayload>(move |event, window, cx| {
                move_actions.emit(
                    PageRenderAction::Drag(PageDragRenderAction::UpdateBlock {
                        data: drag_data.clone(),
                        observation: PageBlockDragObservation {
                            scroll_target,
                            pointer: event.event.position,
                            container_bounds: event.bounds,
                            payload: event.drag(cx).clone(),
                        },
                    }),
                    window,
                    cx,
                );
            })
            .on_drop(move |dragged: &PageBlockDragPayload, window, cx| {
                block_drop_actions.emit(
                    PageRenderAction::Drag(PageDragRenderAction::FinishBlock {
                        scroll_target,
                        payload: dragged.clone(),
                        pointer: window.mouse_position(),
                    }),
                    window,
                    cx,
                );
            })
    }

    fn wire_page_column_resize_handlers(
        &self,
        container: gpui::Stateful<Div>,
    ) -> gpui::Stateful<Div> {
        let resize_move_actions = self.actions.clone();
        let resize_drop_actions = self.actions.clone();
        container
            .on_drag_move::<PageColumnResizeDrag>(move |event, window, cx| {
                resize_move_actions.emit(
                    PageRenderAction::Drag(PageDragRenderAction::UpdateColumnResize {
                        drag: event.drag(cx).clone(),
                        pointer_x: event.event.position.x.as_f32(),
                    }),
                    window,
                    cx,
                );
            })
            .on_drop(move |dragged: &PageColumnResizeDrag, window, cx| {
                cx.stop_propagation();
                resize_drop_actions.emit(
                    PageRenderAction::Drag(PageDragRenderAction::FinishColumnResize {
                        drag: dragged.clone(),
                        pointer_x: window.mouse_position().x.as_f32(),
                    }),
                    window,
                    cx,
                );
            })
    }
}
