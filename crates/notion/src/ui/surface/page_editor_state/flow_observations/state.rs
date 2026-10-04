use std::collections::HashMap;

mod ranking;

use ranking::{best_node, best_unit};

use gpui::{Bounds, ListOffset, Pixels};

use crate::ui::{
    LoadedCardPageData, PageDocumentListAllocation, PageDocumentOuterItemId, PageDocumentUnitKey,
    PageFlowNodeId, PageFlowSequenceId,
};

use super::super::{
    PageFlowOuterLocalY, PageFlowPinTarget, PageFlowSemanticAnchorBaseline,
    PageFlowSemanticAnchorIntent, PageFlowSemanticAnchorProjection, PageFlowSurfaceKey,
    PageFlowViewportRelativeY,
};
use super::{
    drag::{prepare_page_flow_drag_geometry, PageFlowDragGeometry},
    union_bounds, PageFlowDragAuthority, PageFlowLaneObservation, PageFlowLayoutAuthority,
    PageFlowNodeObservation, PageFlowObservationToken, PageFlowUnitObservation,
};

/// The mounted flow target to anchor to, and why.
pub(crate) struct PageFlowAnchorTarget<'a> {
    pub(crate) target: &'a PageFlowPinTarget,
    pub(crate) intent: PageFlowSemanticAnchorIntent,
}

#[derive(Clone)]
pub(crate) struct PageFlowCapturedSemanticAnchor {
    pub(crate) baseline: PageFlowSemanticAnchorBaseline,
    source: PageFlowObservationToken,
}

impl PageFlowCapturedSemanticAnchor {
    pub(crate) fn source_matches_list(&self, list_allocation: &PageDocumentListAllocation) -> bool {
        self.source.matches_list(list_allocation)
    }
}

#[derive(Default)]
pub(crate) struct PageFlowObservations {
    surfaces: HashMap<PageFlowSurfaceKey, PageFlowSurfaceObservations>,
}

struct PageFlowSurfaceObservations {
    token: PageFlowObservationToken,
    logical_scroll_top: ListOffset,
    lanes: Vec<PageFlowLaneObservation>,
    unit_lanes: HashMap<PageDocumentUnitKey, PageFlowSequenceId>,
    units: HashMap<PageDocumentUnitKey, PageFlowUnitObservation>,
    nodes: HashMap<PageFlowNodeId, PageFlowNodeObservation>,
    drag_geometry: Option<PageFlowDragGeometry>,
}

impl PageFlowObservations {
    pub(crate) fn begin_surface_generation(
        &mut self,
        token: PageFlowObservationToken,
        logical_scroll_top: ListOffset,
    ) {
        self.surfaces.insert(
            token.surface(),
            PageFlowSurfaceObservations::new(token, logical_scroll_top),
        );
    }

    /// The basis is stamped at preparation time, before the list lays out. The
    /// list can still adjust its own scroll offset during that layout, so the
    /// prepainted root re-stamps it against the offset the observed bounds were
    /// actually measured at.
    pub(crate) fn observe_scroll_basis(
        &mut self,
        token: &PageFlowObservationToken,
        logical_scroll_top: ListOffset,
    ) -> bool {
        let Some(state) = self.current_mut(token) else {
            return false;
        };
        state.logical_scroll_top = logical_scroll_top;
        true
    }

    pub(crate) fn observe_lane(
        &mut self,
        token: &PageFlowObservationToken,
        observation: PageFlowLaneObservation,
    ) -> bool {
        let Some(state) = self.current_mut(token) else {
            return false;
        };
        state.merge_lane(observation);
        state.drag_geometry = None;
        true
    }

    pub(crate) fn observe_node(
        &mut self,
        token: &PageFlowObservationToken,
        observation: PageFlowNodeObservation,
    ) -> bool {
        let Some(state) = self.current_mut(token) else {
            return false;
        };
        state.nodes.insert(observation.node_id.clone(), observation);
        state.drag_geometry = None;
        true
    }

    pub(crate) fn lanes(
        &self,
        surface: PageFlowSurfaceKey,
        data: &LoadedCardPageData,
    ) -> Option<&[PageFlowLaneObservation]> {
        let state = self.surfaces.get(&surface)?;
        state.matches_data(data).then_some(state.lanes.as_slice())
    }

