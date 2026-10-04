use std::collections::HashMap;

use crate::ui::{PageFlowNodeId, PageFlowSequenceId};

use super::super::{
    cache::{PageFlowMeasurementCache, PageFlowMeasurementKey},
    metrics::PageFlowProjectionMetrics,
    prefix::{PageFlowExtentChange, PageFlowMaxExtents, PageFlowPrefixExtents},
    PageFlowSequenceWidth,
};
use super::{PageFlowNodeExtentAuthority, PageFlowSequenceMetrics, PageFlowSurfaceMetrics};

struct PageFlowPreparedSequence {
    width: Option<PageFlowSequenceWidth>,
    values: Vec<f32>,
    authorities: Vec<PageFlowNodeExtentAuthority>,
}

pub(super) fn hydrate_surface_metrics(
    projection: &PageFlowProjectionMetrics,
    widths: HashMap<PageFlowSequenceId, PageFlowSequenceWidth>,
    measurements: &mut PageFlowMeasurementCache,
) -> PageFlowSurfaceMetrics {
    let (mut prepared, mut totals) = prepare_sequences(projection, &widths, measurements);
    let column_maxima = restore_columns_bottom_up(projection, &mut prepared, &mut totals);
    let sequences = prepared
        .into_iter()
        .map(|(sequence_id, sequence)| {
            (
                sequence_id,
                PageFlowSequenceMetrics {
                    width: sequence.width,
                    extents: PageFlowPrefixExtents::new(sequence.values),
                    authorities: sequence.authorities,
                },
            )
        })
        .collect();
    PageFlowSurfaceMetrics {
        sequences,
        column_maxima,
        pins: Default::default(),
    }
}

/// Each sequence's prepared extents, and each sequence's total extent.
type PreparedSequences = (
    HashMap<PageFlowSequenceId, PageFlowPreparedSequence>,
    HashMap<PageFlowSequenceId, f32>,
);

fn prepare_sequences(
    projection: &PageFlowProjectionMetrics,
    widths: &HashMap<PageFlowSequenceId, PageFlowSequenceWidth>,
    measurements: &mut PageFlowMeasurementCache,
) -> PreparedSequences {
    let mut prepared = HashMap::with_capacity(projection.sequences.len());
    let mut totals = HashMap::with_capacity(projection.sequences.len());
    for (sequence_id, template) in &projection.sequences {
        let width = widths.get(sequence_id).copied();
        let mut values = Vec::with_capacity(template.nodes.len());
        let mut authorities = Vec::with_capacity(template.nodes.len());
        for node in template.nodes.iter() {
            let cached = width
                .and_then(PageFlowSequenceWidth::exact_width)
                .zip(node.section_witness.as_ref())
                .and_then(|(width, witness)| {
                    measurements
                        .exact_extent(
                            &PageFlowMeasurementKey {
                                node_id: node.id.clone(),
                                width,
                            },
                            witness,
                        )
                        .map(|extent| (width.layout_width(), extent))
                });
            let (value, authority) = cached
                .filter(|(_, extent)| {
                    PageFlowExtentChange::between(node.initial_extent, *extent)
                        .admits_exact_observation()
                })
                .map_or(
                    (
                        node.initial_extent,
                        PageFlowNodeExtentAuthority::Provisional,
                    ),
                    |(width, extent)| (extent, PageFlowNodeExtentAuthority::ExactAt(width)),
                );
            values.push(value);
            authorities.push(authority);
        }
        totals.insert(sequence_id.clone(), values.iter().sum());
        prepared.insert(
            sequence_id.clone(),
            PageFlowPreparedSequence {
                width,
                values,
                authorities,
            },
        );
    }
    (prepared, totals)
}

fn restore_columns_bottom_up(
    projection: &PageFlowProjectionMetrics,
    prepared: &mut HashMap<PageFlowSequenceId, PageFlowPreparedSequence>,
    totals: &mut HashMap<PageFlowSequenceId, f32>,
) -> HashMap<PageFlowNodeId, PageFlowMaxExtents> {
    let mut maxima = HashMap::with_capacity(projection.column_sequences.len());
    for node_id in projection.columns_bottom_up.iter() {
        let child_ids = &projection.column_sequences[node_id];
        let lane_totals = child_ids.iter().map(|sequence_id| totals[sequence_id]);
        let lane_maxima = PageFlowMaxExtents::new(lane_totals);
        let location = &projection.node_locations[node_id];
        let template = &projection.sequences[&location.sequence_id].nodes[location.index];
        let restored = template
            .columns_envelope
            .expect("Notion Columns metric must carry its layout envelope")
            .apply(lane_maxima.max());
        let parent = prepared
            .get_mut(&location.sequence_id)
            .expect("Notion Columns parent sequence must be prepared");
        let previous = std::mem::replace(&mut parent.values[location.index], restored);
        *totals
            .get_mut(&location.sequence_id)
            .expect("Notion Columns parent total must be prepared") += restored - previous;
        maxima.insert(node_id.clone(), lane_maxima);
    }
    maxima
}
