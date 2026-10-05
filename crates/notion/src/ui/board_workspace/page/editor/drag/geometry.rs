use super::super::support::page_block_subtree_end;
use super::super::{
    LoadedCardPageData, PageBlockDragLayout, PageBlockDragObservation, PageBlockDragTarget,
    PAGE_BLOCK_DROP_LINE_HEIGHT, PAGE_BLOCK_GUTTER_WIDTH, PAGE_BLOCK_INDENT,
};

mod destination;
mod flow;
mod interpolation;

pub(super) use flow::{page_flow_surface, resolve_page_flow_drag_target};

use destination::{
    page_block_after_target, page_block_before_target, page_block_drop_wash,
    page_block_nested_target,
};
use interpolation::page_block_drag_geometry;

const PAGE_BLOCK_NESTED_INDICATOR_RIGHT_INSET: f32 = 6.0;
const PAGE_BLOCK_DROP_WASH_INSET: f32 = 2.0;
// A Callout body starts 45px from its row base (8 outer + 1 border + 12 padding + 24 icon).
// Canonical depth already contributes 30px, leaving 15px of extra left indentation per layer.
// Its right edge loses only the 8 + 1 + 12 frame inset.
const PAGE_CALLOUT_BODY_INDENT_EXTRA: f32 = 15.0;
const PAGE_CALLOUT_BODY_RIGHT_INSET: f32 = 21.0;

#[derive(Clone, Copy)]
struct PageBlockDragRowGeometry {
    visual_top: f32,
    visual_bottom: f32,
}

struct PageBlockDragGeometry {
    document_units: Vec<Option<PageBlockDragRowGeometry>>,
}

struct PageBlockDragDestination {
    parent_block_id: String,
    before_block_id: Option<String>,
    depth: usize,
    indicator_top: f32,
}

struct PageBlockDragHover {
    index: usize,
    requested_depth: usize,
    lower_half: bool,
}

#[derive(Clone, Copy)]
struct PageBlockDragViewport {
    page_left: f32,
    top: f32,
    bottom: f32,
}

pub(super) fn resolve_page_block_drag_target(
    data: &LoadedCardPageData,
    layout: &PageBlockDragLayout,
) -> Option<PageBlockDragTarget> {
    let observation = layout.observation.as_ref()?;
    if !valid_page_block_drag_observation(data, layout, observation) {
        return None;
    }
    let row_geometry = page_block_drag_geometry(data, layout)?;

    let hover = page_block_drag_hover(data, layout, observation, &row_geometry)?;
    let destination = page_block_drag_destination(
        data,
        &row_geometry,
        hover.index,
        hover.requested_depth,
        hover.lower_half,
    )?;
    if invalid_page_block_drag_destination(&observation.payload, &destination) {
        return None;
    }
    build_page_block_drag_target(data, layout, observation, &row_geometry, destination)
}

fn page_block_drag_hover(
    data: &LoadedCardPageData,
    layout: &PageBlockDragLayout,
    observation: &PageBlockDragObservation,
    geometry: &PageBlockDragGeometry,
) -> Option<PageBlockDragHover> {
    let rows = &data.visible_rows;
    let page_left = page_block_drag_page_left(data, layout)?;
    let pointer_x = observation.pointer.x.as_f32();
    if pointer_x < page_left - PAGE_BLOCK_GUTTER_WIDTH
        || pointer_x > page_left + observation.payload.preview_width.current()
    {
        return None;
    }
    let pointer_y = observation.pointer.y.as_f32().clamp(
        layout.row_bounds.first()?.top().as_f32(),
        layout.row_bounds.last()?.bottom().as_f32(),
    );
    let measured_row_index = page_block_drag_hover_index(pointer_y, &layout.row_bounds);
    let index = layout.owner_visible_row_indices[measured_row_index];
    let block = &data.page.blocks[rows[index].block_index];
    if observation.payload.subtree_contains(&block.block_id) {
        return None;
    }
    let owner_bounds = geometry.row(data, index)?;
    let requested_depth = if pointer_x < page_left {
        0
    } else {
        ((pointer_x - page_left) / PAGE_BLOCK_INDENT).floor() as usize + 1
    };
    Some(PageBlockDragHover {
        index,
        requested_depth,
        lower_half: pointer_y > (owner_bounds.visual_top + owner_bounds.visual_bottom) / 2.0,
    })
}