    pub(crate) fn drag_authority(
        &self,
        surface: PageFlowSurfaceKey,
        data: &std::sync::Arc<LoadedCardPageData>,
    ) -> Option<PageFlowDragAuthority<'_>> {
        let state = self.surfaces.get(&surface)?;
        if !state.matches_data(data) {
            return None;
        }
        Some(PageFlowDragAuthority::new(
            &state.token,
            &state.lanes,
            state.drag_geometry.as_ref()?,
        ))
    }

    pub(crate) fn layout_authority_is_current(
        &self,
        authority: &PageFlowLayoutAuthority,
        data: &std::sync::Arc<LoadedCardPageData>,
    ) -> bool {
        self.surfaces
            .get(&authority.surface())
            .is_some_and(|state| state.matches_data(data) && authority.matches_token(&state.token))
    }

    pub(crate) fn prepare_drag_geometry(
        &mut self,
        token: &PageFlowObservationToken,
        data: &LoadedCardPageData,
    ) -> bool {
        let Some(state) = self.current_mut(token) else {
            return false;
        };
        if state.drag_geometry.is_none() {
            state.drag_geometry = Some(prepare_page_flow_drag_geometry(
                data,
                &state.lanes,
                state.nodes.values(),
            ));
        }
        true
    }

    pub(crate) fn capture_true_top(
        &self,
        surface: PageFlowSurfaceKey,
        list_allocation: &PageDocumentListAllocation,
        viewport: Bounds<Pixels>,
        logical_scroll_top: ListOffset,
    ) -> Option<PageFlowCapturedSemanticAnchor> {
        let state = self.surfaces.get(&surface)?;
        if !state.token.matches_list(list_allocation) {
            return None;
        }
        if state.logical_scroll_top.item_ix != logical_scroll_top.item_ix {
            return None;
        }
        let top = viewport.top().as_f32();
        let observed_top = top + logical_scroll_top.offset_in_item.as_f32()
            - state.logical_scroll_top.offset_in_item.as_f32();
        let target = best_unit(state.units.values(), observed_top)
            .map(|unit| (PageFlowPinTarget::Unit(unit.unit_key.clone()), unit.bounds))
            .or_else(|| {
                best_node(state.nodes.values(), observed_top)
                    .map(|node| (PageFlowPinTarget::Node(node.node_id.clone()), node.bounds))
            })?;
        Some(PageFlowCapturedSemanticAnchor {
            baseline: PageFlowSemanticAnchorBaseline {
                target: target.0,
                viewport_relative_y: PageFlowViewportRelativeY::new(
                    target.1.top().as_f32() - observed_top,
                ),
                intent: PageFlowSemanticAnchorIntent::ViewportPreservation,
            },
            source: state.token.clone(),
        })
    }

    pub(crate) fn capture_target(
        &self,
        surface: PageFlowSurfaceKey,
        list_allocation: &PageDocumentListAllocation,
        anchor: PageFlowAnchorTarget<'_>,
        viewport: Bounds<Pixels>,
    ) -> Option<PageFlowCapturedSemanticAnchor> {
        let PageFlowAnchorTarget { target, intent } = anchor;
        let state = self.surfaces.get(&surface)?;
        if !state.token.matches_list(list_allocation) {
            return None;
        }
        let bounds = state.target_bounds(target)?;
        Some(PageFlowCapturedSemanticAnchor {
            baseline: PageFlowSemanticAnchorBaseline {
                target: target.clone(),
                viewport_relative_y: PageFlowViewportRelativeY::new(
                    bounds.top().as_f32() - viewport.top().as_f32(),
                ),
                intent,
            },
            source: state.token.clone(),
        })
    }

    pub(crate) fn mounted_projection(
        &self,
        token: &PageFlowObservationToken,
        target: &PageFlowPinTarget,
    ) -> Option<PageFlowSemanticAnchorProjection> {
        let state = self.current(token)?;
        state.mounted_projection(target)
    }

    pub(crate) fn outer_extent(
        &self,
        token: &PageFlowObservationToken,
        outer_item: &PageDocumentOuterItemId,
    ) -> Option<f32> {
        let state = self.current(token)?;
        let PageDocumentOuterItemId::Flow(node_id) = outer_item else {
            return None;
        };
        let node = state.nodes.get(node_id)?;
        (&node.outer_item == outer_item).then_some(node.bounds.size.height.as_f32())
    }

    pub(crate) fn is_current(&self, token: &PageFlowObservationToken) -> bool {
        self.current(token).is_some()
    }

    fn current(&self, token: &PageFlowObservationToken) -> Option<&PageFlowSurfaceObservations> {
        self.surfaces
            .get(&token.surface())
            .filter(|state| state.token.matches(token))
    }

    fn current_mut(
        &mut self,
        token: &PageFlowObservationToken,
    ) -> Option<&mut PageFlowSurfaceObservations> {
        self.surfaces
            .get_mut(&token.surface())
            .filter(|state| state.token.matches(token))
    }
}

