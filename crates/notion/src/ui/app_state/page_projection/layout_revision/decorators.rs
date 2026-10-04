use std::collections::HashMap;

use crate::ui::{CardPage, CardPageBlockKind};

use super::super::{
    LoadedCardPageData, LoadedCardPageVisibleRow, PageFlowCalloutLayoutRevision,
    PageFlowDecoratorLayer, PageFlowDecoratorPlan, PageFlowNode, PageFlowNodeId, PageFlowSequence,
};

pub(super) fn fresh_callout_layout_revisions(
    page: &CardPage,
    rows: &[LoadedCardPageVisibleRow],
) -> Vec<Option<PageFlowCalloutLayoutRevision>> {
    rows.iter()
        .map(|row| {
            page.blocks[row.block_index]
                .editable_content()
                .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
                .then(PageFlowCalloutLayoutRevision::fresh)
        })
        .collect()
}

pub(super) fn reuse_decorator_layout_revisions(
    data: &mut LoadedCardPageData,
    previous: &LoadedCardPageData,
    equivalent_rows: &[Option<usize>],
) {
    reuse_callout_layout_revisions(data, previous, equivalent_rows);
    let mut previous_plans = HashMap::new();
    collect_plans(&previous.flow.root, &mut previous_plans);
    reuse_plan_revisions(
        &mut data.flow.root,
        &data.callout_layout_revisions,
        &previous.callout_layout_revisions,
        equivalent_rows,
        &previous_plans,
    );
}

fn reuse_callout_layout_revisions(
    data: &mut LoadedCardPageData,
    previous: &LoadedCardPageData,
    equivalent_rows: &[Option<usize>],
) {
    for (row_index, previous_row_index) in equivalent_rows.iter().copied().enumerate() {
        let Some(previous_row_index) = previous_row_index else {
            continue;
        };
        let current_is_callout = data.callout_layout_revisions[row_index].is_some();
        let previous_revision = previous.callout_layout_revisions[previous_row_index].clone();
        match (current_is_callout, previous_revision) {
            (true, Some(previous_revision)) => {
                data.callout_layout_revisions[row_index] = Some(previous_revision);
            }
            (false, None) => {}
            (true, None) | (false, Some(_)) => {
                panic!("equivalent Notion rows must agree on Callout layout authority")
            }
        }
    }
}

fn collect_plans(
    sequence: &PageFlowSequence,
    plans: &mut HashMap<PageFlowNodeId, PageFlowDecoratorPlan>,
) {
    for node in sequence.nodes.iter() {
        assert!(
            plans
                .insert(node.node_id(), node.decorator_plan().clone())
                .is_none(),
            "Notion flow node IDs must be unique"
        );
        if let PageFlowNode::Columns(columns) = node {
            for column in columns.columns.iter() {
                collect_plans(&column.sequence, plans);
            }
        }
    }
}

fn reuse_plan_revisions(
    sequence: &mut PageFlowSequence,
    revisions: &[Option<PageFlowCalloutLayoutRevision>],
    previous_revisions: &[Option<PageFlowCalloutLayoutRevision>],
    equivalent_rows: &[Option<usize>],
    previous_plans: &HashMap<PageFlowNodeId, PageFlowDecoratorPlan>,
) {
    for node in std::sync::Arc::make_mut(&mut sequence.nodes) {
        let node_id = node.node_id();
        let plan = node_decorator_plan_mut(node);
        if let Some(previous) = previous_plans.get(&node_id) {
            if plans_match(
                plan,
                previous,
                revisions,
                previous_revisions,
                equivalent_rows,
            ) {
                plan.revision = previous.revision.clone();
            }
        }
        if let PageFlowNode::Columns(columns) = node {
            for column in std::sync::Arc::make_mut(&mut columns.columns) {
                reuse_plan_revisions(
                    &mut column.sequence,
                    revisions,
                    previous_revisions,
                    equivalent_rows,
                    previous_plans,
                );
            }
        }
    }
}

fn plans_match(
    plan: &PageFlowDecoratorPlan,
    previous: &PageFlowDecoratorPlan,
    revisions: &[Option<PageFlowCalloutLayoutRevision>],
    previous_revisions: &[Option<PageFlowCalloutLayoutRevision>],
    equivalent_rows: &[Option<usize>],
) -> bool {
    plan.root_leading == previous.root_leading
        && plan.content_origin == previous.content_origin
        && plan.layers.len() == previous.layers.len()
        && plan
            .layers
            .iter()
            .zip(previous.layers.iter())
            .all(|(layer, previous_layer)| {
                decorator_layers_match(
                    layer,
                    previous_layer,
                    revisions,
                    previous_revisions,
                    equivalent_rows,
                )
            })
}

fn decorator_layers_match(
    layer: &PageFlowDecoratorLayer,
    previous: &PageFlowDecoratorLayer,
    revisions: &[Option<PageFlowCalloutLayoutRevision>],
    previous_revisions: &[Option<PageFlowCalloutLayoutRevision>],
    equivalent_rows: &[Option<usize>],
) -> bool {
    layer.callout_block_id == previous.callout_block_id
        && layer.depth == previous.depth
        && layer.presentation == previous.presentation
        && layer.layout == previous.layout
        && equivalent_rows.get(layer.visible_row_index) == Some(&Some(previous.visible_row_index))
        && revisions[layer.visible_row_index] == previous_revisions[previous.visible_row_index]
}

fn node_decorator_plan_mut(node: &mut PageFlowNode) -> &mut PageFlowDecoratorPlan {
    match node {
        PageFlowNode::Section(section) => &mut section.decorator_plan,
        PageFlowNode::Columns(columns) => &mut columns.decorator_plan,
    }
}
