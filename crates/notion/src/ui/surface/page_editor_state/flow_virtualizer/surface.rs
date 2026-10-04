use std::{collections::HashMap, ops::Range};

use crate::ui::{PageFlowNodeId, PageFlowSequenceId};

use super::{
    cache::{PageFlowMeasurementCache, PageFlowMeasurementKey},
    frame::PageFlowPlanningMutationJournal,
    metrics::{PageFlowProjectionMetrics, PageFlowSectionLayoutWitness},
    pins::PageFlowPinnedNodes,
    prefix::{PageFlowExtentChange, PageFlowMaxExtents, PageFlowPrefixExtents},
    PageFlowExactLayoutWidth, PageFlowLayoutWidth, PageFlowSequencePlan, PageFlowSequenceWidth,
    PageFlowViewport,
};

mod hydration;
mod measurements;
mod planning;

use hydration::hydrate_surface_metrics;
pub(super) use measurements::measurement_admits_cache_update;
use measurements::root_outer_index_for_node;
use planning::{render_spans, render_windows, visible_node_range};

const PAGE_FLOW_WIDTH_REFRESH_PASSES: usize = 2;

pub(super) struct PageFlowSurfaceMetrics {
    pub(super) sequences: HashMap<PageFlowSequenceId, PageFlowSequenceMetrics>,
    pub(super) column_maxima: HashMap<PageFlowNodeId, PageFlowMaxExtents>,
    pub(super) pins: PageFlowPinnedNodes,
}

/// The projection a planning pass reads, and the measurement cache and
/// mutation journal it updates.
pub(super) struct PageFlowPlanningContext<'a> {
    pub(super) projection: &'a PageFlowProjectionMetrics,
    pub(super) measurements: &'a mut PageFlowMeasurementCache,
    pub(super) journal: &'a mut PageFlowPlanningMutationJournal,
}

pub(super) struct PageFlowSequenceMetrics {
    pub(super) width: Option<PageFlowSequenceWidth>,
    pub(super) extents: PageFlowPrefixExtents,
    pub(super) authorities: Vec<PageFlowNodeExtentAuthority>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PageFlowNodeExtentAuthority {
    Provisional,
    ExactAt(PageFlowLayoutWidth),
}

impl PageFlowNodeExtentAuthority {
    fn is_exact_at(self, width: PageFlowLayoutWidth) -> bool {
        self == Self::ExactAt(width)
    }
}

impl PageFlowSurfaceMetrics {
    pub(super) fn new(
        projection: &PageFlowProjectionMetrics,
        widths: HashMap<PageFlowSequenceId, PageFlowSequenceWidth>,
        measurements: &mut PageFlowMeasurementCache,
    ) -> Self {
        hydrate_surface_metrics(projection, widths, measurements)
    }

    pub(super) fn reconciliation_widths(
        &self,
    ) -> HashMap<PageFlowSequenceId, PageFlowSequenceWidth> {
        self.sequences
            .iter()
            .filter_map(|(id, sequence)| {
                sequence
                    .width
                    .map(|width| (id.clone(), width.into_provisional()))
            })
            .collect()
    }

    pub(super) fn set_pins(&mut self, pins: PageFlowPinnedNodes) {
        self.pins = pins;
    }

    pub(super) fn observe_root_sequence_exact_width(&mut self, width: PageFlowExactLayoutWidth) {
        let root = self
            .sequences
            .get_mut(&PageFlowSequenceId::Root)
            .expect("Notion flow metrics must retain their root sequence");
        let changed = root
            .width
            .is_some_and(|previous| previous.layout_width() != width.layout_width());
        if changed {
            root.authorities
                .fill(PageFlowNodeExtentAuthority::Provisional);
        }
        root.width = Some(PageFlowSequenceWidth::Exact(width));
    }

