use std::{collections::HashMap, ops::Range, sync::Arc};

use crate::{
    model::{CardPageColumnEffectiveShare, CardPageColumnLayoutIndex, CardPageColumnRatio},
    ui::{CardPage, CardPageBlockKind, CardPageLayoutBlock},
};

use super::super::super::page_rows::LoadedCardPageRowProjection;
use super::super::{
    LoadedCardPageDocumentUnit, LoadedCardPageVisibleRow, PageDocumentUnitKey,
    PageVisibleRowLayouts,
};
use super::{
    callout::finalize_decorator_plans, PageFlowCalloutPath, PageFlowLaneKey, PageFlowLocation,
    PageFlowNode, PageFlowNodeId, PageFlowNodePath, PageFlowOuterItem, PageFlowProjection,
    PageFlowSequence,
};

mod columns;
mod hierarchy;
mod sections;

use hierarchy::{build_block_hierarchy, visible_row_indices, BlockHierarchy};

#[derive(Clone)]
struct CalloutContext {
    path: PageFlowCalloutPath,
    tail_visible_row_index: Option<usize>,
    depth: usize,
}

impl CalloutContext {
    fn root() -> Self {
        Self {
            path: PageFlowCalloutPath::default(),
            tail_visible_row_index: None,
            depth: 0,
        }
    }

    fn push(&self, block_id: Arc<str>, visible_row_index: usize) -> Self {
        Self {
            path: self.path.push(block_id, visible_row_index),
            tail_visible_row_index: Some(visible_row_index),
            depth: self.depth + 1,
        }
    }
}

#[derive(Clone)]
struct PendingFlowUnit {
    document_unit_index: usize,
    callout_path: PageFlowCalloutPath,
    callout_tail_visible_row_index: Option<usize>,
}

/// Where a sequence sits in the flow tree: its parent node and the outer item
/// its nodes belong to. Both are absent for the root sequence.
struct SequenceParent {
    node_path: Option<PageFlowNodePath>,
    outer_item: Option<PageFlowOuterItem>,
}

struct SequenceBuild {
    lane: PageFlowLaneKey,
    common_callout_path: PageFlowCalloutPath,
    parent_node_path: Option<PageFlowNodePath>,
    outer_item: Option<PageFlowOuterItem>,
    nodes: Vec<PageFlowNode>,
    pending_units: Vec<PendingFlowUnit>,
}

#[derive(Clone)]
struct ResolvedColumn {
    block_index: usize,
    block_id: Arc<str>,
    raw_weight: Option<CardPageColumnRatio>,
    layout_weight: CardPageColumnRatio,
    effective_share: CardPageColumnEffectiveShare,
}

struct PageFlowBuilder<'a> {
    page: &'a CardPage,
    rows: &'a [LoadedCardPageVisibleRow],
    document_units: &'a [LoadedCardPageDocumentUnit],
    document_unit_ranges: &'a [Range<usize>],
    callout_layer_row_indices: &'a [usize],
    callout_layer_row_ranges: &'a [Range<usize>],
    flow_visible_block_mask: &'a [bool],
    visible_row_layouts: &'a PageVisibleRowLayouts,
    column_layouts: CardPageColumnLayoutIndex<'a>,
    hierarchy: BlockHierarchy,
    visible_row_indices: Vec<Option<usize>>,
    visited_flow_blocks: Vec<bool>,
    assigned_document_units: Vec<bool>,
    unit_locations: HashMap<PageDocumentUnitKey, PageFlowLocation>,
    node_paths: HashMap<PageFlowNodeId, PageFlowNodePath>,
    node_outer_items: HashMap<PageFlowNodeId, PageFlowOuterItem>,
}

pub(in crate::ui::app_state::page_projection) fn build_page_flow_projection(
    page: &CardPage,
    projection: &LoadedCardPageRowProjection,
    visible_row_layouts: &PageVisibleRowLayouts,
) -> PageFlowProjection {
    page.validate_block_hierarchy()
        .unwrap_or_else(|error| panic!("cannot project malformed Notion page flow: {error}"));
    PageFlowBuilder::new(page, projection, visible_row_layouts).build()
}

impl<'a> PageFlowBuilder<'a> {
    fn new(
        page: &'a CardPage,
        projection: &'a LoadedCardPageRowProjection,
        visible_row_layouts: &'a PageVisibleRowLayouts,
    ) -> Self {
        assert_eq!(
            projection.document_unit_ranges.len(),
            projection.visible_rows.len()
        );
        assert_eq!(
            projection.callout_layer_row_ranges.len(),
            projection.visible_rows.len()
        );
        assert_eq!(projection.flow_visible_block_mask.len(), page.blocks.len());
        let visible_row_indices = visible_row_indices(page, &projection.visible_rows);
        Self {
            page,
            rows: &projection.visible_rows,
            document_units: &projection.document_units,
            document_unit_ranges: &projection.document_unit_ranges,
            callout_layer_row_indices: &projection.callout_layer_row_indices,
            callout_layer_row_ranges: &projection.callout_layer_row_ranges,
            flow_visible_block_mask: &projection.flow_visible_block_mask,
            visible_row_layouts,
            column_layouts: page
                .column_layout_index()
                .unwrap_or_else(|error| panic!("cannot index Notion column layouts: {error}")),
            hierarchy: build_block_hierarchy(page),
            visible_row_indices,
            visited_flow_blocks: vec![false; page.blocks.len()],
            assigned_document_units: vec![false; projection.document_units.len()],
            unit_locations: HashMap::with_capacity(projection.document_units.len()),
            node_paths: HashMap::new(),
            node_outer_items: HashMap::new(),
        }
    }

