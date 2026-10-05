use crate::ui::surface::{
    PageDocumentDragRuntime, PageDocumentFlowRuntime, PageFlowDragAuthority, PageFlowDropAnchor,
    PageFlowLaneObservation, PageFlowSurfaceKey,
};

use super::super::super::{
    Arc, LoadedCardPageData, PageBlockDragObservation, PageBlockDragScrollTarget,
    PageBlockDragTarget, PageBlockDragWash, PAGE_BLOCK_DROP_LINE_HEIGHT,
};

pub(in crate::ui::board_workspace::page::editor::drag) fn page_flow_surface(
    scroll_target: PageBlockDragScrollTarget,
) -> PageFlowSurfaceKey {
    match scroll_target {
        PageBlockDragScrollTarget::Standalone => PageFlowSurfaceKey::Standalone,
        PageBlockDragScrollTarget::SelectedPage => PageFlowSurfaceKey::SelectedPage,
    }
}

fn page_flow_scroll_target(surface: PageFlowSurfaceKey) -> PageBlockDragScrollTarget {
    match surface {
        PageFlowSurfaceKey::Standalone => PageBlockDragScrollTarget::Standalone,
        PageFlowSurfaceKey::SelectedPage => PageBlockDragScrollTarget::SelectedPage,
    }
}

impl PageDocumentDragRuntime {
    pub(in crate::ui::board_workspace::page::editor) fn refresh_from_flow_observations(
        &self,
        flow: &PageDocumentFlowRuntime,
        surface: PageFlowSurfaceKey,
        data: &Arc<LoadedCardPageData>,
    ) -> bool {
        if !data.has_column_structure() {
            return false;
        }
        let scroll_target = page_flow_scroll_target(surface);
        let Some(observation) = self.borrow().layouts.get(scroll_target).observation.clone() else {
            return false;
        };
        let next_target = {
            let observations = flow.state().observations.borrow();
            observations
                .drag_authority(surface, data)
                .and_then(|authority| resolve_page_flow_drag_target(data, &authority, &observation))
        };
        let mut drag = self.borrow_mut();
        if drag.target == next_target {
            return false;
        }
        drag.target = next_target;
        true
    }
}

pub(in crate::ui::board_workspace::page::editor::drag) fn resolve_page_flow_drag_target(
    data: &Arc<LoadedCardPageData>,
    authority: &PageFlowDragAuthority<'_>,
    observation: &PageBlockDragObservation,
) -> Option<PageBlockDragTarget> {
    let lane = resolve_drop_lane(data, authority, observation)?;
    observation
        .payload
        .preview_width
        .set_recursive(lane.lane_bounds.size.width.as_f32());
    let parent_id = lane
        .lane_path
        .column_ids()
        .map_or(data.page.block_id.as_str(), |(_, column_id)| column_id);
    let anchors = authority.anchors_for(&lane.lane_path);
    let before = before_anchor(anchors, observation, observation.pointer.y.as_f32());
    validate_flow_drop(data, observation, parent_id, before)?;
    build_flow_target(
        data,
        lane,
        observation,
        PageFlowDropPlacement { parent_id, before },
        anchors
            .iter()
            .all(|anchor| observation.payload.subtree_contains(&anchor.block_id)),
    )
}

fn resolve_drop_lane<'a>(
    data: &Arc<LoadedCardPageData>,
    authority: &'a PageFlowDragAuthority<'_>,
    observation: &PageBlockDragObservation,
) -> Option<&'a PageFlowLaneObservation> {
    valid_flow_observation(data, authority, observation).then_some(())?;
    authority.lanes().iter().min_by(|left, right| {
        left.rank(observation.pointer)
            .compare(right.rank(observation.pointer))
    })
}

fn validate_flow_drop(
    data: &Arc<LoadedCardPageData>,
    observation: &PageBlockDragObservation,
    parent_id: &str,
    before: Option<&PageFlowDropAnchor>,
) -> Option<()> {
    if observation.payload.subtree_contains(parent_id)
        || before.is_some_and(|anchor| observation.payload.subtree_contains(&anchor.block_id))
    {
        return None;
    }
    observation
        .payload
        .source_counts_allow_target(data, parent_id)
        .then_some(())
}

fn valid_flow_observation(
    data: &Arc<LoadedCardPageData>,
    authority: &PageFlowDragAuthority<'_>,
    observation: &PageBlockDragObservation,
) -> bool {
    data.has_column_structure()
        && authority.page_id() == data.page.block_id
        && observation.pointer.x >= observation.container_bounds.left()
        && observation.pointer.x <= observation.container_bounds.right()
        && observation.payload.source_allocation_matches(data)
}

fn before_anchor<'a>(
    anchors: &'a [PageFlowDropAnchor],
    observation: &PageBlockDragObservation,
    pointer_y: f32,
) -> Option<&'a PageFlowDropAnchor> {
    anchors
        .iter()
        .filter(|anchor| !observation.payload.subtree_contains(&anchor.block_id))
        .find(|anchor| {
            pointer_y <= (anchor.bounds.top().as_f32() + anchor.bounds.bottom().as_f32()) / 2.0
        })
}

struct PageFlowDropPlacement<'a> {
    parent_id: &'a str,
    before: Option<&'a PageFlowDropAnchor>,
}

fn build_flow_target(
    data: &Arc<LoadedCardPageData>,
    lane: &PageFlowLaneObservation,
    observation: &PageBlockDragObservation,
    placement: PageFlowDropPlacement<'_>,
    empty: bool,
) -> Option<PageBlockDragTarget> {
    let PageFlowDropPlacement { parent_id, before } = placement;
    let container = observation.container_bounds;
    let lane_bounds = lane.lane_bounds;
    let raw_top = before
        .map(|anchor| anchor.bounds.top().as_f32())
        .unwrap_or_else(|| lane_bounds.bottom().as_f32());
    let maximum_top = (lane_bounds.bottom().as_f32() - PAGE_BLOCK_DROP_LINE_HEIGHT)
        .max(lane_bounds.top().as_f32());
    let indicator_top = raw_top.clamp(lane_bounds.top().as_f32(), maximum_top);
    let destination_depth = if parent_id == data.page.block_id {
        0
    } else {
        data.page_block(parent_id)?.depth + 1
    };
    Some(PageBlockDragTarget {
        scroll_target: observation.scroll_target,
        page_id: data.page.block_id.clone(),
        dragged_root_block_ids: observation.payload.root_block_ids().to_vec(),
        target_parent_block_id: parent_id.to_string(),
        before_block_id: before.map(|anchor| anchor.block_id.to_string()),
        destination_depth,
        indicator_left: (lane_bounds.left() - container.left()).as_f32(),
        indicator_top: indicator_top - container.top().as_f32(),
        indicator_width: lane_bounds.size.width.as_f32(),
        wash: empty.then(|| PageBlockDragWash {
            top: (lane_bounds.top() - container.top()).as_f32(),
            left: (lane_bounds.left() - container.left()).as_f32(),
            width: lane_bounds.size.width.as_f32(),
            height: lane_bounds.size.height.as_f32(),
        }),
    })
}
