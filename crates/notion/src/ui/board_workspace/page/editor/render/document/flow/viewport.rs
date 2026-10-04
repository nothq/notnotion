use gpui::ListState;

use crate::ui::surface::{PageFlowLayoutWidth, PageFlowViewport};

use super::super::PageDocumentColumn;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor::render::document) struct PageFlowOuterListFrame {
    viewport_height: f32,
    logical_item_index: usize,
    logical_offset: f32,
    layout_width: PageFlowLayoutWidth,
}

impl PageFlowOuterListFrame {
    pub(in crate::ui::board_workspace::page::editor::render::document) fn capture(
        list_state: &ListState,
        item_count: usize,
        column: PageDocumentColumn,
    ) -> Self {
        assert_eq!(list_state.item_count(), item_count);
        let viewport = list_state.viewport_bounds();
        let logical = list_state.logical_scroll_top();
        Self {
            viewport_height: viewport.size.height.as_f32(),
            logical_item_index: logical.item_ix,
            logical_offset: logical.offset_in_item.as_f32(),
            layout_width: page_flow_layout_width(column, viewport.size.width.as_f32()),
        }
    }

    pub(in crate::ui::board_workspace::page::editor::render::document) fn item_viewport(
        &self,
        item_index: usize,
    ) -> PageFlowViewport {
        if self.viewport_height > 0.0 && self.logical_item_index == item_index {
            return PageFlowViewport::known(
                self.logical_offset,
                self.logical_offset + self.viewport_height,
            );
        }
        if self.viewport_height > 0.0 && item_index > self.logical_item_index {
            return PageFlowViewport::known(0.0, self.viewport_height);
        }
        if self.logical_item_index == item_index {
            return PageFlowViewport::unknown_at_anchor(self.logical_offset);
        }
        PageFlowViewport::unknown_leading()
    }

    pub(in crate::ui::board_workspace::page::editor::render::document) const fn layout_width(
        &self,
    ) -> PageFlowLayoutWidth {
        self.layout_width
    }
}

fn page_flow_layout_width(column: PageDocumentColumn, observed_pane: f32) -> PageFlowLayoutWidth {
    let provisional = if observed_pane > 0.0 {
        column.width_in(observed_pane)
    } else {
        column.width()
    };
    PageFlowLayoutWidth::from_estimated_pixels(provisional)
}

pub(super) fn translate_page_flow_viewport(
    viewport: PageFlowViewport,
    content_origin: f32,
) -> PageFlowViewport {
    assert!(content_origin.is_finite());
    match viewport {
        PageFlowViewport::Known { start, end } => {
            PageFlowViewport::known(start - content_origin, end - content_origin)
        }
        PageFlowViewport::UnknownLeading => PageFlowViewport::unknown_leading(),
        PageFlowViewport::UnknownAtAnchor { offset } => {
            PageFlowViewport::unknown_at_anchor((offset - content_origin).max(0.0))
        }
    }
}

pub(super) fn inset_page_flow_width(
    width: PageFlowLayoutWidth,
    horizontal_insets: f32,
) -> PageFlowLayoutWidth {
    assert!(horizontal_insets.is_finite() && horizontal_insets >= 0.0);
    PageFlowLayoutWidth::from_estimated_pixels((width.pixels() - horizontal_insets).max(0.0))
}
