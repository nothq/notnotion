use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use crate::ui::{LoadedCardPageData, PageDocumentUnitKey, PageFlowNodeId, PageFlowSequenceId};

mod cache;
mod frame;
mod frame_lifecycle;
mod layout_width;
mod measurement_batch;
mod metrics;
mod pins;
mod prefix;
mod semantic_projection;
mod state;
mod surface;

use cache::{PageFlowMeasurementKey, PAGE_FLOW_MEASUREMENTS_GLOBAL};
pub(crate) use frame::{
    PageFlowDeferredScrollbarAnchor, PageFlowLayoutCommitSchedule, PageFlowLayoutFrameGeneration,
    PageFlowLayoutFrameToken, PageFlowLogicalViewportBasis, PageFlowOuterExtentAuthority,
    PageFlowOuterItemExtent, PageFlowOuterItemMatch, PageFlowOuterItemRemap, PageFlowOuterLocalY,
    PageFlowSemanticAnchorBaseline, PageFlowSemanticAnchorIntent, PageFlowSemanticAnchorProjection,
    PageFlowSemanticAnchorTransaction, PageFlowViewportRelativeY,
};
use frame_lifecycle::commit_surface_frame;
pub(crate) use layout_width::{
    allocate_page_flow_lane_widths, PageFlowExactLayoutWidth, PageFlowLayoutWidth,
    PageFlowSequenceWidth,
};
pub(crate) use measurement_batch::PageFlowLayoutCommit;
use measurement_batch::PageFlowMeasuredExtent;
pub(crate) use pins::{PageFlowPinSet, PageFlowPinTarget};
use state::{PageFlowPageState, PageFlowRenderPins, PageFlowSurfaceState};
use surface::{section_witness, PageFlowPlanningContext};

