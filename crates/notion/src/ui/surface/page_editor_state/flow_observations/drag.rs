use std::{collections::HashMap, sync::Arc};

use gpui::{Bounds, Pixels};

use crate::ui::{LoadedCardPageData, PageFlowLaneKey, PageFlowNodeId, PageFlowSequenceId};

use super::{
    union_bounds, PageFlowLaneObservation, PageFlowNodeObservation, PageFlowObservationToken,
};

#[derive(Clone)]
pub(crate) struct PageFlowLayoutAuthority {
    token: PageFlowObservationToken,
}

pub(crate) struct PageFlowDragAuthority<'a> {
    token: &'a PageFlowObservationToken,
    lanes: &'a [PageFlowLaneObservation],
    geometry: &'a PageFlowDragGeometry,
}

pub(crate) struct PageFlowDropAnchor {
    pub(crate) block_id: Arc<str>,
    pub(crate) bounds: Bounds<Pixels>,
}

pub(super) struct PageFlowDragGeometry {
    anchors_by_lane: HashMap<PageFlowSequenceId, Arc<[PageFlowDropAnchor]>>,
}

impl<'a> PageFlowDragAuthority<'a> {
    pub(super) fn new(
        token: &'a PageFlowObservationToken,
        lanes: &'a [PageFlowLaneObservation],
        geometry: &'a PageFlowDragGeometry,
    ) -> Self {
        Self {
            token,
            lanes,
            geometry,
        }
    }

    pub(crate) fn page_id(&self) -> &str {
        self.token.page_id()
    }

    pub(crate) fn lanes(&self) -> &[PageFlowLaneObservation] {
        self.lanes
    }

    pub(crate) fn anchors_for(&self, lane: &PageFlowLaneKey) -> &[PageFlowDropAnchor] {
        self.geometry
            .anchors_by_lane
            .get(&lane.sequence_id())
            .map_or(&[], Arc::as_ref)
    }

    pub(crate) fn layout_authority(&self) -> PageFlowLayoutAuthority {
        PageFlowLayoutAuthority {
            token: self.token.clone(),
        }
    }
}

impl PageFlowLayoutAuthority {
    pub(super) const fn surface(&self) -> super::PageFlowSurfaceKey {
        self.token.surface()
    }

    pub(super) fn matches_token(&self, token: &PageFlowObservationToken) -> bool {
        self.token.same_layout_allocation(token)
    }
}

/// Each observed lane's drop anchors, keyed by the block they belong to.
type LaneAnchorBuckets = HashMap<PageFlowSequenceId, HashMap<Arc<str>, Bounds<Pixels>>>;

pub(super) fn prepare_page_flow_drag_geometry<'a>(
    data: &LoadedCardPageData,
    lanes: &[PageFlowLaneObservation],
    nodes: impl Iterator<Item = &'a PageFlowNodeObservation>,
) -> PageFlowDragGeometry {
    let mut anchors = lanes
        .iter()
        .map(|lane| (lane.lane_id.clone(), HashMap::new()))
        .collect::<LaneAnchorBuckets>();
    for lane in lanes {
        prepare_lane_unit_anchors(data, lane, &mut anchors);
    }
    for node in nodes {
        prepare_lane_node_anchor(data, node, &mut anchors);
    }
    PageFlowDragGeometry {
        anchors_by_lane: anchors
            .into_iter()
            .map(|(lane, anchors)| (lane, sorted_anchors(anchors)))
            .collect(),
    }
}

fn prepare_lane_unit_anchors(
    data: &LoadedCardPageData,
    lane: &PageFlowLaneObservation,
    anchors: &mut LaneAnchorBuckets,
) {
    let parent_id = lane_parent_id(data, &lane.lane_path);
    let lane_anchors = anchors
        .get_mut(&lane.lane_id)
        .expect("every observed lane must retain a drag-geometry bucket");
    for row in &lane.rows {
        let owner = data.document_units[row.document_unit_index].owner_visible_row_index();
        let block = &data.page.blocks[data.visible_rows[owner].block_index];
        let block_id = observed_direct_child_id(data, block, parent_id);
        merge_anchor(lane_anchors, block_id, row.bounds);
    }
}

fn prepare_lane_node_anchor(
    data: &LoadedCardPageData,
    node: &PageFlowNodeObservation,
    anchors: &mut LaneAnchorBuckets,
) {
    let PageFlowNodeId::Columns {
        column_list_block_id,
    } = &node.node_id
    else {
        return;
    };
    let lane = anchors
        .get_mut(&node.lane_id)
        .expect("every observed node must belong to an observed flow lane");
    let parent_id = match &node.lane_id {
        PageFlowSequenceId::Root => data.page.block_id.as_str(),
        PageFlowSequenceId::Column { column_block_id } => column_block_id,
    };
    let block = data
        .page_block(column_list_block_id)
        .expect("an observed Columns node must retain its ColumnList block");
    let block_id = observed_direct_child_id(data, block, parent_id);
    merge_anchor(lane, block_id, node.bounds);
}

fn lane_parent_id<'a>(data: &'a LoadedCardPageData, lane: &'a PageFlowLaneKey) -> &'a str {
    lane.column_ids()
        .map_or(data.page.block_id.as_str(), |(_, column_id)| column_id)
}

fn observed_direct_child_id<'a>(
    data: &'a LoadedCardPageData,
    mut block: &'a crate::ui::CardPageBlock,
    parent_id: &str,
) -> &'a str {
    loop {
        if block.parent_block_id == parent_id {
            return &block.block_id;
        }
        block = data.page_block(&block.parent_block_id).unwrap_or_else(|| {
            panic!(
                "observed block {} must remain below flow lane parent {parent_id}",
                block.block_id
            )
        });
    }
}

fn merge_anchor(
    anchors: &mut HashMap<Arc<str>, Bounds<Pixels>>,
    block_id: &str,
    bounds: Bounds<Pixels>,
) {
    anchors
        .entry(Arc::from(block_id))
        .and_modify(|current| *current = union_bounds(*current, bounds))
        .or_insert(bounds);
}

fn sorted_anchors(anchors: HashMap<Arc<str>, Bounds<Pixels>>) -> Arc<[PageFlowDropAnchor]> {
    let mut anchors = anchors
        .into_iter()
        .map(|(block_id, bounds)| PageFlowDropAnchor { block_id, bounds })
        .collect::<Vec<_>>();
    anchors.sort_by(|left, right| {
        left.bounds
            .top()
            .as_f32()
            .total_cmp(&right.bounds.top().as_f32())
            .then_with(|| {
                left.bounds
                    .left()
                    .as_f32()
                    .total_cmp(&right.bounds.left().as_f32())
            })
            .then_with(|| left.block_id.cmp(&right.block_id))
    });
    anchors.into()
}