    pub(super) fn plan_sequence(
        &mut self,
        mut context: PageFlowPlanningContext<'_>,
        sequence_id: &PageFlowSequenceId,
        width: PageFlowSequenceWidth,
        viewport: PageFlowViewport,
    ) -> PageFlowSequencePlan {
        let sequence = self
            .sequences
            .get_mut(sequence_id)
            .expect("planned Notion flow sequence must be indexed");
        let width = width.with_retained_exact(sequence.width);
        sequence.width = Some(width);
        let mut windows = self.sequence_windows(context.projection, sequence_id, viewport);
        for _ in 0..PAGE_FLOW_WIDTH_REFRESH_PASSES {
            self.refresh_windows(&mut context, sequence_id, width, &windows);
            windows = self.sequence_windows(context.projection, sequence_id, viewport);
        }
        let sequence = &self.sequences[sequence_id];
        PageFlowSequencePlan {
            spans: render_spans(&sequence.extents, &windows),
            total_extent: sequence.extents.total(),
        }
    }

    fn sequence_windows(
        &self,
        projection: &PageFlowProjectionMetrics,
        sequence_id: &PageFlowSequenceId,
        viewport: PageFlowViewport,
    ) -> Vec<Range<usize>> {
        let sequence = self
            .sequences
            .get(sequence_id)
            .expect("planned Notion flow sequence must be indexed");
        let visible = visible_node_range(&sequence.extents, viewport);
        render_windows(projection, sequence_id, visible, &self.pins)
    }

    fn refresh_windows(
        &mut self,
        context: &mut PageFlowPlanningContext<'_>,
        sequence_id: &PageFlowSequenceId,
        width: PageFlowSequenceWidth,
        windows: &[Range<usize>],
    ) {
        let Some(exact) = width.exact_width() else {
            self.mark_windows_provisional(sequence_id, windows);
            return;
        };
        let previous_total = self.sequences[sequence_id].extents.total();
        for index in windows.iter().flat_map(|window| window.clone()) {
            self.refresh_node(context, sequence_id, exact, index);
        }
        if previous_total != self.sequences[sequence_id].extents.total()
            && context
                .projection
                .sequence_parents
                .contains_key(sequence_id)
        {
            self.propagate_nested_total(context.projection, context.journal, sequence_id);
        }
    }

    fn mark_windows_provisional(
        &mut self,
        sequence_id: &PageFlowSequenceId,
        windows: &[Range<usize>],
    ) {
        let sequence = self
            .sequences
            .get_mut(sequence_id)
            .expect("provisional Notion flow sequence must be indexed");
        for index in windows.iter().flat_map(|window| window.clone()) {
            sequence.authorities[index] = PageFlowNodeExtentAuthority::Provisional;
        }
    }

    fn refresh_node(
        &mut self,
        context: &mut PageFlowPlanningContext<'_>,
        sequence_id: &PageFlowSequenceId,
        width: PageFlowExactLayoutWidth,
        index: usize,
    ) {
        let projection = context.projection;
        let measurements = &mut *context.measurements;
        let journal = &mut *context.journal;
        if self.sequences[sequence_id].authorities[index].is_exact_at(width.layout_width()) {
            return;
        }
        let node = &projection.sequences[sequence_id].nodes[index];
        let Some(witness) = node.section_witness.as_ref() else {
            return;
        };
        let cached = measurements.exact_extent(
            &PageFlowMeasurementKey {
                node_id: node.id.clone(),
                width,
            },
            witness,
        );
        let sequence = self
            .sequences
            .get_mut(sequence_id)
            .expect("refreshed Notion flow sequence must be indexed");
        let Some(extent) = cached else {
            sequence.authorities[index] = PageFlowNodeExtentAuthority::Provisional;
            return;
        };
        let previous = sequence.extents.value(index);
        let change = PageFlowExtentChange::between(previous, extent);
        if !change.admits_exact_observation() {
            return;
        }
        sequence.authorities[index] = PageFlowNodeExtentAuthority::ExactAt(width.layout_width());
        if !sequence.extents.set(index, extent).is_material() {
            return;
        }
        journal.record(
            node.id.clone(),
            root_outer_index_for_node(projection, &node.id),
            previous,
            extent,
        );
    }

