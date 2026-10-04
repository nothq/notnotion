use std::sync::Arc;

use crate::{
    model::{CardPageColumnLayoutRead, CardPageColumnWeightTotal},
    ui::CardPageLayoutBlock,
};

use super::super::{
    PageColumnFlow, PageFlowColumns, PageFlowDecoratorPlan, PageFlowNode, PageFlowNodeKey,
    PageFlowNodePath, PageFlowOuterItem,
};
use super::{CalloutContext, PageFlowBuilder, ResolvedColumn, SequenceBuild, SequenceParent};

/// The column list whose columns are being built.
struct ColumnGroup<'a> {
    column_list_id: &'a Arc<str>,
    node_path: &'a PageFlowNodePath,
    outer_item: &'a PageFlowOuterItem,
}

/// A column list's resolved columns, their weight total, and whether every
/// authoritative child column is present for resizing.
type ResolvedColumnList = (Vec<ResolvedColumn>, CardPageColumnWeightTotal, bool);

impl PageFlowBuilder<'_> {
    pub(super) fn insert_node_path(
        &mut self,
        key: &PageFlowNodeKey,
        path: &PageFlowNodePath,
        outer_item: &PageFlowOuterItem,
    ) {
        let node_id = key.node_id();
        assert!(
            self.node_paths
                .insert(node_id.clone(), path.clone())
                .is_none(),
            "Notion flow node IDs must be unique"
        );
        assert!(
            self.node_outer_items
                .insert(node_id, outer_item.clone())
                .is_none(),
            "Notion flow node outer identities must be unique"
        );
    }

    pub(super) fn append_columns(
        &mut self,
        column_list_block_index: usize,
        callout: &CalloutContext,
        sequence: &mut SequenceBuild,
    ) {
        let Some((resolved_columns, weight_total, resize_authority_complete)) =
            self.resolved_columns(column_list_block_index)
        else {
            return;
        };
        self.flush_pending_sections(sequence);
        let column_list_id: Arc<str> =
            Arc::from(self.page.blocks[column_list_block_index].block_id.as_str());
        let key = PageFlowNodeKey::Columns {
            column_list_block_id: column_list_id.clone(),
        };
        let node_path = PageFlowNodePath::push(sequence.parent_node_path.as_ref(), key.clone());
        let outer_item = sequence
            .outer_item
            .clone()
            .unwrap_or_else(|| PageFlowOuterItem {
                index: sequence.nodes.len(),
                key: key.clone(),
            });
        self.insert_node_path(&key, &node_path, &outer_item);
        let group = ColumnGroup {
            column_list_id: &column_list_id,
            node_path: &node_path,
            outer_item: &outer_item,
        };
        let columns = resolved_columns
            .into_iter()
            .map(|column| self.build_column(&group, column, callout, sequence))
            .collect::<Vec<_>>()
            .into();
        sequence.nodes.push(PageFlowNode::Columns(PageFlowColumns {
            key,
            column_list_block_index,
            common_callout_path: callout.path.clone(),
            decorator_plan: PageFlowDecoratorPlan::unresolved(),
            weight_total,
            resize_authority_complete,
            columns,
        }));
    }

    fn build_column(
        &mut self,
        group: &ColumnGroup<'_>,
        resolved: ResolvedColumn,
        callout: &CalloutContext,
        sequence: &SequenceBuild,
    ) -> PageColumnFlow {
        let column_block_index = resolved.block_index;
        assert!(
            self.flow_visible_block_mask[column_block_index],
            "visible Notion column list contains a disclosure-hidden column"
        );
        self.claim_flow_block(column_block_index);
        let lane = sequence
            .lane
            .column(group.column_list_id.clone(), resolved.block_id.clone());
        let children = self.hierarchy.children[column_block_index].clone();
        let child_sequence = self.build_sequence(
            &children,
            lane.clone(),
            callout.clone(),
            SequenceParent {
                node_path: Some(group.node_path.clone()),
                outer_item: Some(group.outer_item.clone()),
            },
        );
        PageColumnFlow {
            column_block_index,
            column_block_id: resolved.block_id,
            lane,
            raw_weight: resolved.raw_weight,
            layout_weight: resolved.layout_weight,
            effective_share: resolved.effective_share,
            sequence: child_sequence,
        }
    }

    fn resolved_columns(&self, column_list_block_index: usize) -> Option<ResolvedColumnList> {
        let column_list_id = &self.page.blocks[column_list_block_index].block_id;
        let read = self
            .column_layouts
            .layout(column_list_id)
            .unwrap_or_else(|error| panic!("cannot project Notion column list: {error}"));
        let CardPageColumnLayoutRead::Columns(layout) = read else {
            return None;
        };
        let authoritative_child_count = self.hierarchy.children[column_list_block_index].len();
        let child_indices = self.hierarchy.children[column_list_block_index]
            .iter()
            .copied()
            .filter(|block_index| !self.page.blocks[*block_index].is_opaque_unavailable())
            .collect::<Vec<_>>();
        assert_eq!(
            layout.columns().len(),
            child_indices.len(),
            "resolved Notion columns must match authoritative direct children"
        );
        let resize_authority_complete = child_indices.len() == authoritative_child_count;
        let columns = layout
            .columns()
            .iter()
            .zip(child_indices)
            .map(|(column, block_index)| self.resolved_column(column, block_index))
            .collect();
        Some((columns, layout.weight_total(), resize_authority_complete))
    }

    fn resolved_column(
        &self,
        column: &crate::model::CardPageColumnLayoutEntry<'_>,
        block_index: usize,
    ) -> ResolvedColumn {
        let block = &self.page.blocks[block_index];
        let Some(CardPageLayoutBlock::Column { ratio }) = block.layout_content() else {
            panic!(
                "Notion column list contains non-column child {}",
                block.block_id
            );
        };
        assert_eq!(column.block_id(), block.block_id.as_str());
        assert_eq!(column.raw_weight(), *ratio);
        ResolvedColumn {
            block_index,
            block_id: Arc::from(column.block_id()),
            raw_weight: column.raw_weight(),
            layout_weight: column.weight(),
            effective_share: column.effective_share(),
        }
    }
}
