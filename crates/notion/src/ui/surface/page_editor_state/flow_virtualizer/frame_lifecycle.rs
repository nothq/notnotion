use super::{
    cache::{PageFlowMeasurementCache, PageFlowMeasurementKey},
    frame::PageFlowLayoutFrameMode,
    measurement_batch::{PageFlowDirtyRootRanges, PageFlowLayoutCommit, PageFlowMeasuredExtent},
    metrics::PageFlowProjectionMetrics,
    state::{PageFlowDeferredScrollbarState, PageFlowSurfaceState},
    surface::{measurement_admits_cache_update, section_witness, PageFlowSurfaceMetrics},
    PageFlowDeferredScrollbarAnchor, PageFlowExactLayoutWidth, PageFlowLayoutCommitSchedule,
    PageFlowLayoutFrameToken, PageFlowLogicalViewportBasis, PageFlowOuterExtentAuthority,
    PageFlowSemanticAnchorProjection, PageFlowSemanticAnchorTransaction, PageFlowVirtualizer,
};

pub(super) fn commit_surface_frame(
    measurements: &mut PageFlowMeasurementCache,
    state: &mut PageFlowSurfaceState,
    token: &PageFlowLayoutFrameToken,
) -> Option<PageFlowLayoutCommit> {
    let frame = state.frame.as_ref()?;
    if frame.token != *token || !frame.commit_scheduled {
        return None;
    }
    let mut frame = state
        .frame
        .take()
        .expect("validated Notion flow layout frame must exist");
    let PageFlowLayoutFrameMode::Recursive(semantic_anchor) = frame.mode;
    let entries = frame.measurements.take_entries();
    cache_measurement_entries(measurements, &state.projection, &state.metrics, &entries);
    state
        .metrics
        .apply_measurement_entries(&state.projection, &entries, &mut frame.journal);
    let journal = std::mem::take(&mut frame.journal);
    let mut dirty = PageFlowDirtyRootRanges::default();
    dirty.extend_journal(&journal);
    dirty.extend_indices(
        std::mem::take(&mut state.pending_dirty_root_ids)
            .into_iter()
            .filter_map(|node_id| state.projection.root_index(&node_id)),
    );
    let measurements_pending = !state.measurement_continuation.is_empty();
    state.mark_committed(token);
    Some(PageFlowLayoutCommit {
        frame: token.clone(),
        dirty_outer_ranges: dirty.outer_list_ranges(),
        semantic_anchor,
        measurements_pending,
    })
}

fn cache_measurement_entries(
    cache: &mut PageFlowMeasurementCache,
    projection: &PageFlowProjectionMetrics,
    surface: &PageFlowSurfaceMetrics,
    entries: &[(PageFlowMeasurementKey, PageFlowMeasuredExtent)],
) {
    for (key, measured) in entries {
        let witness = section_witness(projection, &key.node_id)
            .expect("measured Notion Section must retain its layout witness");
        let cached_extent = cache.exact_extent(key, &witness);
        if !measurement_admits_cache_update(surface, projection, key, *measured, cached_extent) {
            continue;
        }
        cache.insert(key.clone(), witness, measured.pixels());
    }
}

impl PageFlowVirtualizer {
    pub(crate) fn semantic_anchor_target(
        &self,
        token: &PageFlowLayoutFrameToken,
    ) -> Option<super::PageFlowPinTarget> {
        let state = self
            .pages
            .get(token.page_id())?
            .surfaces
            .get(&token.surface())?;
        if state.render_generation != token.render_generation() {
            return None;
        }
        let frame = state.frame.as_ref().filter(|frame| frame.token == *token)?;
        frame
            .mode
            .semantic_anchor()
            .map(|anchor| anchor.baseline.target.clone())
    }

