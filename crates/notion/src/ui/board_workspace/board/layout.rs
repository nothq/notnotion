use super::drag::BoardDragLayout;
use crate::ui::{SurfaceState, BOARD_VIEWPORT_X, BOARD_WIDTH};

#[derive(Clone, Copy)]
pub(crate) struct BoardViewport {
    pub(crate) width: f32,
}

impl BoardViewport {
    pub(crate) fn max_scroll_x(self) -> f32 {
        (self.content_width() - self.width).max(0.0)
    }

    pub(crate) fn content_width(self) -> f32 {
        BOARD_VIEWPORT_X + BOARD_WIDTH
    }
}

impl SurfaceState {
    pub(crate) fn board_viewport(&self) -> BoardViewport {
        let width = if self.page_documents.selected_page.is_some() {
            (self.page_layout().main_pane_width() - crate::ui::board_workspace::selected_page_overlay::SelectedPageOverlayLayout::panel_width(
                self.page_layout().main_pane_width(),
            )).max(0.0)
        } else {
            self.page_layout().main_pane_width()
        };
        BoardViewport { width }
    }

    pub(crate) fn board_drag_layout(&self) -> BoardDragLayout<'_> {
        let viewport = self.board_viewport();
        BoardDragLayout {
            columns: &self.columns,
            viewport_width: viewport.width,
            scroll_x: self.board_view.scroll_x(viewport.max_scroll_x()),
            max_scroll_x: viewport.max_scroll_x(),
            surface_offset: self.page_layout().surface_leading_width()
                + self.page_layout().sidebar_layout_width(),
        }
    }
}
