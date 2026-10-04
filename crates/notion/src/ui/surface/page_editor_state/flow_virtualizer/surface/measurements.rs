use std::collections::{BTreeMap, HashSet};

use crate::ui::{PageFlowNodeId, PageFlowSequenceId};

use super::super::{
    cache::PageFlowMeasurementKey, frame::PageFlowPlanningMutationJournal,
    measurement_batch::PageFlowMeasuredExtent, metrics::PageFlowProjectionMetrics,
    prefix::PageFlowExtentChange,
};
use super::{PageFlowNodeExtentAuthority, PageFlowSurfaceMetrics};

impl PageFlowSurfaceMetrics {
    pub(in crate::ui::surface::page_editor_state::flow_virtualizer) fn invalidate_section_node(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        node_id: &PageFlowNodeId,
    ) -> usize {
        let location = projection
            .node_locations
            .get(node_id)
            .expect("invalidated Notion flow node must remain indexed");
        let template = &projection.sequences[&location.sequence_id].nodes[location.index];
        assert!(template.section_witness.is_some());
        let sequence = self
            .sequences
            .get_mut(&location.sequence_id)
            .expect("invalidated Notion flow sequence must remain indexed");
        sequence.authorities[location.index] = PageFlowNodeExtentAuthority::Provisional;
        let previous = sequence.extents.value(location.index);
        let mut journal = PageFlowPlanningMutationJournal::default();
        if sequence
            .extents
            .set(location.index, template.initial_extent)
            .is_material()
        {
            journal.record(
                node_id.clone(),
                root_outer_index_for_node(projection, node_id),
                previous,
                template.initial_extent,
            );
            let mut pending = BTreeMap::new();
            pending
                .entry(projection.sequence_depths[&location.sequence_id])
                .or_insert_with(HashSet::new)
                .insert(location.sequence_id.clone());
            self.propagate_changed_sequences(projection, &mut journal, pending);
        }
        root_outer_index_for_node(projection, node_id)
    }

    pub(in crate::ui::surface::page_editor_state::flow_virtualizer) fn apply_measurement_entries(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        entries: &[(PageFlowMeasurementKey, PageFlowMeasuredExtent)],
        journal: &mut PageFlowPlanningMutationJournal,
    ) {
        let mut pending = BTreeMap::<usize, HashSet<PageFlowSequenceId>>::new();
        for entry in entries {
            self.apply_leaf_measurement(projection, entry, journal, &mut pending);
        }
        self.propagate_changed_sequences(projection, journal, pending);
    }

    fn apply_leaf_measurement(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        entry: &(PageFlowMeasurementKey, PageFlowMeasuredExtent),
        journal: &mut PageFlowPlanningMutationJournal,
        pending: &mut BTreeMap<usize, HashSet<PageFlowSequenceId>>,
    ) {
        let (key, measured) = entry;
        let measured = *measured;
        let location = projection
            .node_locations
            .get(&key.node_id)
            .expect("measured Notion flow node must remain indexed");
        let sequence = self
            .sequences
            .get_mut(&location.sequence_id)
            .expect("measured Notion flow sequence must remain indexed");
        let exact_width = sequence.width.and_then(|width| width.exact_width());
        if exact_width != Some(key.width) {
            return;
        }
        let previous = sequence.extents.value(location.index);
        let change = PageFlowExtentChange::between(previous, measured.pixels());
        if !change.admits_exact_observation() {
            return;
        }
        sequence.authorities[location.index] =
            PageFlowNodeExtentAuthority::ExactAt(key.width.layout_width());
        if !sequence
            .extents
            .set(location.index, measured.pixels())
            .is_material()
        {
            return;
        }
        journal.record(
            key.node_id.clone(),
            root_outer_index_for_node(projection, &key.node_id),
            previous,
            measured.pixels(),
        );
        pending
            .entry(projection.sequence_depths[&location.sequence_id])
            .or_default()
            .insert(location.sequence_id.clone());
    }

