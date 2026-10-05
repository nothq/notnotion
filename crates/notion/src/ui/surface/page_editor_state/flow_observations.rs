use std::{
    cmp::Ordering,
    sync::{Arc, Weak},
};

use gpui::{point, Bounds, Pixels, Point};

use crate::ui::{
    LoadedCardPageData, PageDocumentListAllocation, PageDocumentOuterItemId, PageDocumentUnitKey,
    PageFlowLaneKey, PageFlowNodeId, PageFlowSequenceId,
};

use super::{PageFlowLayoutFrameToken, PageFlowRenderGeneration, PageFlowSurfaceKey};

mod drag;
mod state;

pub(crate) use drag::{PageFlowDragAuthority, PageFlowDropAnchor, PageFlowLayoutAuthority};
pub(crate) use state::{
    PageFlowAnchorTarget, PageFlowCapturedSemanticAnchor, PageFlowObservations,
};

#[derive(Clone)]
pub(crate) struct PageFlowObservationToken {
    surface: PageFlowSurfaceKey,
    page_id: Arc<str>,
    allocation: Weak<LoadedCardPageData>,
    projection_generation: u64,
    page_block_render_generation: u64,
    list_allocation: PageDocumentListAllocation,
    frame: Option<PageFlowLayoutFrameToken>,
}

#[derive(Clone)]
pub(crate) struct PageFlowMountAuthority {
    surface: PageFlowSurfaceKey,
    page_id: Arc<str>,
    allocation: Weak<LoadedCardPageData>,
    projection_generation: u64,
    render_generation: PageFlowRenderGeneration,
    list_allocation: PageDocumentListAllocation,
}

#[derive(Clone)]
pub(crate) struct PageFlowUnitObservation {
    pub(crate) unit_key: PageDocumentUnitKey,
    pub(crate) document_unit_index: usize,
    pub(crate) outer_item: PageDocumentOuterItemId,
    pub(crate) outer_index: usize,
    pub(crate) bounds: Bounds<Pixels>,
}

#[derive(Clone)]
pub(crate) struct PageFlowNodeObservation {
    pub(crate) node_id: PageFlowNodeId,
    lane_id: PageFlowSequenceId,
    pub(crate) outer_item: PageDocumentOuterItemId,
    pub(crate) outer_index: usize,
    pub(crate) bounds: Bounds<Pixels>,
}

#[derive(Clone)]
pub(crate) struct PageFlowLaneObservation {
    pub(crate) lane_path: PageFlowLaneKey,
    pub(crate) lane_bounds: Bounds<Pixels>,
    pub(crate) rows: Vec<PageFlowUnitObservation>,
    lane_id: PageFlowSequenceId,
    lane_depth: usize,
}

impl PageFlowLaneObservation {
    pub(crate) fn linear_section(
        data: &LoadedCardPageData,
        document_unit_indices: &[usize],
        row_bounds: Vec<Bounds<Pixels>>,
    ) -> Self {
        assert_eq!(document_unit_indices.len(), row_bounds.len());
        let rows = document_unit_indices
            .iter()
            .copied()
            .zip(row_bounds)
            .map(|(index, bounds)| PageFlowUnitObservation::new(data, index, bounds))
            .collect::<Vec<_>>();
        let lane_bounds = enclosing_row_bounds(&rows)
            .expect("a rendered page section must observe at least one document unit");
        Self::new(data.flow.root.lane.clone(), lane_bounds, rows)
    }

    pub(crate) fn new(
        lane_path: PageFlowLaneKey,
        lane_bounds: Bounds<Pixels>,
        rows: Vec<PageFlowUnitObservation>,
    ) -> Self {
        let lane_id = lane_path.sequence_id();
        let lane_depth = lane_path_depth(&lane_path);
        Self {
            lane_path,
            lane_bounds,
            rows,
            lane_id,
            lane_depth,
        }
    }

    pub(crate) fn rank(&self, position: Point<Pixels>) -> PageFlowLaneRank<'_> {
        PageFlowLaneRank::new(self, position)
    }
}

impl PageFlowUnitObservation {
    pub(crate) fn new(data: &LoadedCardPageData, index: usize, bounds: Bounds<Pixels>) -> Self {
        let unit = &data.document_units[index];
        let unit_key = unit.key().clone();
        let location = data
            .flow
            .location(&unit_key)
            .expect("every observed document unit must have a flow location");
        assert_eq!(location.document_unit_index, index);
        Self {
            unit_key,
            document_unit_index: index,
            outer_item: PageDocumentOuterItemId::Flow(location.outer_item.key.node_id()),
            outer_index: location.outer_item.index + 1,
            bounds,
        }
    }
}

