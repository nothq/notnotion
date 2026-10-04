use std::{collections::HashMap, sync::Arc};

use crate::model::{CardPageColumnEffectiveShare, CardPageColumnRatio, CardPageColumnWeightTotal};
use crate::ui::{
    LoadedCardPageData, PageDocumentUnitKey, PageDocumentUnitLayoutRevision, PageFlowColumns,
    PageFlowDecoratorPlanRevision, PageFlowNode, PageFlowNodeId, PageFlowNodeKey, PageFlowSection,
};

pub(super) struct PageFlowRootLayoutAuthoritySnapshot {
    roots: HashMap<PageFlowNodeId, PageFlowNodeLayoutAuthority>,
}

enum PageFlowNodeLayoutAuthority {
    Section {
        node_id: PageFlowNodeId,
        last_unit: PageDocumentUnitKey,
        decorator_revision: PageFlowDecoratorPlanRevision,
        unit_revisions: Arc<[PageDocumentUnitLayoutRevision]>,
    },
    Columns {
        node_id: PageFlowNodeId,
        decorator_revision: PageFlowDecoratorPlanRevision,
        weight_total: CardPageColumnWeightTotal,
        columns: Arc<[PageFlowColumnLayoutAuthority]>,
    },
}

struct PageFlowColumnLayoutAuthority {
    column_block_id: Arc<str>,
    raw_weight: Option<CardPageColumnRatio>,
    layout_weight: CardPageColumnRatio,
    effective_share: CardPageColumnEffectiveShare,
    nodes: Arc<[PageFlowNodeLayoutAuthority]>,
}

impl PageFlowRootLayoutAuthoritySnapshot {
    pub(super) fn new(data: &LoadedCardPageData) -> Self {
        let roots = data
            .flow
            .root
            .nodes
            .iter()
            .map(|node| {
                (
                    node.node_id(),
                    PageFlowNodeLayoutAuthority::new(node, &data.document_unit_layout_revisions),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(roots.len(), data.flow.root.nodes.len());
        Self { roots }
    }

    pub(super) fn changed_root_ids(&self, data: &LoadedCardPageData) -> Vec<PageFlowNodeId> {
        data.flow
            .root
            .nodes
            .iter()
            .filter_map(|node| {
                let unchanged = self.roots.get(&node.node_id()).is_some_and(|authority| {
                    authority.matches(node, &data.document_unit_layout_revisions)
                });
                (!unchanged).then(|| node.node_id())
            })
            .collect()
    }
}

impl PageFlowNodeLayoutAuthority {
    fn new(node: &PageFlowNode, revisions: &[PageDocumentUnitLayoutRevision]) -> Self {
        match node {
            PageFlowNode::Section(section) => Self::new_section(section, revisions),
            PageFlowNode::Columns(columns) => Self::new_columns(columns, revisions),
        }
    }

    fn new_section(
        section: &PageFlowSection,
        revisions: &[PageDocumentUnitLayoutRevision],
    ) -> Self {
        Self::Section {
            node_id: section.key.node_id(),
            last_unit: section_last_unit(&section.key).clone(),
            decorator_revision: section.decorator_plan.revision.clone(),
            unit_revisions: revisions[section.document_unit_range.clone()].into(),
        }
    }

    fn new_columns(
        columns: &PageFlowColumns,
        revisions: &[PageDocumentUnitLayoutRevision],
    ) -> Self {
        Self::Columns {
            node_id: columns.key.node_id(),
            decorator_revision: columns.decorator_plan.revision.clone(),
            weight_total: columns.weight_total,
            columns: columns
                .columns
                .iter()
                .map(|column| PageFlowColumnLayoutAuthority {
                    column_block_id: column.column_block_id.clone(),
                    raw_weight: column.raw_weight,
                    layout_weight: column.layout_weight,
                    effective_share: column.effective_share,
                    nodes: column
                        .sequence
                        .nodes
                        .iter()
                        .map(|node| Self::new(node, revisions))
                        .collect::<Vec<_>>()
                        .into(),
                })
                .collect::<Vec<_>>()
                .into(),
        }
    }

    fn matches(&self, node: &PageFlowNode, revisions: &[PageDocumentUnitLayoutRevision]) -> bool {
        match (self, node) {
            (
                Self::Section {
                    node_id,
                    last_unit,
                    decorator_revision,
                    unit_revisions,
                },
                PageFlowNode::Section(section),
            ) => {
                node_id == &section.key.node_id()
                    && last_unit == section_last_unit(&section.key)
                    && decorator_revision == &section.decorator_plan.revision
                    && unit_revisions.as_ref() == &revisions[section.document_unit_range.clone()]
            }
            (Self::Columns { .. }, PageFlowNode::Columns(columns)) => {
                self.matches_columns(columns, revisions)
            }
            (Self::Section { .. }, PageFlowNode::Columns(_))
            | (Self::Columns { .. }, PageFlowNode::Section(_)) => false,
        }
    }

    fn matches_columns(
        &self,
        current: &PageFlowColumns,
        revisions: &[PageDocumentUnitLayoutRevision],
    ) -> bool {
        let Self::Columns {
            node_id,
            decorator_revision,
            weight_total,
            columns,
        } = self
        else {
            return false;
        };
        node_id == &current.key.node_id()
            && decorator_revision == &current.decorator_plan.revision
            && weight_total == &current.weight_total
            && columns.len() == current.columns.len()
            && columns
                .iter()
                .zip(current.columns.iter())
                .all(|(authority, column)| authority.matches(column, revisions))
    }
}

impl PageFlowColumnLayoutAuthority {
    fn matches(
        &self,
        current: &crate::ui::PageColumnFlow,
        revisions: &[PageDocumentUnitLayoutRevision],
    ) -> bool {
        self.column_block_id == current.column_block_id
            && self.raw_weight == current.raw_weight
            && self.layout_weight == current.layout_weight
            && self.effective_share == current.effective_share
            && self.nodes.len() == current.sequence.nodes.len()
            && self
                .nodes
                .iter()
                .zip(current.sequence.nodes.iter())
                .all(|(authority, node)| authority.matches(node, revisions))
    }
}

fn section_last_unit(key: &PageFlowNodeKey) -> &PageDocumentUnitKey {
    match key {
        PageFlowNodeKey::Section { last_unit, .. } => last_unit,
        PageFlowNodeKey::Columns { .. } => unreachable!("Section must retain its node key"),
    }
}