    pub(crate) fn observe_root_sequence_exact_width(
        &mut self,
        token: &PageFlowLayoutFrameToken,
        width: PageFlowExactLayoutWidth,
    ) -> bool {
        let Some(state) = self.current_surface_mut(token) else {
            return false;
        };
        if state.current_frame_mut(token).is_none() {
            return false;
        }
        state.metrics.observe_root_sequence_exact_width(width);
        true
    }

    pub(crate) fn update_semantic_anchor_projection(
        &mut self,
        token: &PageFlowLayoutFrameToken,
        projection: PageFlowSemanticAnchorProjection,
    ) -> bool {
        let Some(state) = self.current_surface_mut(token) else {
            return false;
        };
        if !state.contains_target(&projection.target) {
            return false;
        }
        let Some(frame) = state.current_frame_mut(token) else {
            return false;
        };
        let Some(anchor) = frame.mode.semantic_anchor_mut() else {
            return false;
        };
        if anchor.baseline.target != projection.target {
            return false;
        }
        anchor.update_projection(projection);
        true
    }

    pub(crate) fn schedule_layout_frame_commit(
        &mut self,
        token: &PageFlowLayoutFrameToken,
    ) -> PageFlowLayoutCommitSchedule {
        let Some(state) = self.current_surface_mut(token) else {
            return PageFlowLayoutCommitSchedule::Stale;
        };
        let Some(frame) = state.current_frame_mut(token) else {
            return PageFlowLayoutCommitSchedule::Stale;
        };
        if frame.commit_scheduled {
            return PageFlowLayoutCommitSchedule::AlreadyScheduled;
        }
        frame.commit_scheduled = true;
        PageFlowLayoutCommitSchedule::Schedule
    }

    pub(crate) fn defer_scrollbar_anchor(
        &mut self,
        token: &PageFlowLayoutFrameToken,
        transaction: PageFlowSemanticAnchorTransaction,
        viewport: PageFlowLogicalViewportBasis,
    ) -> bool {
        let Some(state) = self.committed_surface_mut(token) else {
            return false;
        };
        if !state.contains_target(&transaction.baseline.target) {
            return false;
        }
        match state.deferred_scrollbar_anchor.as_mut() {
            Some(deferred) => deferred.anchor.update_latest(transaction, viewport),
            None => {
                state.deferred_scrollbar_anchor = Some(PageFlowDeferredScrollbarState {
                    anchor: PageFlowDeferredScrollbarAnchor::new(transaction, viewport),
                });
            }
        }
        true
    }

    pub(crate) fn resume_deferred_scrollbar_anchor(
        &mut self,
        token: &PageFlowLayoutFrameToken,
        current: PageFlowSemanticAnchorTransaction,
        viewport: PageFlowLogicalViewportBasis,
        extents: &PageFlowOuterExtentAuthority,
    ) -> PageFlowSemanticAnchorTransaction {
        let Some(state) = self.committed_surface_mut(token) else {
            return current;
        };
        let Some(deferred) = state.deferred_scrollbar_anchor.take() else {
            return current;
        };
        if !state.contains_target(deferred.anchor.target()) {
            return current;
        }
        deferred
            .anchor
            .resume(current.clone(), viewport, extents)
            .unwrap_or(current)
    }

    /// A deferred anchor of any shape must be consumed by the full commit path,
    /// so a stable frame may not skip past `resume_deferred_scrollbar_anchor`
    /// and strand it for a later frame to resurrect.
    pub(crate) fn has_deferred_scrollbar_anchor(&self, token: &PageFlowLayoutFrameToken) -> bool {
        self.pages
            .get(token.page_id())
            .and_then(|page| page.surfaces.get(&token.surface()))
            .filter(|state| state.accepts_committed_frame(token))
            .is_some_and(|state| state.deferred_scrollbar_anchor.is_some())
    }

    fn committed_surface_mut(
        &mut self,
        token: &PageFlowLayoutFrameToken,
    ) -> Option<&mut PageFlowSurfaceState> {
        let state = self.current_surface_mut(token)?;
        state.accepts_committed_frame(token).then_some(state)
    }
}
