use crate::ui::{div, px, AnyElement, FluentBuilder, PageFlowNode, ParentElement, Styled};
use gpui::{App, IntoElement};

use super::super::super::units::PageFlowDecoratorLayerRender;
use super::{PageBlockRenderer, PageFlowRenderContext};

pub(super) fn render_page_flow_decorated_node(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    node: &PageFlowNode,
    mut content: AnyElement,
    cx: &mut App,
) -> AnyElement {
    let node_id = &node.node_id();
    let plan = node.decorator_plan();
    for (layer_index, layer) in plan.layers.iter().enumerate().rev() {
        content = renderer.render_page_flow_decorator_layer(
            PageFlowDecoratorLayerRender {
                data: context.data,
                node_id,
                layer_index,
                layer,
                drag_selection: context.drag_selection,
                scroll_target: context.scroll_target,
                observation: context.observation,
                content,
            },
            cx,
        );
    }
    let leading = plan.root_leading_inset();
    div()
        .w_full()
        .min_w(px(0.0))
        .when(leading > 0.0, |node| node.pt(px(leading)))
        .child(content)
        .into_any_element()
}
