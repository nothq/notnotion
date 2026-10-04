use super::{
    px, Context, ScrollDelta, ScrollWheelEvent, SurfaceState, Window, BOARD_SCROLL_LINE_MULTIPLIER,
    BOARD_SCROLL_PIXEL_MULTIPLIER,
};

impl SurfaceState {
    pub(crate) fn handle_board_scroll(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.page_documents.selected_page.is_some() {
            return;
        }

        let horizontal_delta = match event.delta {
            ScrollDelta::Pixels(delta) => {
                if delta.x != px(0.0) {
                    return;
                }
                delta.y.as_f32() * BOARD_SCROLL_PIXEL_MULTIPLIER
            }
            ScrollDelta::Lines(delta) => {
                if delta.x != 0.0 {
                    return;
                }
                delta.y * BOARD_SCROLL_LINE_MULTIPLIER
            }
        };

        if horizontal_delta == 0.0 {
            return;
        }

        let max_scroll_x = self.board_viewport().max_scroll_x();
        let target_scroll_x =
            (self.board_view.scroll_x(max_scroll_x) - horizontal_delta).clamp(0.0, max_scroll_x);
        self.board_view.set_scroll_x(target_scroll_x, max_scroll_x);
        cx.notify();
    }
}