const PAGE_FLOW_PAGE_CACHE_LIMIT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageFlowSurfaceKey {
    Standalone,
    SelectedPage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageFlowRenderGeneration(u64);

#[derive(Clone, Copy, Debug)]
pub(crate) enum PageFlowViewport {
    Known { start: f32, end: f32 },
    UnknownLeading,
    UnknownAtAnchor { offset: f32 },
}

impl PageFlowViewport {
    pub(crate) fn known(start: f32, end: f32) -> Self {
        assert!(start.is_finite() && end.is_finite() && start <= end);
        Self::Known { start, end }
    }

    pub(crate) const fn unknown_leading() -> Self {
        Self::UnknownLeading
    }

    pub(crate) fn unknown_at_anchor(offset: f32) -> Self {
        assert!(offset.is_finite() && offset >= 0.0);
        Self::UnknownAtAnchor { offset }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PageFlowPlannedNode {
    pub(crate) index: usize,
    pub(crate) offset: f32,
    pub(crate) extent: f32,
}

pub(crate) enum PageFlowRenderSpan {
    Spacer { extent: f32 },
    Nodes(Arc<[PageFlowPlannedNode]>),
}

pub(crate) struct PageFlowSequencePlan {
    pub(crate) spans: Vec<PageFlowRenderSpan>,
    pub(crate) total_extent: f32,
}

#[derive(Default)]
pub(crate) struct PageFlowVirtualizer {
    pages: HashMap<String, PageFlowPageState>,
    page_lru: VecDeque<String>,
    next_render_generation: u64,
    next_layout_generation: u64,
}

pub(crate) struct PageFlowSectionMeasurement<'a> {
    pub(crate) frame: &'a PageFlowLayoutFrameToken,
    pub(crate) node_id: &'a PageFlowNodeId,
    pub(crate) width: PageFlowExactLayoutWidth,
    pub(crate) extent: f32,
}

impl PageFlowVirtualizer {
    pub(crate) fn reconcile_surface_projection(
        &mut self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
    ) {
        let generation = self.allocate_render_generation();
        let page_id = data.page.block_id.clone();
        self.touch_page(&page_id);
        self.pages.entry(page_id).or_default().reconcile_surface(
            data,
            surface,
            generation,
            PageFlowRenderPins {
                pins: &PageFlowPinSet::default(),
                semantic_anchor: None,
            },
        );
        self.prune_pages();
    }

    pub(crate) fn deactivate_surface(&mut self, page_id: &str, surface: PageFlowSurfaceKey) {
        if let Some(page) = self.pages.get_mut(page_id) {
            page.surfaces.remove(&surface);
        }
    }

    pub(crate) fn invalidate_document_unit_layout(
        &mut self,
        page_id: &str,
        unit_key: &PageDocumentUnitKey,
    ) -> bool {
        let Some(surface_nodes) = self.pages.get(page_id).map(|page| {
            page.surfaces
                .iter()
                .filter_map(|(surface, state)| {
                    let data = state.data()?;
                    let location = data.flow.location(unit_key)?;
                    let node_id = location.node_path.key().node_id();
                    let root_node_id = data.flow.outer_item_for_node(&node_id)?.key.node_id();
                    Some((*surface, node_id, root_node_id))
                })
                .collect::<Vec<_>>()
        }) else {
            return false;
        };
        let generations = surface_nodes
            .iter()
            .map(|(surface, _, _)| (*surface, self.allocate_render_generation()))
            .collect::<HashMap<_, _>>();
        let page = self
            .pages
            .get_mut(page_id)
            .expect("invalidated Notion flow page must remain cached");
        page.measurements.remove_unit(unit_key);
        for (surface, node_id, root_node_id) in surface_nodes {
            page.surfaces
                .get_mut(&surface)
                .expect("invalidated Notion flow surface must remain cached")
                .invalidate_section_layout(&node_id, root_node_id, generations[&surface]);
        }
        true
    }

    pub(crate) fn prepare_recursive_layout_frame(
        &mut self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        pins: &PageFlowPinSet,
        semantic_anchor: Option<PageFlowSemanticAnchorTransaction>,
    ) -> PageFlowLayoutFrameToken {
        self.prepare_layout_frame(data, surface, pins, semantic_anchor)
    }

    fn prepare_layout_frame(
        &mut self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        pins: &PageFlowPinSet,
        semantic_anchor: Option<PageFlowSemanticAnchorTransaction>,
    ) -> PageFlowLayoutFrameToken {
        let candidate_render = self.allocate_render_generation();
        let layout_generation = self.allocate_layout_generation();
        let page_id = data.page.block_id.clone();
        let semantic_target = semantic_anchor
            .as_ref()
            .map(|anchor| &anchor.baseline.target);
        self.touch_page(&page_id);
        let render_generation = self
            .pages
            .entry(page_id.clone())
            .or_default()
            .reconcile_surface(
                data,
                surface,
                candidate_render,
                PageFlowRenderPins {
                    pins,
                    semantic_anchor: semantic_target,
                },
            );
        let token = PageFlowLayoutFrameToken::new(
            Arc::from(page_id.as_str()),
            surface,
            render_generation,
            layout_generation,
        );
        self.pages
            .get_mut(&page_id)
            .and_then(|page| page.surfaces.get_mut(&surface))
            .expect("prepared Notion flow surface must exist")
            .begin_frame(token.clone(), semantic_anchor);
        self.prune_pages();
        token
    }

    pub(crate) fn plan_sequence(
        &mut self,
        frame: &PageFlowLayoutFrameToken,
        sequence_id: &PageFlowSequenceId,
        width: PageFlowSequenceWidth,
        viewport: PageFlowViewport,
    ) -> Option<PageFlowSequencePlan> {
        let page = self.pages.get_mut(frame.page_id())?;
        let PageFlowPageState {
            measurements,
            surfaces,
        } = page;
        let state = surfaces.get_mut(&frame.surface())?;
        let PageFlowSurfaceState {
            projection,
            metrics,
            frame: current,
            ..
        } = state;
        let current = current.as_mut().filter(|current| current.token == *frame)?;
        Some(metrics.plan_sequence(
            PageFlowPlanningContext {
                projection,
                measurements,
                journal: &mut current.journal,
            },
            sequence_id,
            width,
            viewport,
        ))
    }

    pub(crate) fn queue_section_measurement(
        &mut self,
        request: PageFlowSectionMeasurement<'_>,
    ) -> bool {
        let Some(state) = self.current_surface_mut(request.frame) else {
            return false;
        };
        if section_witness(&state.projection, request.node_id).is_none() {
            return false;
        }
        let PageFlowSurfaceState {
            frame,
            measurement_continuation,
            ..
        } = state;
        let Some(frame) = frame.as_mut().filter(|frame| frame.token == *request.frame) else {
            return false;
        };
        frame.measurements.admit(
            measurement_continuation,
            PageFlowMeasurementKey {
                node_id: request.node_id.clone(),
                width: request.width,
            },
            PageFlowMeasuredExtent::new(request.extent),
        );
        true
    }

    pub(crate) fn commit_layout_frame(
        &mut self,
        token: &PageFlowLayoutFrameToken,
    ) -> Option<PageFlowLayoutCommit> {
        let commit = self.commit_page_frame(token)?;
        self.prune_measurements();
        Some(commit)
    }

    pub(crate) fn estimate_semantic_projection(
        &self,
        frame: &PageFlowLayoutFrameToken,
        target: &PageFlowPinTarget,
    ) -> Option<PageFlowSemanticAnchorProjection> {
        let state = self
            .pages
            .get(frame.page_id())?
            .surfaces
            .get(&frame.surface())?;
        let current = state.frame.as_ref()?;
        if state.render_generation != frame.render_generation() || current.token != *frame {
            return None;
        }
        let data = state.data()?;
        semantic_projection::estimate(state, &data, target)
    }

    pub(crate) fn clear(&mut self) {
        self.pages.clear();
        self.page_lru.clear();
    }

    fn current_surface_mut(
        &mut self,
        token: &PageFlowLayoutFrameToken,
    ) -> Option<&mut PageFlowSurfaceState> {
        let state = self
            .pages
            .get_mut(token.page_id())?
            .surfaces
            .get_mut(&token.surface())?;
        (state.render_generation == token.render_generation()).then_some(state)
    }

    fn commit_page_frame(
        &mut self,
        token: &PageFlowLayoutFrameToken,
    ) -> Option<PageFlowLayoutCommit> {
        let page = self.pages.get_mut(token.page_id())?;
        let PageFlowPageState {
            measurements,
            surfaces,
        } = page;
        let state = surfaces.get_mut(&token.surface())?;
        if state.render_generation != token.render_generation() {
            return None;
        }
        commit_surface_frame(measurements, state, token)
    }

    fn allocate_render_generation(&mut self) -> PageFlowRenderGeneration {
        let generation = PageFlowRenderGeneration(self.next_render_generation);
        self.next_render_generation = self
            .next_render_generation
            .checked_add(1)
            .expect("Notion flow render generation must not overflow");
        generation
    }

    fn allocate_layout_generation(&mut self) -> PageFlowLayoutFrameGeneration {
        let generation = PageFlowLayoutFrameGeneration(self.next_layout_generation);
        self.next_layout_generation = self
            .next_layout_generation
            .checked_add(1)
            .expect("Notion flow layout generation must not overflow");
        generation
    }

    fn touch_page(&mut self, page_id: &str) {
        self.page_lru.retain(|candidate| candidate != page_id);
        self.page_lru.push_back(page_id.to_owned());
    }

    fn prune_pages(&mut self) {
        while self.pages.len() > PAGE_FLOW_PAGE_CACHE_LIMIT {
            let oldest = self
                .page_lru
                .pop_front()
                .expect("Notion flow page LRU must cover cached pages");
            self.pages.remove(&oldest);
        }
    }

    fn prune_measurements(&mut self) {
        while self.measurement_count() > PAGE_FLOW_MEASUREMENTS_GLOBAL {
            let removed = self.page_lru.iter().any(|page_id| {
                self.pages
                    .get_mut(page_id)
                    .is_some_and(|page| page.measurements.remove_oldest())
            });
            assert!(removed, "Notion flow measurement LRU must make progress");
        }
    }

    fn measurement_count(&self) -> usize {
        self.pages
            .values()
            .map(|page| page.measurements.len())
            .sum()
    }
}
