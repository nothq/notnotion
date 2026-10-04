use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};

use crate::ui::{LoadedCardPageData, PageFlowSequenceId};

use super::{
    cache::PageFlowMeasurementCache,
    frame::{
        PageFlowDeferredScrollbarAnchor, PageFlowLayoutFrameGeneration, PageFlowLayoutFrameMode,
        PageFlowLayoutFrameToken, PageFlowPlanningMutationJournal,
    },
    measurement_batch::{PageFlowMeasurementBatch, PageFlowMeasurementContinuation},
    metrics::PageFlowProjectionMetrics,
    pins::{pin_node_path, PageFlowPinSet, PageFlowPinTarget, PageFlowPinnedNodes},
    surface::PageFlowSurfaceMetrics,
    PageFlowRenderGeneration, PageFlowSequenceWidth, PageFlowSurfaceKey,
};

#[derive(Default)]
pub(super) struct PageFlowPageState {
    pub(super) measurements: PageFlowMeasurementCache,
    pub(super) surfaces: HashMap<PageFlowSurfaceKey, PageFlowSurfaceState>,
}

pub(super) struct PageFlowSurfaceState {
    allocation: Weak<LoadedCardPageData>,
    projection_generation: u64,
    pub(super) render_generation: PageFlowRenderGeneration,
    pub(super) projection: PageFlowProjectionMetrics,
    pub(super) metrics: PageFlowSurfaceMetrics,
    pub(super) frame: Option<PageFlowLayoutFrameState>,
    pub(super) deferred_scrollbar_anchor: Option<PageFlowDeferredScrollbarState>,
    pub(super) measurement_continuation: PageFlowMeasurementContinuation,
    pub(super) pending_dirty_root_ids: HashSet<crate::ui::PageFlowNodeId>,
    committed_layout_generation: Option<PageFlowLayoutFrameGeneration>,
}

pub(super) struct PageFlowLayoutFrameState {
    pub(super) token: PageFlowLayoutFrameToken,
    pub(super) mode: PageFlowLayoutFrameMode,
    pub(super) journal: PageFlowPlanningMutationJournal,
    pub(super) measurements: PageFlowMeasurementBatch,
    pub(super) commit_scheduled: bool,
}

pub(super) struct PageFlowDeferredScrollbarState {
    pub(super) anchor: PageFlowDeferredScrollbarAnchor,
}

/// The pins and semantic anchor target a surface keeps rendered.
pub(super) struct PageFlowRenderPins<'a> {
    pub(super) pins: &'a PageFlowPinSet,
    pub(super) semantic_anchor: Option<&'a PageFlowPinTarget>,
}

/// What a replaced surface state hands on to its replacement.
struct PageFlowSurfaceCarryover {
    widths: HashMap<PageFlowSequenceId, PageFlowSequenceWidth>,
    deferred_scrollbar_anchor: Option<PageFlowDeferredScrollbarState>,
    pending_dirty_root_ids: HashSet<crate::ui::PageFlowNodeId>,
}

impl PageFlowPageState {
    pub(super) fn reconcile_surface(
        &mut self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        generation: PageFlowRenderGeneration,
        render_pins: PageFlowRenderPins<'_>,
    ) -> PageFlowRenderGeneration {
        let PageFlowRenderPins {
            pins,
            semantic_anchor,
        } = render_pins;
        let replace = self
            .surfaces
            .get(&surface)
            .is_none_or(|state| !state.matches_projection(data));
        if replace {
            let previous = self.surfaces.remove(&surface);
            let widths = previous
                .as_ref()
                .map_or_else(HashMap::new, |state| state.metrics.reconciliation_widths());
            let mut dirty = previous.as_ref().map_or_else(HashSet::new, |state| {
                state
                    .projection
                    .changed_root_ids(data)
                    .into_iter()
                    .collect()
            });
            if let Some(previous) = previous.as_ref() {
                dirty.extend(previous.pending_dirty_root_ids.iter().cloned());
            }
            let deferred = previous.and_then(|state| state.deferred_scrollbar_anchor);
            self.surfaces.insert(
                surface,
                PageFlowSurfaceState::new(
                    data,
                    surface,
                    generation,
                    PageFlowSurfaceCarryover {
                        widths,
                        deferred_scrollbar_anchor: deferred,
                        pending_dirty_root_ids: dirty,
                    },
                    &mut self.measurements,
                ),
            );
        }
        let state = self
            .surfaces
            .get_mut(&surface)
            .expect("reconciled Notion flow surface must exist");
        state
            .metrics
            .set_pins(PageFlowPinnedNodes::new(&data.flow, pins, semantic_anchor));
        state.render_generation
    }
}