impl PageFlowNodeObservation {
    pub(crate) fn new(
        data: &LoadedCardPageData,
        node_id: PageFlowNodeId,
        lane: &PageFlowLaneKey,
        bounds: Bounds<Pixels>,
    ) -> Self {
        let outer = data
            .flow
            .outer_item_for_node(&node_id)
            .expect("every observed flow node must retain its root outer item");
        Self {
            node_id,
            lane_id: lane.sequence_id(),
            outer_item: PageDocumentOuterItemId::Flow(outer.key.node_id()),
            outer_index: outer.index + 1,
            bounds,
        }
    }
}

impl PageFlowObservationToken {
    pub(crate) fn existing_linear(
        data: &Arc<LoadedCardPageData>,
        list_allocation: &PageDocumentListAllocation,
        surface: PageFlowSurfaceKey,
        page_block_render_generation: u64,
    ) -> Self {
        Self::new(
            data,
            list_allocation,
            surface,
            page_block_render_generation,
            None,
        )
    }

    pub(crate) fn recursive(
        data: &Arc<LoadedCardPageData>,
        list_allocation: &PageDocumentListAllocation,
        surface: PageFlowSurfaceKey,
        page_block_render_generation: u64,
        frame: PageFlowLayoutFrameToken,
    ) -> Self {
        assert_eq!(frame.page_id(), data.page.block_id);
        assert_eq!(frame.surface(), surface);
        Self::new(
            data,
            list_allocation,
            surface,
            page_block_render_generation,
            Some(frame),
        )
    }

    fn new(
        data: &Arc<LoadedCardPageData>,
        list_allocation: &PageDocumentListAllocation,
        surface: PageFlowSurfaceKey,
        page_block_render_generation: u64,
        frame: Option<PageFlowLayoutFrameToken>,
    ) -> Self {
        Self {
            surface,
            page_id: Arc::from(data.page.block_id.as_str()),
            allocation: Arc::downgrade(data),
            projection_generation: data.flow_projection_generation,
            page_block_render_generation,
            list_allocation: list_allocation.clone(),
            frame,
        }
    }

    pub(crate) const fn surface(&self) -> PageFlowSurfaceKey {
        self.surface
    }

    pub(crate) fn page_id(&self) -> &str {
        &self.page_id
    }

    fn matches_list(&self, list_allocation: &PageDocumentListAllocation) -> bool {
        self.list_allocation.same_allocation(list_allocation)
    }

    fn matches(&self, other: &Self) -> bool {
        self.surface == other.surface
            && self.page_id == other.page_id
            && Weak::ptr_eq(&self.allocation, &other.allocation)
            && self.projection_generation == other.projection_generation
            && self.page_block_render_generation == other.page_block_render_generation
            && self.list_allocation.same_allocation(&other.list_allocation)
            && self.frame == other.frame
    }

    fn same_layout_allocation(&self, other: &Self) -> bool {
        self.surface == other.surface
            && self.page_id == other.page_id
            && Weak::ptr_eq(&self.allocation, &other.allocation)
            && self.projection_generation == other.projection_generation
            && self.list_allocation.same_allocation(&other.list_allocation)
    }

    fn data(&self) -> Option<Arc<LoadedCardPageData>> {
        self.allocation.upgrade()
    }

    fn render_generation(&self) -> Option<PageFlowRenderGeneration> {
        self.frame
            .as_ref()
            .map(PageFlowLayoutFrameToken::render_generation)
    }

    pub(crate) fn mount_authority(&self) -> Option<PageFlowMountAuthority> {
        Some(PageFlowMountAuthority {
            surface: self.surface,
            page_id: self.page_id.clone(),
            allocation: self.allocation.clone(),
            projection_generation: self.projection_generation,
            render_generation: self.render_generation()?,
            list_allocation: self.list_allocation.clone(),
        })
    }
}

