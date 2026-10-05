use gpui::{FocusHandle, ListState};

use crate::ui::surface::{PageFlowLayoutFrameToken, PageFlowLayoutWidth, PageFlowViewport};
use crate::ui::{LoadedCardPageData, PageFlowNode, PageFlowSequence};

use super::super::super::{
    AnyElement, App, Arc, PageBlockDragScrollTarget, PageBlockDragSelection,
};
use super::super::PageBlockRenderer;

mod columns;
mod decorators;
mod measurements;
mod section;
mod viewport;

use decorators::render_page_flow_decorated_node;
pub(super) use viewport::PageFlowOuterListFrame;
use viewport::{inset_page_flow_width, translate_page_flow_viewport};

pub(super) struct PageFlowRootNodeRender<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) node: &'a PageFlowNode,
    pub(super) nesting_offsets: &'a [f32],
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) observation: &'a crate::ui::surface::PageFlowObservationToken,
    pub(super) frame: &'a PageFlowLayoutFrameToken,
    pub(super) viewport: PageFlowViewport,
    pub(super) layout_width: PageFlowLayoutWidth,
    pub(super) commit_list_state: &'a ListState,
    pub(super) focus_handle: &'a FocusHandle,
}

/// The viewport slice and layout width a flow item renders into.
#[derive(Clone, Copy)]
pub(super) struct PageFlowRenderArea {
    pub(super) viewport: PageFlowViewport,
    pub(super) width: PageFlowLayoutWidth,
}

/// A flow node and the lane it sits in.
#[derive(Clone, Copy)]
pub(super) struct PageFlowLaneNode<'a> {
    pub(super) node: &'a PageFlowNode,
    pub(super) lane: &'a crate::ui::PageFlowLaneKey,
}

#[derive(Clone, Copy)]
pub(super) struct PageFlowRenderContext<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) nesting_offsets: &'a [f32],
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) observation: &'a crate::ui::surface::PageFlowObservationToken,
    pub(super) frame: &'a PageFlowLayoutFrameToken,
}

impl PageBlockRenderer {
    pub(super) fn render_page_flow_root_node(
        &self,
        render: PageFlowRootNodeRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let context = PageFlowRenderContext {
            data: render.data,
            nesting_offsets: render.nesting_offsets,
            drag_selection: render.drag_selection,
            scroll_target: render.scroll_target,
            observation: render.observation,
            frame: render.frame,
        };
        let node = self.render_page_flow_node(
            context,
            PageFlowLaneNode {
                node: render.node,
                lane: &render.data.flow.root.lane,
            },
            PageFlowRenderArea {
                viewport: render.viewport,
                width: render.layout_width,
            },
            cx,
        );
        measurements::observe_page_flow_root(
            self,
            context,
            render.commit_list_state,
            render.focus_handle,
            node,
        )
    }

    pub(super) fn render_page_flow_node(
        &self,
        context: PageFlowRenderContext<'_>,
        lane_node: PageFlowLaneNode<'_>,
        area: PageFlowRenderArea,
        cx: &mut App,
    ) -> AnyElement {
        let PageFlowLaneNode { node, lane } = lane_node;
        let PageFlowRenderArea {
            viewport,
            width: layout_width,
        } = area;
        let plan = node.decorator_plan();
        let content_viewport =
            translate_page_flow_viewport(viewport, plan.content_origin.block_offset());
        let content_width =
            inset_page_flow_width(layout_width, plan.content_origin.horizontal_insets());
        let content = match node {
            PageFlowNode::Section(section) => {
                section::render_page_flow_section(self, context, section, content_width, cx)
            }
            PageFlowNode::Columns(columns) => columns::render_page_flow_columns(
                self,
                context,
                columns,
                PageFlowRenderArea {
                    viewport: content_viewport,
                    width: content_width,
                },
                cx,
            ),
        };
        let decorated = render_page_flow_decorated_node(self, context, node, content, cx);
        measurements::observe_page_flow_node(self, context, node, lane, decorated)
    }

    pub(super) fn render_page_flow_sequence(
        &self,
        context: PageFlowRenderContext<'_>,
        sequence: &PageFlowSequence,
        area: PageFlowRenderArea,
        cx: &mut App,
    ) -> AnyElement {
        columns::render_page_flow_sequence(self, context, sequence, area, cx)
    }
}
