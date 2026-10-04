use std::sync::Arc;

use gpui::{Bounds, IntoElement, Pixels};

use crate::model::CardPageColumnEffectiveShare;
use crate::ui::surface::{
    allocate_page_flow_lane_widths, PageDocumentFlowRuntime, PageFlowExactLayoutWidth,
    PageFlowLaneObservation, PageFlowLayoutWidth, PageFlowRenderSpan, PageFlowSequenceWidth,
    PageFlowViewport,
};
use crate::ui::{PageFlowColumns, PageFlowLaneKey, PageFlowSequence, PageFlowSequenceId};

use super::super::super::super::{div, px, AnyElement, App, FluentBuilder, ParentElement, Styled};
use super::super::super::PageBlockRenderer;
use super::{
    translate_page_flow_viewport, PageFlowLaneNode, PageFlowRenderArea, PageFlowRenderContext,
};

mod divider;

use divider::{render_page_flow_column_divider, PageFlowColumnDivider};

#[derive(Clone)]
struct PageFlowObservedLane {
    path: PageFlowLaneKey,
    sequence_id: PageFlowSequenceId,
}

/// How a Columns node's lanes lay out: the viewport they share, each lane's
/// width, and the width of the dividers between them.
struct PageFlowColumnLaneLayout<'a> {
    viewport: PageFlowViewport,
    widths: &'a [PageFlowLayoutWidth],
    divider_width: f32,
}

struct PageFlowColumnsObservation {
    lanes: Arc<[PageFlowObservedLane]>,
    token: crate::ui::surface::PageFlowObservationToken,
    frame: crate::ui::surface::PageFlowLayoutFrameToken,
    viewport: PageFlowViewport,
}

pub(super) fn render_page_flow_columns(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    columns: &PageFlowColumns,
    area: PageFlowRenderArea,
    cx: &mut App,
) -> AnyElement {
    let PageFlowRenderArea {
        viewport,
        width: layout_width,
    } = area;
    let spec = crate::ui::PageFlowColumnsPresentationSpec::NOTION;
    let shares = page_flow_column_shares(renderer, context, columns);
    let widths = allocate_page_flow_lane_widths(layout_width, &shares);
    assert_eq!(widths.as_slice().len(), columns.columns.len());
    let lane_viewport = translate_page_flow_viewport(viewport, spec.top_inset);
    let lanes = render_page_flow_column_children(
        renderer,
        context,
        columns,
        PageFlowColumnLaneLayout {
            viewport: lane_viewport,
            widths: widths.as_slice(),
            divider_width: spec.lane_gap,
        },
        cx,
    );
    let observation = page_flow_columns_observation(context, columns, lane_viewport);
    let lane_count = observation.lanes.len();
    let flow = renderer.flow.clone();
    let row = div()
        .w_full()
        .min_w(px(0.0))
        .flex()
        .items_stretch()
        .children(lanes)
        .on_children_prepainted(move |mut bounds, _, _cx| {
            let bounds = take_lane_bounds(&mut bounds, lane_count);
            observe_page_flow_column_lanes(&flow, &observation, bounds);
        });
    div()
        .w_full()
        .when_some(renderer.column.maximum_width(), |columns, width| {
            columns.max_w(px(width))
        })
        .min_w(px(0.0))
        .pt(px(spec.top_inset))
        .pb(px(spec.bottom_inset))
        .child(row)
        .into_any_element()
}

fn page_flow_column_shares(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    columns: &PageFlowColumns,
) -> Vec<CardPageColumnEffectiveShare> {
    columns
        .columns
        .iter()
        .map(|column| {
            renderer
                .drag
                .borrow()
                .column_resize
                .as_ref()
                .and_then(|resize| {
                    resize.effective_share_for(
                        context.data,
                        context.observation.surface(),
                        column_list_id(columns),
                        &column.column_block_id,
                    )
                })
                .unwrap_or(column.effective_share)
        })
        .collect()
}

fn render_page_flow_column_children(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    columns: &PageFlowColumns,
    lane_layout: PageFlowColumnLaneLayout<'_>,
    cx: &mut App,
) -> Vec<AnyElement> {
    let PageFlowColumnLaneLayout {
        viewport,
        widths,
        divider_width,
    } = lane_layout;
    let mut children = Vec::with_capacity(columns.columns.len().saturating_mul(2));
    for (index, (column, width)) in columns
        .columns
        .iter()
        .zip(widths.iter().copied())
        .enumerate()
    {
        children.push(render_page_flow_lane(
            renderer,
            context,
            &column.sequence,
            PageFlowRenderArea { viewport, width },
            cx,
        ));
        if let Some(right) = columns.columns.get(index + 1) {
            children.push(render_page_flow_column_divider(
                renderer,
                context,
                columns,
                PageFlowColumnDivider {
                    left: column,
                    right,
                    width: divider_width,
                },
                cx,
            ));
        }
    }
    children
}

