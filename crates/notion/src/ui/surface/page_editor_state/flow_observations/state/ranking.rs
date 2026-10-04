//! Ranking used to choose the observation nearest the viewport top.

use gpui::{Bounds, Pixels};

use super::super::{PageFlowNodeObservation, PageFlowUnitObservation};

pub(super) fn best_unit<'a>(
    units: impl Iterator<Item = &'a PageFlowUnitObservation>,
    viewport_top: f32,
) -> Option<&'a PageFlowUnitObservation> {
    units.min_by_key(|unit| {
        observation_rank(
            unit.bounds,
            viewport_top,
            unit.outer_index,
            unit.document_unit_index,
        )
    })
}

pub(super) fn best_node<'a>(
    nodes: impl Iterator<Item = &'a PageFlowNodeObservation>,
    viewport_top: f32,
) -> Option<&'a PageFlowNodeObservation> {
    nodes.min_by_key(|node| observation_rank(node.bounds, viewport_top, node.outer_index, 0))
}

fn observation_rank(
    bounds: Bounds<Pixels>,
    viewport_top: f32,
    outer_index: usize,
    stable_index: usize,
) -> (u8, u32, usize, u32, usize) {
    let top = bounds.top().as_f32();
    let bottom = bounds.bottom().as_f32();
    let (proximity, distance) = if top <= viewport_top && bottom >= viewport_top {
        (0, viewport_top - top)
    } else if top > viewport_top {
        (1, top - viewport_top)
    } else {
        (2, viewport_top - bottom)
    };
    (
        proximity,
        distance.to_bits(),
        outer_index,
        bounds.left().as_f32().to_bits(),
        stable_index,
    )
}
