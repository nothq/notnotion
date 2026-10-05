use crate::ui::{LoadedCardPageData, PageDocumentOuterItemId, PageDocumentUnitKey, PageFlowNodeId};

use super::state::PageFlowSurfaceState;
use super::{metrics, PageFlowOuterLocalY, PageFlowPinTarget, PageFlowSemanticAnchorProjection};

pub(super) fn estimate(
    state: &PageFlowSurfaceState,
    data: &LoadedCardPageData,
    target: &PageFlowPinTarget,
) -> Option<PageFlowSemanticAnchorProjection> {
    match target {
        PageFlowPinTarget::Unit(key) => estimate_unit(state, data, target, key),
        PageFlowPinTarget::Node(node_id) => estimate_node(state, data, target, node_id),
        PageFlowPinTarget::Outer(outer_item) => Some(PageFlowSemanticAnchorProjection {
            target: target.clone(),
            outer_item: outer_item.clone(),
            outer_index: outer_item_index(data, outer_item)?,
            outer_local_y: PageFlowOuterLocalY::new(0.0),
        }),
    }
}

fn estimate_unit(
    state: &PageFlowSurfaceState,
    data: &LoadedCardPageData,
    target: &PageFlowPinTarget,
    key: &PageDocumentUnitKey,
) -> Option<PageFlowSemanticAnchorProjection> {
    let location = data.flow.location(key)?;
    let node_id = location.node_path.key().node_id();
    let section_offset = estimated_section_unit_offset(state, data, &node_id, key)?;
    let node_offset = state
        .metrics
        .node_outer_local_y(&state.projection, &node_id)?;
    Some(PageFlowSemanticAnchorProjection {
        target: target.clone(),
        outer_item: PageDocumentOuterItemId::Flow(location.outer_item.key.node_id()),
        outer_index: location.outer_item.index + 1,
        outer_local_y: PageFlowOuterLocalY::new(node_offset + section_offset),
    })
}

fn estimate_node(
    state: &PageFlowSurfaceState,
    data: &LoadedCardPageData,
    target: &PageFlowPinTarget,
    node_id: &PageFlowNodeId,
) -> Option<PageFlowSemanticAnchorProjection> {
    let outer = data.flow.outer_item_for_node(node_id)?;
    Some(PageFlowSemanticAnchorProjection {
        target: target.clone(),
        outer_item: PageDocumentOuterItemId::Flow(outer.key.node_id()),
        outer_index: outer.index + 1,
        outer_local_y: PageFlowOuterLocalY::new(
            state
                .metrics
                .node_outer_local_y(&state.projection, node_id)?,
        ),
    })
}

fn estimated_section_unit_offset(
    state: &PageFlowSurfaceState,
    data: &LoadedCardPageData,
    node_id: &PageFlowNodeId,
    unit_key: &PageDocumentUnitKey,
) -> Option<f32> {
    let location = data.flow.location(unit_key)?;
    let PageFlowNodeId::Section { first_unit } = node_id else {
        return None;
    };
    let first_index = data.flow.location(first_unit)?.document_unit_index;
    let metric_location = state.projection.node_locations.get(node_id)?;
    let metric = state
        .projection
        .sequences
        .get(&metric_location.sequence_id)?
        .nodes
        .get(metric_location.index)?;
    Some(
        metric.content_block_offset
            + (first_index..location.document_unit_index)
                .map(|index| {
                    metrics::estimate_document_unit(data, index, state.projection.estimate)
                })
                .sum::<f32>(),
    )
}

fn outer_item_index(
    data: &LoadedCardPageData,
    outer_item: &PageDocumentOuterItemId,
) -> Option<usize> {
    match outer_item {
        PageDocumentOuterItemId::Lead => Some(0),
        PageDocumentOuterItemId::Flow(node_id) => {
            Some(data.flow.outer_item_for_node(node_id)?.index + 1)
        }
        PageDocumentOuterItemId::Footer => Some(data.flow.root.nodes.len() + 1),
    }
}