fn valid_page_block_drag_observation(
    data: &LoadedCardPageData,
    layout: &PageBlockDragLayout,
    observation: &PageBlockDragObservation,
) -> bool {
    !data.visible_rows.is_empty()
        && !layout.document_unit_indices.is_empty()
        && layout.row_bounds.len() == layout.document_unit_indices.len()
        && layout.owner_visible_row_indices.len() == layout.document_unit_indices.len()
        && layout.owner_block_ids.len() == layout.document_unit_indices.len()
        && layout
            .document_unit_indices
            .windows(2)
            .all(|indices| indices[0] < indices[1])
        && layout
            .document_unit_indices
            .iter()
            .all(|unit_index| *unit_index < data.document_units.len())
        && layout.row_bounds.windows(2).all(|bounds| {
            bounds[0].top() <= bounds[1].top() && bounds[0].bottom() <= bounds[1].bottom()
        })
        && layout
            .owner_visible_row_indices
            .iter()
            .zip(&layout.document_unit_indices)
            .all(|(visible_row_index, document_unit_index)| {
                data.document_units[*document_unit_index].owner_visible_row_index()
                    == *visible_row_index
            })
        && layout
            .owner_block_ids
            .iter()
            .zip(&layout.owner_visible_row_indices)
            .all(|(block_id, visible_row_index)| {
                data.page.blocks[data.visible_rows[*visible_row_index].block_index].block_id
                    == *block_id
            })
        && observation
            .payload
            .root_block_ids()
            .iter()
            .all(|root_id| data.page_block(root_id).is_some())
        && observation.pointer.x >= observation.container_bounds.left()
        && observation.pointer.x <= observation.container_bounds.right()
}

fn page_block_drag_destination(
    data: &LoadedCardPageData,
    geometry: &PageBlockDragGeometry,
    hover_index: usize,
    requested_depth: usize,
    lower_half: bool,
) -> Option<PageBlockDragDestination> {
    let hover_row = &data.visible_rows[hover_index];
    let hover = &data.page.blocks[hover_row.block_index];
    let target =
        if lower_half && requested_depth > hover_row.visual_depth && hover.can_accept_children() {
            page_block_nested_target(data, geometry, hover_index)?
        } else {
            let depth = requested_depth.min(hover_row.visual_depth);
            let ancestor = page_block_visible_ancestor_at_depth(data, hover_index, depth)?;
            if lower_half && page_block_visible_subtree_end_row(data, ancestor) == hover_index + 1 {
                page_block_after_target(data, geometry, ancestor)?
            } else {
                page_block_before_target(data, geometry, ancestor)?
            }
        };
    Some(target)
}

fn invalid_page_block_drag_destination(
    payload: &super::PageBlockDragPayload,
    destination: &PageBlockDragDestination,
) -> bool {
    payload.subtree_contains(&destination.parent_block_id)
        || destination
            .before_block_id
            .as_ref()
            .is_some_and(|block_id| payload.subtree_contains(block_id))
}

