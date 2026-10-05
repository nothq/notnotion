use std::{collections::HashMap, ops::Range, sync::Arc};

use crate::ui::{
    CardPageBlockContent, CardPageBlockKind, CardPageStructuralBlock, LoadedCardPageData,
    LoadedCardPageDocumentUnit, PageDocumentUnitLayoutRevision, PageFlowColumns,
    PageFlowDecoratorPlanRevision, PageFlowExtentEnvelope, PageFlowNode, PageFlowNodeId,
    PageFlowSection, PageFlowSequence, PageFlowSequenceId,
};

mod layout_authority;

use layout_authority::PageFlowRootLayoutAuthoritySnapshot;

use super::PageFlowSurfaceKey;
use crate::ui::board_workspace::{PageDocumentColumn, PageDocumentLayout};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageFlowSectionLayoutWitness {
    pub(super) unit_keys: Arc<[crate::ui::PageDocumentUnitKey]>,
    pub(super) unit_revisions: Arc<[PageDocumentUnitLayoutRevision]>,
    pub(super) decorator_revision: PageFlowDecoratorPlanRevision,
}

impl PageFlowSectionLayoutWitness {
    pub(super) fn contains_unit(&self, unit_key: &crate::ui::PageDocumentUnitKey) -> bool {
        self.unit_keys.iter().any(|key| key == unit_key)
    }
}

#[derive(Clone)]
pub(super) struct PageFlowNodeMetric {
    pub(super) id: PageFlowNodeId,
    pub(super) initial_extent: f32,
    pub(super) section_witness: Option<PageFlowSectionLayoutWitness>,
    pub(super) columns_envelope: Option<PageFlowExtentEnvelope>,
    pub(super) content_block_offset: f32,
    pub(super) child_block_offset: Option<f32>,
}

pub(super) struct PageFlowSequenceTemplate {
    pub(super) nodes: Arc<[PageFlowNodeMetric]>,
}

#[derive(Clone)]
pub(super) struct PageFlowNodeMetricLocation {
    pub(super) sequence_id: PageFlowSequenceId,
    pub(super) index: usize,
}

#[derive(Clone)]
pub(super) struct PageFlowSequenceParent {
    pub(super) sequence_id: PageFlowSequenceId,
    pub(super) columns_node_id: PageFlowNodeId,
    pub(super) lane_index: usize,
}

pub(super) struct PageFlowProjectionMetrics {
    pub(super) sequences: HashMap<PageFlowSequenceId, PageFlowSequenceTemplate>,
    pub(super) node_locations: HashMap<PageFlowNodeId, PageFlowNodeMetricLocation>,
    pub(super) sequence_parents: HashMap<PageFlowSequenceId, PageFlowSequenceParent>,
    pub(super) sequence_depths: HashMap<PageFlowSequenceId, usize>,
    pub(super) column_sequences: HashMap<PageFlowNodeId, Arc<[PageFlowSequenceId]>>,
    pub(super) columns_bottom_up: Arc<[PageFlowNodeId]>,
    pub(super) estimate: PageFlowUnitEstimate,
    root_layout_authority: PageFlowRootLayoutAuthoritySnapshot,
}

/// What a page's unmeasured blocks are estimated against: the widest its
/// column grows on the surface showing it, if it has a limit.
#[derive(Clone, Copy)]
pub(super) struct PageFlowUnitEstimate {
    column_maximum_width: Option<f32>,
}

impl PageFlowUnitEstimate {
    fn new(data: &LoadedCardPageData, surface: PageFlowSurfaceKey) -> Self {
        Self {
            column_maximum_width: PageDocumentColumn::maximum_width_for(
                PageDocumentLayout::for_flow_surface(surface),
                data.page.format,
            ),
        }
    }
}

impl PageFlowProjectionMetrics {
    pub(super) fn new(data: &LoadedCardPageData, surface: PageFlowSurfaceKey) -> Self {
        let root_layout_authority = PageFlowRootLayoutAuthoritySnapshot::new(data);
        let estimate = PageFlowUnitEstimate::new(data, surface);
        let mut builder = PageFlowMetricsBuilder {
            data,
            estimate,
            sequences: HashMap::new(),
            node_locations: HashMap::new(),
            sequence_parents: HashMap::new(),
            sequence_depths: HashMap::new(),
            column_sequences: HashMap::new(),
            columns_bottom_up: Vec::new(),
        };
        builder.build_sequence(&data.flow.root, None, 0);
        Self {
            sequences: builder.sequences,
            node_locations: builder.node_locations,
            sequence_parents: builder.sequence_parents,
            sequence_depths: builder.sequence_depths,
            column_sequences: builder.column_sequences,
            columns_bottom_up: builder.columns_bottom_up.into(),
            estimate,
            root_layout_authority,
        }
    }

