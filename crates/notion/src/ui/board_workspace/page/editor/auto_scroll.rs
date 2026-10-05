use std::time::Duration;

use gpui::{px, Context};

use super::{CardPeekState, PageBlockDragObservation, SurfaceState};
use crate::ui::surface::PageEditorState;
use crate::ui::{PageBlockDragAutoScroll, PageBlockDragScrollTarget};

const PAGE_BLOCK_DRAG_SCROLL_EDGE: f32 = 72.0;
const PAGE_BLOCK_DRAG_SCROLL_MAX_SPEED: f32 = 18.0;
const PAGE_BLOCK_DRAG_SCROLL_TICK: Duration = Duration::from_millis(16);

impl SurfaceState {
    pub(in crate::ui::board_workspace::page::editor) fn update_page_block_drag_auto_scroll(
        &mut self,
        observation: &PageBlockDragObservation,
        cx: &mut Context<Self>,
    ) {
        let target = observation.scroll_target;
        let position = observation.pointer;
        let bounds = observation.container_bounds;
        let velocity = if position.x < bounds.left() || position.x > bounds.right() {
            None
        } else if position.y < bounds.top() + px(PAGE_BLOCK_DRAG_SCROLL_EDGE) {
            let pressure = ((bounds.top() + px(PAGE_BLOCK_DRAG_SCROLL_EDGE) - position.y)
                / px(PAGE_BLOCK_DRAG_SCROLL_EDGE))
            .clamp(0.0, 1.0);
            Some(-PAGE_BLOCK_DRAG_SCROLL_MAX_SPEED * pressure)
        } else if position.y > bounds.bottom() - px(PAGE_BLOCK_DRAG_SCROLL_EDGE) {
            let pressure = ((position.y - (bounds.bottom() - px(PAGE_BLOCK_DRAG_SCROLL_EDGE)))
                / px(PAGE_BLOCK_DRAG_SCROLL_EDGE))
            .clamp(0.0, 1.0);
            Some(PAGE_BLOCK_DRAG_SCROLL_MAX_SPEED * pressure)
        } else {
            None
        };
        let Some(velocity) = velocity.filter(|velocity| velocity.abs() >= 0.5) else {
            self.page_editor.stop_page_block_drag_auto_scroll();
            return;
        };

        let epoch = {
            let mut drag = self.page_editor.drag.borrow_mut();
            if let Some(scroll) = drag.auto_scroll.as_mut() {
                scroll.target = target;
                scroll.velocity = velocity;
                return;
            }
            drag.auto_scroll_epoch = drag.auto_scroll_epoch.wrapping_add(1);
            let epoch = drag.auto_scroll_epoch;
            drag.auto_scroll = Some(PageBlockDragAutoScroll {
                target,
                velocity,
                epoch,
            });
            epoch
        };
        self.advance_page_block_drag_auto_scroll(epoch, cx);
    }

    fn advance_page_block_drag_auto_scroll(&mut self, epoch: usize, cx: &mut Context<Self>) {
        let Some(scroll) = self
            .page_editor
            .drag
            .borrow()
            .auto_scroll
            .filter(|scroll| scroll.epoch == epoch)
        else {
            return;
        };
        if !cx.has_active_drag() {
            self.page_editor.stop_page_block_drag_auto_scroll();
            return;
        }

        let list_state = match scroll.target {
            PageBlockDragScrollTarget::Standalone => self
                .page_documents
                .standalone
                .as_ref()
                .map(|page| page.list_state.clone()),
            PageBlockDragScrollTarget::SelectedPage => self
                .page_documents
                .selected_page
                .as_ref()
                .and_then(|page| match page {
                    CardPeekState::Loaded(page) => Some(page.list_state.clone()),
                    CardPeekState::Loading { .. } | CardPeekState::Error { .. } => None,
                }),
        };
        let Some(list_state) = list_state else {
            self.page_editor.stop_page_block_drag_auto_scroll();
            return;
        };
        let previous_offset = list_state.scroll_px_offset_for_scrollbar();
        list_state.scroll_by(px(scroll.velocity));
        if list_state.scroll_px_offset_for_scrollbar() == previous_offset {
            self.page_editor.stop_page_block_drag_auto_scroll();
            return;
        }
        cx.notify();
        self.spawn_timer_task(epoch, PAGE_BLOCK_DRAG_SCROLL_TICK, cx, |this, epoch, cx| {
            this.advance_page_block_drag_auto_scroll(epoch, cx);
        });
    }
}

impl PageEditorState {
    pub(super) fn stop_page_block_drag_auto_scroll(&mut self) {
        let mut drag = self.drag.borrow_mut();
        if drag.auto_scroll.take().is_some() {
            drag.auto_scroll_epoch = drag.auto_scroll_epoch.wrapping_add(1);
        }
    }
}
