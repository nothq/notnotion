use super::super::super::input::PageBlockInputContent;
use super::super::super::support::page_block_visible_row_spacing;
use super::super::super::{
    div, px, AnyElement, App, Arc, CardPageBlock, CardPageBlockColor, CardPageBlockKind, ElementId,
    FluentBuilder, InteractiveElement, IntoElement, LoadedCardPageData, PageBlockDragScrollTarget,
    PageBlockDragSelection, ParentElement, Styled, PAGE_BLOCK_GUTTER_WIDTH, PAGE_BLOCK_INDENT,
};
use super::super::content::{render_page_callout_segment_content, PageBlockVisualStyle};
use super::super::row::PageBlockRenderContext;
use super::super::PageBlockRenderer;

mod decorator;
mod layout;

use super::{PageDocumentUnitDecorationScope, PageDocumentUnitPlacement, PageDocumentUnitRender};
pub(in crate::ui::board_workspace::page::editor::render) use decorator::PageFlowDecoratorLayerRender;
use layout::{
    page_callout_gutter_center, page_callout_segment_layout, page_callout_segment_position,
};

pub(super) struct PageCalloutDocumentUnitRender<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) document_unit_index: usize,
    pub(super) visible_row_index: usize,
    pub(super) callout_path: &'a [usize],
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) nesting_offsets: &'a [f32],
    pub(super) decoration_scope: PageDocumentUnitDecorationScope,
    pub(super) flow_observation: &'a crate::ui::surface::PageFlowObservationToken,
}

impl<'a> PageCalloutDocumentUnitRender<'a> {
    pub(super) fn new(
        render: &PageDocumentUnitRender<'a>,
        visible_row_index: usize,
        callout_path: &'a [usize],
    ) -> Self {
        Self {
            data: render.data,
            document_unit_index: render.document_unit_index,
            visible_row_index,
            callout_path,
            drag_selection: render.drag_selection,
            scroll_target: render.scroll_target,
            nesting_offsets: render.nesting_offsets,
            decoration_scope: render.decoration_scope.clone(),
            flow_observation: render.flow_observation,
        }
    }
}

struct PageCalloutSegmentRender<'a> {
    data: &'a Arc<LoadedCardPageData>,
    element_id: ElementId,
    document_visible_row_index: usize,
    visible_row_index: usize,
    parent_callout_row_index: Option<usize>,
    color: CardPageBlockColor,
    presentation: crate::ui::PageFlowCalloutPresentationSpec,
    drag_selection: &'a Arc<PageBlockDragSelection>,
    scroll_target: PageBlockDragScrollTarget,
    nesting_offsets: &'a [f32],
    observation: &'a crate::ui::surface::PageFlowObservationToken,
    content: AnyElement,
}

/// A callout segment row's element id and the body it wraps.
struct PageCalloutSegmentRow {
    element_id: ElementId,
    body: gpui::Div,
}

struct PageCalloutSegmentInteraction {
    first: bool,
    selected_without_drag: bool,
    show_controls: bool,
}

struct PageCalloutSegmentFrame<'a> {
    page_id: &'a str,
    color: CardPageBlockColor,
    presentation: crate::ui::PageFlowCalloutPresentationSpec,
    content: AnyElement,
    block: &'a CardPageBlock,
    interaction: &'a PageCalloutSegmentInteraction,
}