    pub(super) fn root_index(&self, node_id: &PageFlowNodeId) -> Option<usize> {
        let location = self.node_locations.get(node_id)?;
        (location.sequence_id == PageFlowSequenceId::Root).then_some(location.index)
    }

    pub(super) fn changed_root_ids(&self, data: &LoadedCardPageData) -> Vec<PageFlowNodeId> {
        self.root_layout_authority.changed_root_ids(data)
    }
}

struct PageFlowMetricsBuilder<'a> {
    data: &'a LoadedCardPageData,
    estimate: PageFlowUnitEstimate,
    sequences: HashMap<PageFlowSequenceId, PageFlowSequenceTemplate>,
    node_locations: HashMap<PageFlowNodeId, PageFlowNodeMetricLocation>,
    sequence_parents: HashMap<PageFlowSequenceId, PageFlowSequenceParent>,
    sequence_depths: HashMap<PageFlowSequenceId, usize>,
    column_sequences: HashMap<PageFlowNodeId, Arc<[PageFlowSequenceId]>>,
    columns_bottom_up: Vec<PageFlowNodeId>,
}

impl PageFlowMetricsBuilder<'_> {
    fn build_sequence(
        &mut self,
        sequence: &PageFlowSequence,
        parent: Option<PageFlowSequenceParent>,
        depth: usize,
    ) -> f32 {
        let sequence_id = sequence.sequence_id();
        assert!(
            self.sequence_depths
                .insert(sequence_id.clone(), depth)
                .is_none(),
            "Notion flow sequence IDs must be unique"
        );
        if let Some(parent) = parent {
            assert!(
                self.sequence_parents
                    .insert(sequence_id.clone(), parent)
                    .is_none(),
                "Notion flow sequence IDs must be unique"
            );
        }
        let mut metrics = Vec::with_capacity(sequence.nodes.len());
        for (index, node) in sequence.nodes.iter().enumerate() {
            let metric = self.build_node(node, &sequence_id);
            assert!(
                self.node_locations
                    .insert(
                        metric.id.clone(),
                        PageFlowNodeMetricLocation {
                            sequence_id: sequence_id.clone(),
                            index,
                        },
                    )
                    .is_none(),
                "Notion flow node IDs must be unique"
            );
            metrics.push(metric);
        }
        let total = metrics.iter().map(|metric| metric.initial_extent).sum();
        assert!(
            self.sequences
                .insert(
                    sequence_id,
                    PageFlowSequenceTemplate {
                        nodes: metrics.into(),
                    },
                )
                .is_none(),
            "Notion flow sequence IDs must be unique"
        );
        total
    }

    fn build_node(
        &mut self,
        node: &PageFlowNode,
        parent_sequence_id: &PageFlowSequenceId,
    ) -> PageFlowNodeMetric {
        match node {
            PageFlowNode::Section(section) => self.build_section_metric(section),
            PageFlowNode::Columns(columns) => {
                self.build_columns_metric(columns, parent_sequence_id)
            }
        }
    }

    fn build_section_metric(&self, section: &PageFlowSection) -> PageFlowNodeMetric {
        PageFlowNodeMetric {
            id: section.key.node_id(),
            initial_extent: section.decorator_plan.envelope.apply(estimate_section(
                self.data,
                section.document_unit_range.clone(),
                self.estimate,
            )),
            section_witness: Some(PageFlowSectionLayoutWitness {
                unit_keys: self.data.document_units[section.document_unit_range.clone()]
                    .iter()
                    .map(|unit| unit.key().clone())
                    .collect::<Vec<_>>()
                    .into(),
                unit_revisions: self.data.document_unit_layout_revisions
                    [section.document_unit_range.clone()]
                .into(),
                decorator_revision: section.decorator_plan.revision.clone(),
            }),
            columns_envelope: None,
            content_block_offset: section.decorator_plan.content_origin.block_offset(),
            child_block_offset: None,
        }
    }

    fn build_columns_metric(
        &mut self,
        columns: &PageFlowColumns,
        parent_sequence_id: &PageFlowSequenceId,
    ) -> PageFlowNodeMetric {
        let node_id = columns.key.node_id();
        let mut child_ids = Vec::with_capacity(columns.columns.len());
        let mut lane_extent = 0.0_f32;
        for (lane_index, column) in columns.columns.iter().enumerate() {
            let child_id = column.sequence.sequence_id();
            lane_extent = lane_extent.max(self.build_sequence(
                &column.sequence,
                Some(PageFlowSequenceParent {
                    sequence_id: parent_sequence_id.clone(),
                    columns_node_id: node_id.clone(),
                    lane_index,
                }),
                self.sequence_depths[parent_sequence_id] + 1,
            ));
            child_ids.push(child_id);
        }
        assert!(
            self.column_sequences
                .insert(node_id.clone(), child_ids.into())
                .is_none(),
            "Notion Columns node IDs must be unique"
        );
        self.columns_bottom_up.push(node_id.clone());
        let envelope = columns.extent_envelope();
        PageFlowNodeMetric {
            id: node_id,
            initial_extent: envelope.apply(lane_extent),
            section_witness: None,
            columns_envelope: Some(envelope),
            content_block_offset: columns.decorator_plan.content_origin.block_offset(),
            child_block_offset: Some(
                columns.decorator_plan.content_origin.block_offset()
                    + crate::ui::PageFlowColumnsPresentationSpec::NOTION.top_inset,
            ),
        }
    }
}