    fn propagate_nested_total(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        journal: &mut PageFlowPlanningMutationJournal,
        source_sequence_id: &PageFlowSequenceId,
    ) {
        let mut sequence_id = source_sequence_id.clone();
        loop {
            let parent = projection.sequence_parents[&sequence_id].clone();
            let sequence_total = self.sequences[&sequence_id].extents.total();
            let maxima = self
                .column_maxima
                .get_mut(&parent.columns_node_id)
                .expect("Notion Columns max-lane index must exist");
            if !maxima.set(parent.lane_index, sequence_total).is_material() {
                return;
            }
            if !self.update_parent_columns(projection, journal, &parent.columns_node_id) {
                return;
            }
            if !projection
                .sequence_parents
                .contains_key(&parent.sequence_id)
            {
                return;
            }
            sequence_id = parent.sequence_id;
        }
    }

    fn update_parent_columns(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        journal: &mut PageFlowPlanningMutationJournal,
        node_id: &PageFlowNodeId,
    ) -> bool {
        let location = &projection.node_locations[node_id];
        let next_extent = self
            .columns_extent(projection, node_id)
            .expect("parent Notion Columns extent must resolve");
        let parent = self
            .sequences
            .get_mut(&location.sequence_id)
            .expect("parent Notion flow sequence must be indexed");
        parent.authorities[location.index] = PageFlowNodeExtentAuthority::Provisional;
        let previous = parent.extents.value(location.index);
        if !parent
            .extents
            .set(location.index, next_extent)
            .is_material()
        {
            return false;
        }
        journal.record(
            node_id.clone(),
            root_outer_index_for_node(projection, node_id),
            previous,
            next_extent,
        );
        true
    }

    pub(super) fn columns_extent(
        &self,
        projection: &PageFlowProjectionMetrics,
        node_id: &PageFlowNodeId,
    ) -> Option<f32> {
        let lane_extent = self.column_maxima.get(node_id)?.max();
        let location = projection.node_locations.get(node_id)?;
        let template = projection.sequences.get(&location.sequence_id)?;
        let envelope = template.nodes[location.index].columns_envelope?;
        Some(envelope.apply(lane_extent))
    }

    pub(super) fn node_outer_local_y(
        &self,
        projection: &PageFlowProjectionMetrics,
        node_id: &PageFlowNodeId,
    ) -> Option<f32> {
        let location = projection.node_locations.get(node_id)?;
        if location.sequence_id == PageFlowSequenceId::Root {
            return Some(0.0);
        }
        let mut sequence_id = location.sequence_id.clone();
        let mut offset = self
            .sequences
            .get(&sequence_id)?
            .extents
            .prefix(location.index);
        loop {
            let parent = projection.sequence_parents.get(&sequence_id)?;
            let parent_location = projection.node_locations.get(&parent.columns_node_id)?;
            let template = projection.sequences.get(&parent_location.sequence_id)?;
            offset += template.nodes[parent_location.index].child_block_offset?;
            if parent_location.sequence_id == PageFlowSequenceId::Root {
                return Some(offset);
            }
            offset += self
                .sequences
                .get(&parent_location.sequence_id)?
                .extents
                .prefix(parent_location.index);
            sequence_id.clone_from(&parent_location.sequence_id);
        }
    }
}

pub(super) fn section_witness(
    projection: &PageFlowProjectionMetrics,
    node_id: &PageFlowNodeId,
) -> Option<PageFlowSectionLayoutWitness> {
    let location = projection.node_locations.get(node_id)?;
    projection.sequences[&location.sequence_id].nodes[location.index]
        .section_witness
        .clone()
}