fn build_page_block_drag_target(
    data: &LoadedCardPageData,
    layout: &PageBlockDragLayout,
    observation: &PageBlockDragObservation,
    geometry: &PageBlockDragGeometry,
    destination: PageBlockDragDestination,
) -> Option<PageBlockDragTarget> {
    let indicator_right = if destination.depth == 0 {
        0.0
    } else {
        PAGE_BLOCK_NESTED_INDICATOR_RIGHT_INSET
    };
    let callout_layers = data.callout_layer_depth_for_block_id(&destination.parent_block_id);
    let depth_left = destination.depth as f32 * PAGE_BLOCK_INDENT
        + callout_layers as f32 * PAGE_CALLOUT_BODY_INDENT_EXTRA;
    let callout_right = callout_layers as f32 * PAGE_CALLOUT_BODY_RIGHT_INSET;
    let indicator_width = (observation.payload.preview_width.current()
        - depth_left
        - callout_right
        - indicator_right)
        .max(0.0);
    let page_left = page_block_drag_page_left(data, layout)?;
    let viewport_left = observation.container_bounds.left().as_f32();
    let viewport_top = observation.container_bounds.top().as_f32();
    let viewport_height = observation.container_bounds.size.height.as_f32();
    let viewport = PageBlockDragViewport {
        page_left: page_left - viewport_left,
        top: viewport_top,
        bottom: observation.container_bounds.bottom().as_f32(),
    };
    let wash = page_block_drop_wash(
        data,
        geometry,
        &destination,
        observation.payload.preview_width.current(),
        viewport,
    );
    Some(PageBlockDragTarget {
        scroll_target: observation.scroll_target,
        page_id: data.page.block_id.clone(),
        dragged_root_block_ids: observation.payload.root_block_ids().to_vec(),
        target_parent_block_id: destination.parent_block_id.clone(),
        before_block_id: destination.before_block_id,
        destination_depth: destination.depth,
        indicator_left: page_left - viewport_left + depth_left,
        indicator_top: (destination.indicator_top - viewport_top).clamp(
            0.0,
            (viewport_height - PAGE_BLOCK_DROP_LINE_HEIGHT).max(0.0),
        ),
        indicator_width,
        wash,
    })
}

fn page_block_drag_hover_index(pointer_y: f32, row_bounds: &[gpui::Bounds<gpui::Pixels>]) -> usize {
    row_bounds
        .windows(2)
        .position(|bounds| {
            let transition = (bounds[0].bottom().as_f32() + bounds[1].top().as_f32()) / 2.0;
            pointer_y <= transition
        })
        .unwrap_or(row_bounds.len() - 1)
}

fn page_block_drag_page_left(
    data: &LoadedCardPageData,
    layout: &PageBlockDragLayout,
) -> Option<f32> {
    let visible_row_index = *layout.owner_visible_row_indices.first()?;
    let rendered_depth = data.page_block_render_depth(visible_row_index);
    Some(layout.row_bounds.first()?.left().as_f32() - rendered_depth as f32 * PAGE_BLOCK_INDENT)
}

impl PageBlockDragGeometry {
    fn row(
        &self,
        data: &LoadedCardPageData,
        visible_row_index: usize,
    ) -> Option<PageBlockDragRowGeometry> {
        let range = data.document_unit_ranges[visible_row_index].clone();
        self.document_unit_span(range)
            .or_else(|| self.document_unit_span(data.callout_render_unit_range(visible_row_index)))
    }

    fn subtree_span(
        &self,
        data: &LoadedCardPageData,
        visible_row_index: usize,
    ) -> Option<PageBlockDragRowGeometry> {
        let visible_subtree_end = page_block_visible_subtree_end_row(data, visible_row_index);
        let start = data.document_unit_ranges.get(visible_row_index)?.start;
        let end = data
            .document_unit_ranges
            .get(visible_subtree_end.checked_sub(1)?)?
            .end;
        self.document_unit_span(start..end)
    }

    fn document_unit_span(
        &self,
        range: std::ops::Range<usize>,
    ) -> Option<PageBlockDragRowGeometry> {
        if range.is_empty() {
            return None;
        }
        let first_unit = self.document_units.get(range.start)?.as_ref()?;
        let last_unit = self
            .document_units
            .get(range.end.checked_sub(1)?)?
            .as_ref()?;
        Some(PageBlockDragRowGeometry {
            visual_top: first_unit.visual_top,
            visual_bottom: last_unit.visual_bottom,
        })
    }
}

fn page_block_visible_ancestor_at_depth(
    data: &LoadedCardPageData,
    mut row_index: usize,
    depth: usize,
) -> Option<usize> {
    while data.visible_rows[row_index].visual_depth > depth {
        row_index = data.visible_rows[row_index].visible_parent_row_index?;
    }
    (data.visible_rows[row_index].visual_depth == depth).then_some(row_index)
}

fn page_block_visible_subtree_end_row(data: &LoadedCardPageData, row_index: usize) -> usize {
    let block_index = data.visible_rows[row_index].block_index;
    let canonical_end = page_block_subtree_end(&data.page.blocks, block_index);
    data.visible_rows
        .partition_point(|row| row.block_index < canonical_end)
}