fn estimate_section(
    data: &LoadedCardPageData,
    range: Range<usize>,
    estimate: PageFlowUnitEstimate,
) -> f32 {
    range
        .map(|index| estimate_document_unit(data, index, estimate))
        .sum()
}

pub(super) fn estimate_document_unit(
    data: &LoadedCardPageData,
    index: usize,
    estimate: PageFlowUnitEstimate,
) -> f32 {
    let unit = &data.document_units[index];
    if matches!(unit, LoadedCardPageDocumentUnit::SimpleTableRow { .. }) {
        return 35.0;
    }
    let row = &data.visible_rows[unit.owner_visible_row_index()];
    let block = &data.page.blocks[row.block_index];
    match &block.content {
        CardPageBlockContent::Editable(editable) => match editable.kind {
            CardPageBlockKind::Text
            | CardPageBlockKind::BulletedList
            | CardPageBlockKind::NumberedList
            | CardPageBlockKind::ToDoList
            | CardPageBlockKind::ToggleList => 36.0,
            CardPageBlockKind::SubHeader => 49.0,
            CardPageBlockKind::SubSubHeader => 43.0,
            CardPageBlockKind::Heading3 => 39.0,
            CardPageBlockKind::Heading4 => 35.0,
            CardPageBlockKind::PageLink => 30.0,
            CardPageBlockKind::Callout => 40.0,
            CardPageBlockKind::Quote => 40.0,
            CardPageBlockKind::Code => 64.0,
        },
        CardPageBlockContent::Structural(CardPageStructuralBlock::Divider) => 13.0,
        CardPageBlockContent::Structural(
            CardPageStructuralBlock::CollectionView { .. }
            | CardPageStructuralBlock::CollectionViewPage { .. },
        ) => 74.0,
        CardPageBlockContent::Resource(resource) => {
            resource.image().size_hint().map_or(49.0, |hint| {
                estimated_image_height(hint.width(), hint.height(), estimate)
            })
        }
        CardPageBlockContent::Alias(_) => 36.0,
        CardPageBlockContent::UnsupportedLeaf(_) => 48.0,
        CardPageBlockContent::OpaqueUnavailable { .. }
        | CardPageBlockContent::Layout(_)
        | CardPageBlockContent::Table { .. }
        | CardPageBlockContent::TableRow { .. } => {
            panic!("non-render-unit Notion block escaped into a flow Section")
        }
    }
}

fn estimated_image_height(width: u32, height: u32, estimate: PageFlowUnitEstimate) -> f32 {
    let rendered_width = estimate
        .column_maximum_width
        .map_or(width as f32, |maximum| (width as f32).min(maximum));
    (rendered_width * height as f32 / width as f32).clamp(49.0, 540.0)
}
