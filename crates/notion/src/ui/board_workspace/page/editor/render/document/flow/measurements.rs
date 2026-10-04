use std::sync::Arc;

use gpui::{App, FocusHandle, InteractiveElement, IntoElement, ListState, Window};

use crate::ui::surface::{
    PageDocumentFlowRuntime, PageFlowExactLayoutWidth, PageFlowLaneObservation,
    PageFlowLayoutCommitSchedule, PageFlowNodeObservation, PageFlowSectionMeasurement,
};
use crate::ui::{LoadedCardPageData, PageFlowLaneKey, PageFlowNode, PageFlowNodeId};

use super::super::super::super::{div, px, AnyElement, FluentBuilder, ParentElement, Styled};
use super::PageFlowRenderContext;
use crate::ui::board_workspace::page::editor::render::PageBlockRenderer;

mod commit;

/// A root layout frame to commit, with the observation, page data, and list it
/// commits against.
struct PageFlowRootCommit {
    frame: crate::ui::surface::PageFlowLayoutFrameToken,
    observation: crate::ui::surface::PageFlowObservationToken,
    data: Arc<LoadedCardPageData>,
    list_state: ListState,
}

pub(super) fn observe_page_flow_node(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    node: &PageFlowNode,
    lane: &PageFlowLaneKey,
    rendered: AnyElement,
) -> AnyElement {
    let renderer = renderer.clone();
    let frame = context.frame.clone();
    let observation = context.observation.clone();
    let data = context.data.clone();
    let node_id = node.node_id();
    let lane = lane.clone();
    let measure_section = matches!(node, PageFlowNode::Section(_));
    div()
        .w_full()
        .min_w(px(0.0))
        .child(rendered)
        .on_children_prepainted(move |bounds, _, _| {
            if !renderer.notifier.is_alive() {
                return;
            }
            let bounds = bounds
                .first()
                .expect("rendered Notion flow Section must retain its measured child");
            let width =
                PageFlowExactLayoutWidth::from_observed_content_box(bounds.size.width.as_f32());
            let extent = bounds.size.height.as_f32();
            renderer
                .flow
                .state()
                .observations
                .borrow_mut()
                .observe_node(
                    &observation,
                    PageFlowNodeObservation::new(&data, node_id.clone(), &lane, *bounds),
                );
            if measure_section {
                renderer
                    .flow
                    .queue_page_flow_section_measurement(&frame, &node_id, width, extent);
            }
        })
        .into_any_element()
}

pub(super) fn observe_page_flow_root(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    commit_list_state: &ListState,
    focus_handle: &FocusHandle,
    rendered: AnyElement,
) -> AnyElement {
    let renderer = renderer.clone();

    let frame = context.frame.clone();
    let data = context.data.clone();
    let observation = context.observation.clone();
    let root_lane = context.data.flow.root.lane.clone();
    let commit_list_state = commit_list_state.clone();
    div()
        .w_full()
        .when_some(renderer.column.maximum_width(), |root, width| {
            root.max_w(px(width))
        })
        .min_w(px(0.0))
        .track_focus(focus_handle)
        .child(rendered)
        .on_children_prepainted(move |bounds, window, cx| {
            if !renderer.notifier.is_alive() {
                return;
            }
            let [bounds] = bounds.as_slice() else {
                panic!("rendered Notion root flow node must retain exactly one measured child");
            };
            let width =
                PageFlowExactLayoutWidth::from_observed_content_box(bounds.size.width.as_f32());
            renderer
                .flow
                .state()
                .observations
                .borrow_mut()
                .observe_lane(
                    &observation,
                    PageFlowLaneObservation::new(root_lane.clone(), *bounds, Vec::new()),
                );
            let scheduled =
                renderer
                    .flow
                    .prepare_page_flow_root_commit(&frame, &observation, width);
            if scheduled {
                defer_page_flow_root_commit(
                    renderer.clone(),
                    PageFlowRootCommit {
                        frame: frame.clone(),
                        observation: observation.clone(),
                        data: data.clone(),
                        list_state: commit_list_state.clone(),
                    },
                    window,
                    cx,
                );
            }
        })
        .into_any_element()
}

fn defer_page_flow_root_commit(
    renderer: PageBlockRenderer,
    root_commit: PageFlowRootCommit,
    window: &mut Window,
    cx: &mut App,
) {
    window.defer(cx, move |window, cx| {
        if !renderer.notifier.is_alive() {
            return;
        }
        commit::commit_page_flow_layout_frame(&renderer, &root_commit, window, cx);
        let PageFlowRootCommit {
            observation, data, ..
        } = &root_commit;
        renderer
            .flow
            .state()
            .observations
            .borrow_mut()
            .prepare_drag_geometry(observation, data);
        if cx.has_active_drag()
            && renderer.drag.refresh_from_flow_observations(
                &renderer.flow,
                observation.surface(),
                data,
            )
        {
            renderer.notifier.notify(cx);
        }
    });
}

impl PageDocumentFlowRuntime {
    fn prepare_page_flow_root_commit(
        &self,
        frame: &crate::ui::surface::PageFlowLayoutFrameToken,
        observation: &crate::ui::surface::PageFlowObservationToken,
        width: PageFlowExactLayoutWidth,
    ) -> bool {
        let target = self
            .state()
            .virtualizer
            .borrow()
            .semantic_anchor_target(frame);
        let projection = target.as_ref().and_then(|target| {
            self.state()
                .observations
                .borrow()
                .mounted_projection(observation, target)
        });
        let mut virtualizer = self.state().virtualizer.borrow_mut();
        if !virtualizer.observe_root_sequence_exact_width(frame, width) {
            return false;
        }
        if let Some(projection) = projection {
            virtualizer.update_semantic_anchor_projection(frame, projection);
        }
        matches!(
            virtualizer.schedule_layout_frame_commit(frame),
            PageFlowLayoutCommitSchedule::Schedule
        )
    }

    fn queue_page_flow_section_measurement(
        &self,
        frame: &crate::ui::surface::PageFlowLayoutFrameToken,
        node_id: &PageFlowNodeId,
        width: PageFlowExactLayoutWidth,
        extent: f32,
    ) {
        self.state()
            .virtualizer
            .borrow_mut()
            .queue_section_measurement(PageFlowSectionMeasurement {
                frame,
                node_id,
                width,
                extent,
            });
    }
}