impl PageFlowSurfaceObservations {
    fn new(token: PageFlowObservationToken, logical_scroll_top: ListOffset) -> Self {
        Self {
            token,
            logical_scroll_top,
            lanes: Vec::new(),
            unit_lanes: HashMap::new(),
            units: HashMap::new(),
            nodes: HashMap::new(),
            drag_geometry: None,
        }
    }

    fn matches_data(&self, data: &LoadedCardPageData) -> bool {
        self.token.data().as_deref().is_some_and(|current| {
            std::ptr::eq(current, data)
                && current.flow_projection_generation == data.flow_projection_generation
        })
    }

    fn merge_lane(&mut self, observation: PageFlowLaneObservation) {
        self.register_units(&observation);
        let lane_id = observation.lane_id.clone();
        let Some(lane) = self.lanes.iter_mut().find(|lane| lane.lane_id == lane_id) else {
            self.lanes.push(observation);
            return;
        };
        lane.lane_bounds = union_bounds(lane.lane_bounds, observation.lane_bounds);
        for row in observation.rows {
            match lane
                .rows
                .iter_mut()
                .find(|existing| existing.unit_key == row.unit_key)
            {
                Some(existing) => *existing = row,
                None => lane.rows.push(row),
            }
        }
        lane.rows.sort_by_key(|row| row.document_unit_index);
    }

    fn register_units(&mut self, observation: &PageFlowLaneObservation) {
        for row in &observation.rows {
            let previous_lane = self
                .unit_lanes
                .insert(row.unit_key.clone(), observation.lane_id.clone());
            assert!(
                previous_lane.is_none_or(|lane| lane == observation.lane_id),
                "a document unit cannot be observed in more than one flow lane"
            );
            self.units.insert(row.unit_key.clone(), row.clone());
        }
    }

    fn mounted_projection(
        &self,
        target: &PageFlowPinTarget,
    ) -> Option<PageFlowSemanticAnchorProjection> {
        let (outer_item, outer_index, target_top) = match target {
            PageFlowPinTarget::Unit(key) => {
                let unit = self.units.get(key)?;
                (
                    &unit.outer_item,
                    unit.outer_index,
                    unit.bounds.top().as_f32(),
                )
            }
            PageFlowPinTarget::Node(id) => {
                let node = self.nodes.get(id)?;
                (
                    &node.outer_item,
                    node.outer_index,
                    node.bounds.top().as_f32(),
                )
            }
            PageFlowPinTarget::Outer(PageDocumentOuterItemId::Flow(id)) => {
                let node = self.nodes.get(id)?;
                (
                    &node.outer_item,
                    node.outer_index,
                    node.bounds.top().as_f32(),
                )
            }
            PageFlowPinTarget::Outer(
                PageDocumentOuterItemId::Lead | PageDocumentOuterItemId::Footer,
            ) => return None,
        };
        let PageDocumentOuterItemId::Flow(root_id) = outer_item else {
            return None;
        };
        let root = self.nodes.get(root_id)?;
        let local_y = target_top - root.bounds.top().as_f32();
        Some(PageFlowSemanticAnchorProjection {
            target: target.clone(),
            outer_item: outer_item.clone(),
            outer_index,
            outer_local_y: PageFlowOuterLocalY::new(local_y),
        })
    }

    fn target_bounds(&self, target: &PageFlowPinTarget) -> Option<Bounds<Pixels>> {
        match target {
            PageFlowPinTarget::Unit(key) => self.units.get(key).map(|unit| unit.bounds),
            PageFlowPinTarget::Node(id)
            | PageFlowPinTarget::Outer(PageDocumentOuterItemId::Flow(id)) => {
                self.nodes.get(id).map(|node| node.bounds)
            }
            PageFlowPinTarget::Outer(
                PageDocumentOuterItemId::Lead | PageDocumentOuterItemId::Footer,
            ) => None,
        }
    }
}