impl PageBlockRenderer {
    pub(super) fn render_page_callout_document_unit(
        &self,
        render: PageCalloutDocumentUnitRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let mut content = self.render_page_callout_document_content(&render, cx);
        for (layer_index, callout_row_index) in
            render.callout_path.iter().copied().enumerate().rev()
        {
            let range = render.data.callout_render_unit_range(callout_row_index);
            content = self.render_page_callout_segment(
                PageCalloutSegmentRender {
                    data: render.data,
                    element_id: ElementId::Name(
                        format!(
                            "notion-page-callout-segment-{}-{}",
                            render.data.page.blocks
                                [render.data.visible_rows[callout_row_index].block_index]
                                .block_id,
                            render.document_unit_index
                        )
                        .into(),
                    ),
                    document_visible_row_index: render.visible_row_index,
                    visible_row_index: callout_row_index,
                    parent_callout_row_index: layer_index
                        .checked_sub(1)
                        .map(|index| render.callout_path[index]),
                    color: render.data.effective_callout_color(callout_row_index),
                    presentation: crate::ui::PageFlowCalloutPresentationSpec::notion(
                        page_callout_segment_position(&range, render.document_unit_index),
                    ),
                    drag_selection: render.drag_selection,
                    scroll_target: render.scroll_target,
                    nesting_offsets: render.nesting_offsets,
                    observation: render.flow_observation,
                    content,
                },
                cx,
            );
        }
        content
    }

    pub(super) fn render_page_callout_document_content(
        &self,
        render: &PageCalloutDocumentUnitRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let row = &render.data.visible_rows[render.visible_row_index];
        let block = &render.data.page.blocks[row.block_index];
        if block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
        {
            return self.render_page_callout_input_content(render, block, cx);
        }
        self.render_nested_page_callout_block(render, cx)
    }

    fn render_page_callout_input_content(
        &self,
        render: &PageCalloutDocumentUnitRender<'_>,
        block: &CardPageBlock,
        cx: &mut App,
    ) -> AnyElement {
        let editable = block
            .editable_content()
            .expect("rendered Callout input rows must be editable");
        assert!(
            render.data.block_has_text_input(&block.block_id),
            "container-only Callout rows must not become document render units"
        );
        let color = render
            .data
            .effective_callout_color(render.visible_row_index);
        let input = self.page_block_input_entity(
            render.data,
            PageBlockInputContent {
                block_id: &block.block_id,
                editable,
                color,
            },
            cx,
        );
        div()
            .w_full()
            .px(px(6.0))
            .py(px(6.0))
            .child(input)
            .into_any_element()
    }

    fn render_nested_page_callout_block(
        &self,
        render: &PageCalloutDocumentUnitRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let row = &render.data.visible_rows[render.visible_row_index];
        let innermost_row_index = *render
            .callout_path
            .last()
            .expect("Callout document units require at least one layer");
        let innermost_row = &render.data.visible_rows[innermost_row_index];
        let nesting_origin = render.nesting_offsets[innermost_row_index]
            + page_block_visible_row_spacing(render.data, innermost_row_index).bottom;
        self.render_page_document_unit_content(
            &PageDocumentUnitRender {
                data: render.data,
                document_unit_index: render.document_unit_index,
                drag_selection: render.drag_selection,
                scroll_target: render.scroll_target,
                nesting_offsets: render.nesting_offsets,
                decoration_scope: render.decoration_scope.clone(),
                flow_observation: render.flow_observation,
            },
            PageDocumentUnitPlacement {
                inherited_text_color: render.data.inherited_text_color(render.visible_row_index),
                render_depth: row
                    .visual_depth
                    .saturating_sub(innermost_row.visual_depth.saturating_add(1)),
                nesting_offset: (render.nesting_offsets[render.visible_row_index] - nesting_origin)
                    .max(0.0),
                render_gutter: render
                    .data
                    .callout_first_text_input_row_index(innermost_row_index)
                    != Some(render.visible_row_index),
            },
            cx,
        )
    }