impl PageFlowMountAuthority {
    pub(crate) fn matches(&self, token: &PageFlowObservationToken) -> bool {
        self.surface == token.surface
            && self.page_id == token.page_id
            && Weak::ptr_eq(&self.allocation, &token.allocation)
            && self.projection_generation == token.projection_generation
            && Some(self.render_generation) == token.render_generation()
            && self.list_allocation.same_allocation(&token.list_allocation)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PageFlowLaneRank<'a> {
    proximity: u8,
    primary_distance: f32,
    secondary_distance: f32,
    containing_depth: usize,
    bounds: Bounds<Pixels>,
    lane_id: &'a PageFlowSequenceId,
}

impl<'a> PageFlowLaneRank<'a> {
    fn new(lane: &'a PageFlowLaneObservation, position: Point<Pixels>) -> Self {
        let contains_x = horizontal_distance(lane.lane_bounds, position) == 0.0;
        let contains_point = contains_x && vertical_distance(lane.lane_bounds, position) == 0.0;
        let (proximity, primary_distance, secondary_distance, containing_depth) = if contains_point
        {
            (0, 0.0, 0.0, lane.lane_depth)
        } else if contains_x {
            (1, vertical_distance(lane.lane_bounds, position), 0.0, 0)
        } else {
            (
                2,
                horizontal_distance(lane.lane_bounds, position),
                vertical_distance(lane.lane_bounds, position),
                0,
            )
        };
        Self {
            proximity,
            primary_distance,
            secondary_distance,
            containing_depth,
            bounds: lane.lane_bounds,
            lane_id: &lane.lane_id,
        }
    }

    pub(crate) fn compare(self, other: Self) -> Ordering {
        self.proximity
            .cmp(&other.proximity)
            .then_with(|| other.containing_depth.cmp(&self.containing_depth))
            .then_with(|| self.primary_distance.total_cmp(&other.primary_distance))
            .then_with(|| self.secondary_distance.total_cmp(&other.secondary_distance))
            .then_with(|| compare_bounds(self.bounds, other.bounds))
            .then_with(|| compare_lane_ids(self.lane_id, other.lane_id))
    }
}

pub(super) fn union_bounds(left: Bounds<Pixels>, right: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::from_corners(
        point(left.left().min(right.left()), left.top().min(right.top())),
        point(
            left.right().max(right.right()),
            left.bottom().max(right.bottom()),
        ),
    )
}

fn enclosing_row_bounds(rows: &[PageFlowUnitObservation]) -> Option<Bounds<Pixels>> {
    let first = rows.first()?.bounds;
    let (left, top, right, bottom) = rows.iter().skip(1).fold(
        (first.left(), first.top(), first.right(), first.bottom()),
        |(left, top, right, bottom), row| {
            (
                left.min(row.bounds.left()),
                top.min(row.bounds.top()),
                right.max(row.bounds.right()),
                bottom.max(row.bounds.bottom()),
            )
        },
    );
    Some(Bounds::from_corners(point(left, top), point(right, bottom)))
}

fn lane_path_depth(lane: &PageFlowLaneKey) -> usize {
    let mut depth = 0;
    let mut current = lane.parent();
    while let Some(parent) = current {
        depth += 1;
        current = parent.parent();
    }
    depth
}

fn horizontal_distance(bounds: Bounds<Pixels>, position: Point<Pixels>) -> f32 {
    if position.x < bounds.left() {
        (bounds.left() - position.x).as_f32()
    } else if position.x > bounds.right() {
        (position.x - bounds.right()).as_f32()
    } else {
        0.0
    }
}

fn vertical_distance(bounds: Bounds<Pixels>, position: Point<Pixels>) -> f32 {
    if position.y < bounds.top() {
        (bounds.top() - position.y).as_f32()
    } else if position.y > bounds.bottom() {
        (position.y - bounds.bottom()).as_f32()
    } else {
        0.0
    }
}

fn compare_bounds(left: Bounds<Pixels>, right: Bounds<Pixels>) -> Ordering {
    left.top()
        .as_f32()
        .total_cmp(&right.top().as_f32())
        .then_with(|| left.left().as_f32().total_cmp(&right.left().as_f32()))
        .then_with(|| left.bottom().as_f32().total_cmp(&right.bottom().as_f32()))
        .then_with(|| left.right().as_f32().total_cmp(&right.right().as_f32()))
}

fn compare_lane_ids(left: &PageFlowSequenceId, right: &PageFlowSequenceId) -> Ordering {
    match (left, right) {
        (PageFlowSequenceId::Root, PageFlowSequenceId::Root) => Ordering::Equal,
        (PageFlowSequenceId::Root, PageFlowSequenceId::Column { .. }) => Ordering::Less,
        (PageFlowSequenceId::Column { .. }, PageFlowSequenceId::Root) => Ordering::Greater,
        (
            PageFlowSequenceId::Column {
                column_block_id: left,
            },
            PageFlowSequenceId::Column {
                column_block_id: right,
            },
        ) => left.cmp(right),
    }
}