impl PageFlowSurfaceState {
    fn new(
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        generation: PageFlowRenderGeneration,
        carryover: PageFlowSurfaceCarryover,
        measurements: &mut PageFlowMeasurementCache,
    ) -> Self {
        let PageFlowSurfaceCarryover {
            widths,
            deferred_scrollbar_anchor,
            pending_dirty_root_ids,
        } = carryover;
        let deferred_scrollbar_anchor = deferred_scrollbar_anchor
            .filter(|deferred| pin_node_path(&data.flow, deferred.anchor.target()).is_some());
        let projection = PageFlowProjectionMetrics::new(data, surface);
        let metrics = PageFlowSurfaceMetrics::new(&projection, widths, measurements);
        Self {
            allocation: Arc::downgrade(data),
            projection_generation: data.flow_projection_generation,
            render_generation: generation,
            projection,
            metrics,
            frame: None,
            deferred_scrollbar_anchor,
            measurement_continuation: PageFlowMeasurementContinuation::default(),
            pending_dirty_root_ids,
            committed_layout_generation: None,
        }
    }

    fn matches_projection(&self, data: &Arc<LoadedCardPageData>) -> bool {
        Weak::ptr_eq(&self.allocation, &Arc::downgrade(data))
            && self.projection_generation == data.flow_projection_generation
    }

    pub(super) fn begin_frame(
        &mut self,
        token: PageFlowLayoutFrameToken,
        semantic_anchor: Option<super::PageFlowSemanticAnchorTransaction>,
    ) {
        assert_eq!(self.render_generation, token.render_generation());
        if semantic_anchor.is_none() {
            self.deferred_scrollbar_anchor = None;
        }
        let journal = match self.frame.take() {
            Some(PageFlowLayoutFrameState {
                journal,
                measurements,
                ..
            }) => {
                measurements.spill_into(&mut self.measurement_continuation);
                journal
            }
            None => PageFlowPlanningMutationJournal::default(),
        };
        let measurements =
            PageFlowMeasurementBatch::from_continuation(&mut self.measurement_continuation);
        self.frame = Some(PageFlowLayoutFrameState::new(
            token,
            PageFlowLayoutFrameMode::Recursive(semantic_anchor),
            journal,
            measurements,
        ));
    }

    pub(super) fn current_frame_mut(
        &mut self,
        token: &PageFlowLayoutFrameToken,
    ) -> Option<&mut PageFlowLayoutFrameState> {
        self.frame.as_mut().filter(|frame| frame.token == *token)
    }

    pub(super) fn mark_committed(&mut self, token: &PageFlowLayoutFrameToken) {
        assert_eq!(self.render_generation, token.render_generation());
        assert!(self.frame.is_none());
        self.committed_layout_generation = Some(token.layout_generation());
    }

    pub(super) fn accepts_committed_frame(&self, token: &PageFlowLayoutFrameToken) -> bool {
        self.render_generation == token.render_generation()
            && self.frame.is_none()
            && self.committed_layout_generation == Some(token.layout_generation())
    }

    pub(super) fn contains_target(&self, target: &PageFlowPinTarget) -> bool {
        self.allocation.upgrade().is_some_and(|data| match target {
            PageFlowPinTarget::Outer(crate::ui::PageDocumentOuterItemId::Lead)
            | PageFlowPinTarget::Outer(crate::ui::PageDocumentOuterItemId::Footer) => true,
            _ => pin_node_path(&data.flow, target).is_some(),
        })
    }

    pub(super) fn data(&self) -> Option<Arc<LoadedCardPageData>> {
        self.allocation.upgrade()
    }

    pub(super) fn invalidate_section_layout(
        &mut self,
        node_id: &crate::ui::PageFlowNodeId,
        root_node_id: crate::ui::PageFlowNodeId,
        generation: PageFlowRenderGeneration,
    ) {
        let root_index = self
            .metrics
            .invalidate_section_node(&self.projection, node_id);
        assert_eq!(self.projection.root_index(&root_node_id), Some(root_index));
        self.pending_dirty_root_ids.insert(root_node_id);
        self.render_generation = generation;
        self.frame = None;
        self.measurement_continuation.clear();
        self.committed_layout_generation = None;
    }
}

impl PageFlowLayoutFrameState {
    fn new(
        token: PageFlowLayoutFrameToken,
        mode: PageFlowLayoutFrameMode,
        journal: PageFlowPlanningMutationJournal,
        measurements: PageFlowMeasurementBatch,
    ) -> Self {
        Self {
            token,
            mode,
            journal,
            measurements,
            commit_scheduled: false,
        }
    }
}