    fn render_page_callout_segment(
        &self,
        render: PageCalloutSegmentRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let row = &render.data.visible_rows[render.visible_row_index];
        let block = &render.data.page.blocks[row.block_index];
        let layout = page_callout_segment_layout(
            render.data,
            render.visible_row_index,
            render.parent_callout_row_index,
            render.nesting_offsets,
        );
        let interaction = self.page_callout_segment_interaction(&render, block);
        let context = PageBlockRenderContext {
            data: render.data,
            index: row.block_index,
            block,
            drag_selection: render.drag_selection,
            scroll_target: render.scroll_target,
            inherited_text_color: render.data.inherited_text_color(render.visible_row_index),
            render_depth: layout.render_depth,
            nesting_offset: layout.nesting_offset,
            row_spacing: Some(layout.spacing),
            render_gutter: true,
            gutter_center: interaction.first.then(|| {
                page_callout_gutter_center(block, layout.spacing, render.data.page.format)
            }),
            observation: render.observation,
        };
        let element_id = render.element_id;
        let frame = self.render_page_callout_segment_frame(
            PageCalloutSegmentFrame {
                page_id: &render.data.page.block_id,
                color: render.color,
                presentation: render.presentation,
                content: render.content,
                block,
                interaction: &interaction,
            },
            cx,
        );
        let body = self.render_page_callout_segment_body(&context, interaction.first, frame, cx);
        self.render_page_callout_segment_row(
            PageCalloutSegmentRow { element_id, body },
            &context,
            &interaction,
            cx,
        )
    }

    fn render_page_callout_segment_frame(
        &self,
        frame: PageCalloutSegmentFrame<'_>,
        cx: &mut App,
    ) -> gpui::Div {
        let marker = frame
            .interaction
            .first
            .then(|| self.render_page_callout_icon(frame.page_id, frame.block, cx));
        render_page_callout_segment_content(
            PageBlockVisualStyle::new(self.theme, self.appearance_mode, frame.color)
                .with_selection(frame.interaction.selected_without_drag),
            frame.presentation,
            frame.content,
            marker,
            frame.interaction.first.then(|| {
                ElementId::Name(format!("notion-callout-note-{}", frame.block.block_id).into())
            }),
        )
    }

    fn render_page_callout_segment_body(
        &self,
        context: &PageBlockRenderContext<'_>,
        first: bool,
        frame: gpui::Div,
        cx: &mut App,
    ) -> gpui::Div {
        div()
            .relative()
            .ml(px(PAGE_BLOCK_GUTTER_WIDTH))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .child(frame)
            .when(
                first
                    && self
                        .interaction
                        .slash_menu
                        .as_ref()
                        .is_some_and(|menu| menu.block_id == context.block.block_id),
                |body| {
                    body.child(self.render_native_page_slash_menu(
                        context.block,
                        context.data.page.format,
                        cx,
                    ))
                },
            )
            .when(
                first && self.interaction.block_menu_is_open(&context.block.block_id),
                |body| {
                    body.child(self.render_page_block_context_menu(
                        context.block,
                        context.first_line_center(),
                        cx,
                    ))
                },
            )
    }

    fn render_page_callout_segment_row(
        &self,
        row: PageCalloutSegmentRow,
        context: &PageBlockRenderContext<'_>,
        interaction: &PageCalloutSegmentInteraction,
        cx: &mut App,
    ) -> AnyElement {
        let PageCalloutSegmentRow { element_id, body } = row;
        div()
            .id(element_id)
            .relative()
            .flex()
            .top(px(-context.nesting_offset))
            .ml(px(
                context.render_depth as f32 * PAGE_BLOCK_INDENT - PAGE_BLOCK_GUTTER_WIDTH
            ))
            .when(interaction.first, |segment| {
                segment.child(self.render_page_block_gutter(context, interaction.show_controls, cx))
            })
            .child(body)
            .into_any_element()
    }
}

impl PageBlockRenderer {
    fn page_callout_segment_interaction(
        &self,
        render: &PageCalloutSegmentRender<'_>,
        block: &CardPageBlock,
    ) -> PageCalloutSegmentInteraction {
        let first = render.presentation.segment.is_first();
        let document_block_id = &render.data.page.blocks
            [render.data.visible_rows[render.document_visible_row_index].block_index]
            .block_id;
        let hovered_block_id = self.interaction.hovered_block.as_deref();
        let hovered = hovered_block_id == Some(block.block_id.as_str())
            || (first && hovered_block_id == Some(document_block_id.as_str()));
        let selected = self.interaction.block_selected(&block.block_id);
        PageCalloutSegmentInteraction {
            first,
            selected_without_drag: selected && !self.interaction.active_drag,
            show_controls: first
                && (hovered || self.interaction.block_menu_is_open(&block.block_id)),
        }
    }
}
