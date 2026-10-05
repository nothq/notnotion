use gpui::{App, AppContext, Entity, Pixels, Point, Window};

use super::super::super::{CardPageBlockColor, CardPageBlockColorValue, PageBlockDragPayload};
use super::super::PageBlockRenderer;
use super::PageBlockRenderContext;
use std::sync::Arc;

impl PageBlockRenderer {
    pub(super) fn page_block_drag_payload(
        &self,
        context: &PageBlockRenderContext<'_>,
        cx: &mut App,
    ) -> PageBlockDragPayload {
        let data = context.data.clone();
        let index = context.index;
        let block = context.block;
        let selection = context.drag_selection.clone();
        PageBlockDragPayload {
            source: super::super::super::drag::PageBlockDragSource {
                data: data.clone(),
                dragged_index: index,
                dragged_block_id: block.block_id.clone(),
                selection,
                block_images: self.resources.block_image_cache_shared(),
            },
            resolved: Arc::new(std::sync::OnceLock::new()),
            preview_width: super::super::super::drag::PageBlockDragPreviewWidth::for_data(
                &data,
                self.column.width(),
            ),
            cursor_offset: gpui::point(gpui::px(0.0), gpui::px(0.0)),
            appearance_mode: self.appearance_mode,
            toggle_markers: CardPageBlockColorValue::ALL.map(|value| {
                self.icons
                    .page_toggle_marker(false, CardPageBlockColor::Text(value))
                    .render(cx)
            }),
            page_marker: self.icons.page.render(cx),
            checked_marker: self.icons.page_checkbox_checked.render(cx),
        }
    }
}

pub(super) fn page_block_drag_preview(
    dragged: &PageBlockDragPayload,
    cursor_offset: Point<Pixels>,
    _: &mut Window,
    cx: &mut App,
) -> Entity<PageBlockDragPayload> {
    cx.stop_propagation();
    dragged.resolve();
    let mut preview = dragged.clone();
    preview.cursor_offset = cursor_offset;
    cx.new(|_| preview)
}