fn take_lane_bounds(bounds: &mut Vec<Bounds<Pixels>>, lane_count: usize) -> Vec<Bounds<Pixels>> {
    let expected = lane_count.saturating_mul(2).saturating_sub(1);
    bounds.truncate(expected);
    assert_eq!(bounds.len(), expected);
    bounds.iter().step_by(2).copied().collect()
}

fn column_list_id(columns: &PageFlowColumns) -> &Arc<str> {
    match &columns.key {
        crate::ui::PageFlowNodeKey::Columns {
            column_list_block_id,
        } => column_list_block_id,
        crate::ui::PageFlowNodeKey::Section { .. } => {
            unreachable!("Columns flow must retain a Columns key")
        }
    }
}

fn page_flow_columns_observation(
    context: PageFlowRenderContext<'_>,
    columns: &PageFlowColumns,
    viewport: PageFlowViewport,
) -> PageFlowColumnsObservation {
    PageFlowColumnsObservation {
        lanes: columns
            .columns
            .iter()
            .map(|column| PageFlowObservedLane {
                path: column.lane.clone(),
                sequence_id: column.sequence.sequence_id(),
            })
            .collect::<Vec<_>>()
            .into(),
        token: context.observation.clone(),
        frame: context.frame.clone(),
        viewport,
    }
}

fn render_page_flow_lane(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    sequence: &PageFlowSequence,
    area: PageFlowRenderArea,
    cx: &mut App,
) -> AnyElement {
    let lane_minimum = crate::ui::PageFlowColumnsPresentationSpec::NOTION.lane_minimum;
    div()
        .relative()
        .w(px(area.width.pixels()))
        .min_w(px(0.0))
        .min_h(px(lane_minimum))
        .self_stretch()
        .flex_none()
        .flex()
        .flex_col()
        .child(renderer.render_page_flow_sequence(context, sequence, area, cx))
        .into_any_element()
}

pub(super) fn render_page_flow_sequence(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    sequence: &PageFlowSequence,
    area: PageFlowRenderArea,
    cx: &mut App,
) -> AnyElement {
    let PageFlowRenderArea { viewport, width } = area;
    let plan = renderer
        .flow
        .state()
        .virtualizer
        .borrow_mut()
        .plan_sequence(
            context.frame,
            &sequence.sequence_id(),
            PageFlowSequenceWidth::Provisional(width),
            viewport,
        )
        .expect("recursive Notion flow frame must plan every projected lane");
    let total_extent = plan.total_extent;
    let mut children = Vec::with_capacity(plan.spans.len());
    for span in plan.spans {
        match span {
            PageFlowRenderSpan::Spacer { extent } => {
                children.push(div().h(px(extent)).flex_none().into_any_element())
            }
            PageFlowRenderSpan::Nodes(nodes) => {
                for planned in nodes.iter() {
                    debug_assert!(planned.extent.is_finite() && planned.extent >= 0.0);
                    let node = &sequence.nodes[planned.index];
                    children.push(renderer.render_page_flow_node(
                        context,
                        PageFlowLaneNode {
                            node,
                            lane: &sequence.lane,
                        },
                        PageFlowRenderArea {
                            viewport: translate_page_flow_viewport(viewport, planned.offset),
                            width,
                        },
                        cx,
                    ));
                }
            }
        }
    }
    div()
        .w_full()
        .min_w(px(0.0))
        .min_h(px(total_extent))
        .flex()
        .flex_col()
        .children(children)
        .into_any_element()
}

fn observe_page_flow_column_lanes(
    flow: &PageDocumentFlowRuntime,
    observation: &PageFlowColumnsObservation,
    bounds: Vec<Bounds<Pixels>>,
) {
    assert_eq!(observation.lanes.len(), bounds.len());
    for (lane, bounds) in observation.lanes.iter().zip(bounds) {
        let current = flow.state().observations.borrow_mut().observe_lane(
            &observation.token,
            PageFlowLaneObservation::new(lane.path.clone(), bounds, Vec::new()),
        );
        if !current {
            return;
        }
        let exact = PageFlowExactLayoutWidth::from_observed_content_box(bounds.size.width.as_f32());
        let _ = flow.state().virtualizer.borrow_mut().plan_sequence(
            &observation.frame,
            &lane.sequence_id,
            PageFlowSequenceWidth::Exact(exact),
            observation.viewport,
        );
    }
}
