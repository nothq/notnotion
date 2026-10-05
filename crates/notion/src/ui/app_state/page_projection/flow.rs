use std::{collections::HashMap, ops::Range, sync::Arc};

use crate::model::{CardPageColumnEffectiveShare, CardPageColumnRatio, CardPageColumnWeightTotal};

use super::PageDocumentUnitKey;

mod builder;
mod callout;

pub(super) use builder::build_page_flow_projection;
pub(crate) use callout::{
    PageFlowCalloutLayoutRevision, PageFlowCalloutPath, PageFlowCalloutPresentationSpec,
    PageFlowCalloutSegment, PageFlowColumnsPresentationSpec, PageFlowDecoratorLayer,
    PageFlowDecoratorPlan, PageFlowDecoratorPlanRevision, PageFlowExtentEnvelope,
    PageFlowPaintedCalloutPrefix, PAGE_FLOW_BLOCK_INDENT,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageFlowLaneKey(Arc<PageFlowLaneKeyNode>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageFlowSequenceId {
    Root,
    Column { column_block_id: Arc<str> },
}

#[derive(Debug, PartialEq, Eq)]
enum PageFlowLaneKeyNode {
    Root,
    Column {
        parent: PageFlowLaneKey,
        column_list_block_id: Arc<str>,
        column_block_id: Arc<str>,
    },
}

impl PageFlowLaneKey {
    pub(super) fn root() -> Self {
        Self(Arc::new(PageFlowLaneKeyNode::Root))
    }

    pub(super) fn column(&self, column_list_block_id: Arc<str>, column_block_id: Arc<str>) -> Self {
        Self(Arc::new(PageFlowLaneKeyNode::Column {
            parent: self.clone(),
            column_list_block_id,
            column_block_id,
        }))
    }

    pub(crate) fn parent(&self) -> Option<&Self> {
        match self.0.as_ref() {
            PageFlowLaneKeyNode::Root => None,
            PageFlowLaneKeyNode::Column { parent, .. } => Some(parent),
        }
    }

    pub(crate) fn column_ids(&self) -> Option<(&str, &str)> {
        match self.0.as_ref() {
            PageFlowLaneKeyNode::Root => None,
            PageFlowLaneKeyNode::Column {
                column_list_block_id,
                column_block_id,
                ..
            } => Some((column_list_block_id.as_ref(), column_block_id.as_ref())),
        }
    }

    pub(crate) fn sequence_id(&self) -> PageFlowSequenceId {
        match self.0.as_ref() {
            PageFlowLaneKeyNode::Root => PageFlowSequenceId::Root,
            PageFlowLaneKeyNode::Column {
                column_block_id, ..
            } => PageFlowSequenceId::Column {
                column_block_id: column_block_id.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageFlowNodeKey {
    Section {
        lane: PageFlowLaneKey,
        first_unit: PageDocumentUnitKey,
        last_unit: PageDocumentUnitKey,
    },
    Columns {
        column_list_block_id: Arc<str>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageFlowNodeId {
    Section { first_unit: PageDocumentUnitKey },
    Columns { column_list_block_id: Arc<str> },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageDocumentOuterItemId {
    Lead,
    Flow(PageFlowNodeId),
    Footer,
}

impl PageFlowNodeKey {
    pub(crate) fn node_id(&self) -> PageFlowNodeId {
        match self {
            Self::Section { first_unit, .. } => PageFlowNodeId::Section {
                first_unit: first_unit.clone(),
            },
            Self::Columns {
                column_list_block_id,
            } => PageFlowNodeId::Columns {
                column_list_block_id: column_list_block_id.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageFlowNodePath(Arc<PageFlowNodePathNode>);

#[derive(Debug, PartialEq, Eq)]
struct PageFlowNodePathNode {
    parent: Option<PageFlowNodePath>,
    key: PageFlowNodeKey,
}

impl PageFlowNodePath {
    pub(super) fn push(parent: Option<&Self>, key: PageFlowNodeKey) -> Self {
        Self(Arc::new(PageFlowNodePathNode {
            parent: parent.cloned(),
            key,
        }))
    }

    pub(crate) fn parent(&self) -> Option<&Self> {
        self.0.parent.as_ref()
    }

    pub(crate) fn key(&self) -> &PageFlowNodeKey {
        &self.0.key
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowSection {
    pub(crate) key: PageFlowNodeKey,
    pub(crate) document_unit_range: Range<usize>,
    pub(crate) callout_path: PageFlowCalloutPath,
    pub(crate) decorator_plan: PageFlowDecoratorPlan,
    pub(crate) unit_adjacencies: Arc<[PageFlowUnitAdjacency]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PageFlowUnitAdjacency {
    pub(crate) next_owner_visible_row_index: Option<usize>,
}

impl PageFlowSection {
    pub(crate) const MAX_UNITS: usize = super::super::page_sections::PAGE_SECTION_MAX_UNITS;

    pub(crate) fn painted_callout_prefix(&self) -> PageFlowPaintedCalloutPrefix {
        self.callout_path.painted_prefix()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowColumns {
    pub(crate) key: PageFlowNodeKey,
    pub(crate) column_list_block_index: usize,
    pub(crate) common_callout_path: PageFlowCalloutPath,
    pub(crate) decorator_plan: PageFlowDecoratorPlan,
    pub(crate) weight_total: CardPageColumnWeightTotal,
    pub(crate) resize_authority_complete: bool,
    pub(crate) columns: Arc<[PageColumnFlow]>,
}

#[derive(Clone, Debug)]
pub(crate) struct PageColumnFlow {
    pub(crate) column_block_index: usize,
    pub(crate) column_block_id: Arc<str>,
    pub(crate) lane: PageFlowLaneKey,
    pub(crate) raw_weight: Option<CardPageColumnRatio>,
    pub(crate) layout_weight: CardPageColumnRatio,
    pub(crate) effective_share: CardPageColumnEffectiveShare,
    pub(crate) sequence: PageFlowSequence,
}

#[derive(Clone, Debug)]
pub(crate) enum PageFlowNode {
    Section(PageFlowSection),
    Columns(PageFlowColumns),
}

impl PageFlowNode {
    pub(crate) fn key(&self) -> &PageFlowNodeKey {
        match self {
            Self::Section(section) => &section.key,
            Self::Columns(columns) => &columns.key,
        }
    }

    pub(crate) fn node_id(&self) -> PageFlowNodeId {
        self.key().node_id()
    }

    pub(crate) fn decorator_plan(&self) -> &PageFlowDecoratorPlan {
        match self {
            Self::Section(section) => &section.decorator_plan,
            Self::Columns(columns) => &columns.decorator_plan,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowSequence {
    pub(crate) lane: PageFlowLaneKey,
    pub(crate) common_callout_path: PageFlowCalloutPath,
    pub(crate) nodes: Arc<[PageFlowNode]>,
}

impl PageFlowSequence {
    pub(crate) fn sequence_id(&self) -> PageFlowSequenceId {
        self.lane.sequence_id()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageFlowOuterItem {
    pub(crate) index: usize,
    pub(crate) key: PageFlowNodeKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PageFlowLocation {
    pub(crate) document_unit_index: usize,
    pub(crate) outer_item: PageFlowOuterItem,
    pub(crate) node_path: PageFlowNodePath,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowProjection {
    pub(crate) root: PageFlowSequence,
    unit_locations: HashMap<PageDocumentUnitKey, PageFlowLocation>,
    node_paths: HashMap<PageFlowNodeId, PageFlowNodePath>,
    node_outer_items: HashMap<PageFlowNodeId, PageFlowOuterItem>,
}

impl PageFlowProjection {
    pub(crate) fn location(&self, key: &PageDocumentUnitKey) -> Option<&PageFlowLocation> {
        self.unit_locations.get(key)
    }

    pub(crate) fn node_path(&self, id: &PageFlowNodeId) -> Option<&PageFlowNodePath> {
        self.node_paths.get(id)
    }

    pub(crate) fn outer_item_for_node(&self, id: &PageFlowNodeId) -> Option<&PageFlowOuterItem> {
        self.node_outer_items.get(id)
    }

    pub(super) fn has_same_authoritative_flow_semantics(&self, other: &Self) -> bool {
        page_flow_sequences_match(&self.root, &other.root)
    }
}

fn page_flow_sequences_match(left: &PageFlowSequence, right: &PageFlowSequence) -> bool {
    left.nodes.len() == right.nodes.len()
        && left
            .nodes
            .iter()
            .zip(right.nodes.iter())
            .all(|(left, right)| page_flow_nodes_match(left, right))
}

fn page_flow_nodes_match(left: &PageFlowNode, right: &PageFlowNode) -> bool {
    match (left, right) {
        (PageFlowNode::Section(left), PageFlowNode::Section(right)) => {
            page_flow_sections_match(left, right)
        }
        (PageFlowNode::Columns(left), PageFlowNode::Columns(right)) => {
            page_flow_columns_match(left, right)
        }
        (PageFlowNode::Section(_), PageFlowNode::Columns(_))
        | (PageFlowNode::Columns(_), PageFlowNode::Section(_)) => false,
    }
}

fn page_flow_sections_match(left: &PageFlowSection, right: &PageFlowSection) -> bool {
    left.document_unit_range == right.document_unit_range
        && section_unit_keys_match(&left.key, &right.key)
        && left.decorator_plan.revision == right.decorator_plan.revision
        && left.unit_adjacencies == right.unit_adjacencies
}

fn section_unit_keys_match(left: &PageFlowNodeKey, right: &PageFlowNodeKey) -> bool {
    match (left, right) {
        (
            PageFlowNodeKey::Section {
                first_unit: left_first,
                last_unit: left_last,
                ..
            },
            PageFlowNodeKey::Section {
                first_unit: right_first,
                last_unit: right_last,
                ..
            },
        ) => left_first == right_first && left_last == right_last,
        (PageFlowNodeKey::Section { .. }, PageFlowNodeKey::Columns { .. })
        | (PageFlowNodeKey::Columns { .. }, PageFlowNodeKey::Section { .. })
        | (PageFlowNodeKey::Columns { .. }, PageFlowNodeKey::Columns { .. }) => false,
    }
}

fn page_flow_columns_match(left: &PageFlowColumns, right: &PageFlowColumns) -> bool {
    column_list_keys_match(&left.key, &right.key)
        && left.column_list_block_index == right.column_list_block_index
        && left.decorator_plan.revision == right.decorator_plan.revision
        && left.weight_total == right.weight_total
        && left.resize_authority_complete == right.resize_authority_complete
        && left.columns.len() == right.columns.len()
        && left
            .columns
            .iter()
            .zip(right.columns.iter())
            .all(|(left, right)| page_column_flows_match(left, right))
}

fn column_list_keys_match(left: &PageFlowNodeKey, right: &PageFlowNodeKey) -> bool {
    match (left, right) {
        (
            PageFlowNodeKey::Columns {
                column_list_block_id: left_id,
            },
            PageFlowNodeKey::Columns {
                column_list_block_id: right_id,
            },
        ) => left_id == right_id,
        (PageFlowNodeKey::Columns { .. }, PageFlowNodeKey::Section { .. })
        | (PageFlowNodeKey::Section { .. }, PageFlowNodeKey::Columns { .. })
        | (PageFlowNodeKey::Section { .. }, PageFlowNodeKey::Section { .. }) => false,
    }
}

fn page_column_flows_match(left: &PageColumnFlow, right: &PageColumnFlow) -> bool {
    left.column_block_index == right.column_block_index
        && left.column_block_id == right.column_block_id
        && left.raw_weight == right.raw_weight
        && left.layout_weight == right.layout_weight
        && left.effective_share == right.effective_share
        && page_flow_sequences_match(&left.sequence, &right.sequence)
}