    fn build(mut self) -> PageFlowProjection {
        let root_children = self.hierarchy.root_children.clone();
        let mut root = self.build_sequence(
            &root_children,
            PageFlowLaneKey::root(),
            CalloutContext::root(),
            SequenceParent {
                node_path: None,
                outer_item: None,
            },
        );
        finalize_decorator_plans(&mut root, true, self.rows, self.visible_row_layouts);
        self.assert_exact_document_unit_coverage();
        PageFlowProjection {
            root,
            unit_locations: self.unit_locations,
            node_paths: self.node_paths,
            node_outer_items: self.node_outer_items,
        }
    }

    fn build_sequence(
        &mut self,
        child_indices: &[usize],
        lane: PageFlowLaneKey,
        callout: CalloutContext,
        parent: SequenceParent,
    ) -> PageFlowSequence {
        let SequenceParent {
            node_path: parent_node_path,
            outer_item,
        } = parent;
        let mut build = SequenceBuild {
            lane: lane.clone(),
            common_callout_path: callout.path.clone(),
            parent_node_path,
            outer_item,
            nodes: Vec::new(),
            pending_units: Vec::new(),
        };
        for &block_index in child_indices {
            self.append_block(block_index, &callout, &mut build);
        }
        self.flush_pending_sections(&mut build);
        PageFlowSequence {
            lane,
            common_callout_path: build.common_callout_path,
            nodes: build.nodes.into(),
        }
    }

    fn append_block(
        &mut self,
        block_index: usize,
        inherited_callout: &CalloutContext,
        sequence: &mut SequenceBuild,
    ) {
        if !self.flow_visible_block_mask[block_index] {
            return;
        }
        self.claim_flow_block(block_index);
        let block = &self.page.blocks[block_index];
        match block.layout_content() {
            Some(CardPageLayoutBlock::ColumnList) => {
                self.append_columns(block_index, inherited_callout, sequence)
            }
            Some(CardPageLayoutBlock::Passthrough { .. }) => {
                self.append_children(block_index, inherited_callout, sequence)
            }
            Some(CardPageLayoutBlock::Column { .. }) => panic!(
                "Notion column {} escaped its owning column-list lane",
                block.block_id
            ),
            None => self.append_visible_content(block_index, inherited_callout, sequence),
        }
    }

    fn append_visible_content(
        &mut self,
        block_index: usize,
        inherited_callout: &CalloutContext,
        sequence: &mut SequenceBuild,
    ) {
        let visible_row_index = self.visible_row_indices[block_index].unwrap_or_else(|| {
            panic!(
                "flow-visible Notion block {} has no canonical visible row",
                self.page.blocks[block_index].block_id
            )
        });
        let block = &self.page.blocks[block_index];
        let is_callout = block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout);
        self.assert_callout_context(visible_row_index, inherited_callout, is_callout);
        let active_callout = if is_callout {
            inherited_callout.push(Arc::from(block.block_id.as_str()), visible_row_index)
        } else {
            inherited_callout.clone()
        };
        self.append_row_units(visible_row_index, &active_callout, sequence);
        if block.simple_table_content().is_none() {
            self.append_children(block_index, &active_callout, sequence);
        }
    }

    fn append_children(
        &mut self,
        block_index: usize,
        callout: &CalloutContext,
        sequence: &mut SequenceBuild,
    ) {
        let children = self.hierarchy.children[block_index].clone();
        for child_index in children.iter().copied() {
            self.append_block(child_index, callout, sequence);
        }
    }

    fn append_row_units(
        &self,
        visible_row_index: usize,
        callout: &CalloutContext,
        sequence: &mut SequenceBuild,
    ) {
        for document_unit_index in self.document_unit_ranges[visible_row_index].clone() {
            assert_eq!(
                self.document_units[document_unit_index].owner_visible_row_index(),
                visible_row_index,
                "Notion document unit must remain owned by its canonical visible row"
            );
            sequence.pending_units.push(PendingFlowUnit {
                document_unit_index,
                callout_path: callout.path.clone(),
                callout_tail_visible_row_index: callout.tail_visible_row_index,
            });
        }
    }

    fn assert_callout_context(
        &self,
        visible_row_index: usize,
        inherited: &CalloutContext,
        row_is_callout: bool,
    ) {
        let row = &self.rows[visible_row_index];
        assert_eq!(
            row.callout_parent_row_index,
            inherited.tail_visible_row_index
        );
        let layer_range = self.callout_layer_row_ranges[visible_row_index].clone();
        let expected_depth = inherited.depth + usize::from(row_is_callout);
        assert_eq!(layer_range.len(), expected_depth);
        let expected_tail = row_is_callout
            .then_some(visible_row_index)
            .or(inherited.tail_visible_row_index);
        assert_eq!(
            layer_range
                .last()
                .map(|index| self.callout_layer_row_indices[index]),
            expected_tail
        );
    }

    fn assert_exact_document_unit_coverage(&self) {
        if let Some(index) = self
            .flow_visible_block_mask
            .iter()
            .zip(&self.visited_flow_blocks)
            .position(|(visible, visited)| *visible && !visited)
        {
            panic!(
                "flow-visible Notion block {} is absent from the page flow",
                self.page.blocks[index].block_id
            );
        }
        if let Some(index) = self
            .assigned_document_units
            .iter()
            .position(|assigned| !assigned)
        {
            panic!(
                "Notion document unit {index} ({:?}) is absent from the page flow",
                self.document_units[index].key()
            );
        }
        assert_eq!(self.unit_locations.len(), self.document_units.len());
    }

    fn claim_flow_block(&mut self, block_index: usize) {
        assert!(self.flow_visible_block_mask[block_index]);
        assert!(
            !std::mem::replace(&mut self.visited_flow_blocks[block_index], true),
            "flow-visible Notion block was assigned twice"
        );
    }
}