    fn propagate_changed_sequences(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        journal: &mut PageFlowPlanningMutationJournal,
        mut pending: BTreeMap<usize, HashSet<PageFlowSequenceId>>,
    ) {
        while let Some((depth, sequence_ids)) = pending.pop_last() {
            let changed_columns = self.update_changed_lanes(projection, depth, sequence_ids);
            for node_id in changed_columns {
                self.update_columns_node(projection, &node_id, journal, &mut pending);
            }
        }
    }

    fn update_changed_lanes(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        depth: usize,
        sequence_ids: HashSet<PageFlowSequenceId>,
    ) -> HashSet<PageFlowNodeId> {
        let mut changed_columns = HashSet::new();
        for sequence_id in sequence_ids {
            assert_eq!(projection.sequence_depths[&sequence_id], depth);
            let Some(parent) = projection.sequence_parents.get(&sequence_id) else {
                continue;
            };
            let total = self.sequences[&sequence_id].extents.total();
            let maxima = self
                .column_maxima
                .get_mut(&parent.columns_node_id)
                .expect("changed Notion Columns max-lane index must exist");
            if maxima.set(parent.lane_index, total).is_material() {
                changed_columns.insert(parent.columns_node_id.clone());
            }
        }
        changed_columns
    }

    fn update_columns_node(
        &mut self,
        projection: &PageFlowProjectionMetrics,
        node_id: &PageFlowNodeId,
        journal: &mut PageFlowPlanningMutationJournal,
        pending: &mut BTreeMap<usize, HashSet<PageFlowSequenceId>>,
    ) {
        let location = &projection.node_locations[node_id];
        let next_extent = self
            .columns_extent(projection, node_id)
            .expect("changed Notion Columns extent must resolve");
        let parent = self
            .sequences
            .get_mut(&location.sequence_id)
            .expect("changed Notion Columns parent sequence must exist");
        parent.authorities[location.index] = PageFlowNodeExtentAuthority::Provisional;
        let previous = parent.extents.value(location.index);
        if !parent
            .extents
            .set(location.index, next_extent)
            .is_material()
        {
            return;
        }
        journal.record(
            node_id.clone(),
            root_outer_index_for_node(projection, node_id),
            previous,
            next_extent,
        );
        pending
            .entry(projection.sequence_depths[&location.sequence_id])
            .or_default()
            .insert(location.sequence_id.clone());
    }
}

pub(in crate::ui::surface::page_editor_state::flow_virtualizer) fn measurement_admits_cache_update(
    metrics: &PageFlowSurfaceMetrics,
    projection: &PageFlowProjectionMetrics,
    key: &PageFlowMeasurementKey,
    measured: PageFlowMeasuredExtent,
    cached_extent: Option<f32>,
) -> bool {
    let location = &projection.node_locations[&key.node_id];
    let sequence = &metrics.sequences[&location.sequence_id];
    let previous =
        if sequence.width.map(|width| width.layout_width()) == Some(key.width.layout_width()) {
            Some(sequence.extents.value(location.index))
        } else {
            cached_extent
        };
    previous.is_none_or(|previous| {
        PageFlowExtentChange::between(previous, measured.pixels()).admits_exact_observation()
    })
}

pub(super) fn root_outer_index_for_node(
    projection: &PageFlowProjectionMetrics,
    node_id: &PageFlowNodeId,
) -> usize {
    let location = &projection.node_locations[node_id];
    if location.sequence_id == PageFlowSequenceId::Root {
        return location.index;
    }
    root_outer_index_for_sequence(projection, &location.sequence_id)
}

fn root_outer_index_for_sequence(
    projection: &PageFlowProjectionMetrics,
    sequence_id: &PageFlowSequenceId,
) -> usize {
    let mut parent = projection
        .sequence_parents
        .get(sequence_id)
        .expect("nested Notion flow sequence must have a parent");
    loop {
        let location = &projection.node_locations[&parent.columns_node_id];
        let Some(next_parent) = projection.sequence_parents.get(&location.sequence_id) else {
            return location.index;
        };
        parent = next_parent;
    }
}
