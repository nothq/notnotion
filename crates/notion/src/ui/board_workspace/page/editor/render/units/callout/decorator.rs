use super::super::super::super::{
    AnyElement, App, Arc, CardPageBlock, ElementId, LoadedCardPageData, PageBlockDragScrollTarget,
    PageBlockDragSelection,
};
use super::super::super::row::PageBlockRenderContext;
use super::super::super::PageBlockRenderer;
use super::layout::{
    page_callout_gutter_center, page_flow_decorator_segment_layout, PageCalloutSegmentLayout,
};
use super::{PageCalloutSegmentFrame, PageCalloutSegmentInteraction, PageCalloutSegmentRow};

pub(in crate::ui::board_workspace::page::editor::render) struct PageFlowDecoratorLayerRender<'a> {
    pub(in crate::ui::board_workspace::page::editor::render) data: &'a Arc<LoadedCardPageData>,
    pub(in crate::ui::board_workspace::page::editor::render) node_id: &'a crate::ui::PageFlowNodeId,
    pub(in crate::ui::board_workspace::page::editor::render) layer_index: usize,
    pub(in crate::ui::board_workspace::page::editor::render) layer:
        &'a crate::ui::PageFlowDecoratorLayer,
    pub(in crate::ui::board_workspace::page::editor::render) drag_selection:
        &'a Arc<PageBlockDragSelection>,
    pub(in crate::ui::board_workspace::page::editor::render) scroll_target:
        PageBlockDragScrollTarget,
    pub(in crate::ui::board_workspace::page::editor::render) observation:
        &'a crate::ui::surface::PageFlowObservationToken,
    pub(in crate::ui::board_workspace::page::editor::render) content: AnyElement,
}

impl PageBlockRenderer {
    pub(in crate::ui::board_workspace::page::editor::render) fn render_page_flow_decorator_layer(
        &self,
        render: PageFlowDecoratorLayerRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let row = &render.data.visible_rows[render.layer.visible_row_index];
        let block = &render.data.page.blocks[row.block_index];
        assert_eq!(
            block.block_id.as_str(),
            render.layer.callout_block_id.as_ref()
        );
        let layout = page_flow_decorator_segment_layout(render.layer);
        let interaction = self.page_flow_decorator_interaction(&render, block);
        let context = page_flow_decorator_context(&render, block, &layout, &interaction);
        let frame = self.render_page_callout_segment_frame(
            PageCalloutSegmentFrame {
                page_id: &render.data.page.block_id,
                color: render
                    .data
                    .effective_callout_color(render.layer.visible_row_index),
                presentation: render.layer.presentation,
                content: render.content,
                block,
                interaction: &interaction,
            },
            cx,
        );
        let body = self.render_page_callout_segment_body(&context, interaction.first, frame, cx);
        self.render_page_callout_segment_row(
            PageCalloutSegmentRow {
                element_id: flow_decorator_element_id(render.node_id, render.layer_index, block),
                body,
            },
            &context,
            &interaction,
            cx,
        )
    }
}

impl PageBlockRenderer {
    fn page_flow_decorator_interaction(
        &self,
        render: &PageFlowDecoratorLayerRender<'_>,
        block: &CardPageBlock,
    ) -> PageCalloutSegmentInteraction {
        let first = render.layer.presentation.segment.is_first();
        let hovered_inside = self
            .interaction
            .hovered_block
            .as_deref()
            .is_some_and(|block_id| {
                block_id == block.block_id
                    || render
                        .data
                        .block_is_within_callout(block_id, render.layer.visible_row_index)
            });
        let selected = self.interaction.block_selected(&block.block_id);
        PageCalloutSegmentInteraction {
            first,
            selected_without_drag: selected && !self.interaction.active_drag,
            show_controls: first
                && (hovered_inside || self.interaction.block_menu_is_open(&block.block_id)),
        }
    }
}

fn page_flow_decorator_context<'a>(
    render: &PageFlowDecoratorLayerRender<'a>,
    block: &'a CardPageBlock,
    layout: &PageCalloutSegmentLayout,
    interaction: &PageCalloutSegmentInteraction,
) -> PageBlockRenderContext<'a> {
    PageBlockRenderContext {
        data: render.data,
        index: render.data.visible_rows[render.layer.visible_row_index].block_index,
        block,
        drag_selection: render.drag_selection,
        scroll_target: render.scroll_target,
        inherited_text_color: render
            .data
            .inherited_text_color(render.layer.visible_row_index),
        render_depth: layout.render_depth,
        nesting_offset: layout.nesting_offset,
        row_spacing: Some(layout.spacing),
        render_gutter: true,
        gutter_center: interaction
            .first
            .then(|| page_callout_gutter_center(block, layout.spacing, render.data.page.format)),
        observation: render.observation,
    }
}

fn flow_decorator_element_id(
    node_id: &crate::ui::PageFlowNodeId,
    layer_index: usize,
    block: &CardPageBlock,
) -> ElementId {
    ElementId::Name(
        format!(
            "notion-page-flow-callout-segment-{}-{layer_index}-{node_id:?}",
            block.block_id
        )
        .into(),
    )
}
