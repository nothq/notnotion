use super::super::support::{page_block_flow_row_spacing, page_block_visible_row_spacing};
use super::super::{
    AnyElement, App, Arc, CardPageBlockColorValue, LoadedCardPageData, PageBlockDragScrollTarget,
    PageBlockDragSelection,
};
use super::row::PageBlockRenderContext;
use super::PageBlockRenderer;
use crate::ui::{
    LoadedCardPageDocumentUnit, PageFlowPaintedCalloutPrefix, PageFlowSection,
    PageFlowUnitAdjacency,
};

mod callout;
mod table;

use callout::PageCalloutDocumentUnitRender;
pub(super) use callout::PageFlowDecoratorLayerRender;
use table::PageSimpleTableRowRender;

/// How a document unit sits in its row: the text color it inherits, its indent
/// depth and nesting offset, and whether it shows a block gutter.
#[derive(Clone, Copy)]
pub(super) struct PageDocumentUnitPlacement {
    pub(super) inherited_text_color: Option<CardPageBlockColorValue>,
    pub(super) render_depth: usize,
    pub(super) nesting_offset: f32,
    pub(super) render_gutter: bool,
}

pub(super) struct PageDocumentUnitRender<'a> {
    data: &'a Arc<LoadedCardPageData>,
    document_unit_index: usize,
    drag_selection: &'a Arc<PageBlockDragSelection>,
    scroll_target: PageBlockDragScrollTarget,
    nesting_offsets: &'a [f32],
    decoration_scope: PageDocumentUnitDecorationScope,
    flow_observation: &'a crate::ui::surface::PageFlowObservationToken,
}

pub(super) struct PageFlowSectionRowsRender<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) section: &'a PageFlowSection,
    pub(super) nesting_offsets: &'a [f32],
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) observation: &'a crate::ui::surface::PageFlowObservationToken,
}

pub(super) struct PageBlockSectionRowsRender<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) document_unit_indices: Vec<usize>,
    pub(super) nesting_offsets: &'a [f32],
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) observation: &'a crate::ui::surface::PageFlowObservationToken,
}

#[derive(Clone)]
pub(super) enum PageDocumentUnitDecorationScope {
    ExistingLinear,
    FlowNode {
        painted_callout_prefix: PageFlowPaintedCalloutPrefix,
        adjacency: PageFlowUnitAdjacency,
    },
}

impl PageBlockRenderer {
    pub(super) fn render_page_block_section_rows(
        &self,
        render: PageBlockSectionRowsRender<'_>,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        render
            .document_unit_indices
            .into_iter()
            .map(|document_unit_index| {
                self.render_page_document_unit(
                    PageDocumentUnitRender {
                        data: render.data,
                        document_unit_index,
                        drag_selection: render.drag_selection,
                        scroll_target: render.scroll_target,
                        nesting_offsets: render.nesting_offsets,
                        decoration_scope: PageDocumentUnitDecorationScope::ExistingLinear,
                        flow_observation: render.observation,
                    },
                    cx,
                )
            })
            .collect()
    }

    pub(super) fn render_page_flow_section_rows(
        &self,
        render: PageFlowSectionRowsRender<'_>,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        let unit_indices = render.section.document_unit_range.clone();
        assert_eq!(unit_indices.len(), render.section.unit_adjacencies.len());
        let painted_callout_prefix = render.section.painted_callout_prefix();
        unit_indices
            .zip(render.section.unit_adjacencies.iter().copied())
            .map(|(document_unit_index, adjacency)| {
                self.render_page_document_unit(
                    PageDocumentUnitRender {
                        data: render.data,
                        document_unit_index,
                        drag_selection: render.drag_selection,
                        scroll_target: render.scroll_target,
                        nesting_offsets: render.nesting_offsets,
                        decoration_scope: PageDocumentUnitDecorationScope::FlowNode {
                            painted_callout_prefix: painted_callout_prefix.clone(),
                            adjacency,
                        },
                        flow_observation: render.observation,
                    },
                    cx,
                )
            })
            .collect()
    }

    fn render_page_document_unit(
        &self,
        render: PageDocumentUnitRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let visible_row_index =
            render.data.document_units[render.document_unit_index].owner_visible_row_index();
        let callout_path = render.data.callout_layer_row_indices(visible_row_index);
        if let PageDocumentUnitDecorationScope::FlowNode {
            painted_callout_prefix,
            ..
        } = &render.decoration_scope
        {
            painted_callout_prefix.assert_matches_visible_rows(callout_path);
        }
        if !callout_path.is_empty() {
            if matches!(
                &render.decoration_scope,
                PageDocumentUnitDecorationScope::FlowNode { .. }
            ) {
                return self.render_page_callout_document_content(
                    &PageCalloutDocumentUnitRender::new(&render, visible_row_index, callout_path),
                    cx,
                );
            }
            return self.render_page_callout_document_unit(
                PageCalloutDocumentUnitRender::new(&render, visible_row_index, callout_path),
                cx,
            );
        }
        let row = &render.data.visible_rows[visible_row_index];
        self.render_page_document_unit_content(
            &render,
            PageDocumentUnitPlacement {
                inherited_text_color: None,
                render_depth: row.visual_depth,
                nesting_offset: render.nesting_offsets[visible_row_index],
                render_gutter: true,
            },
            cx,
        )
    }

    pub(super) fn render_page_document_unit_content(
        &self,
        render: &PageDocumentUnitRender<'_>,
        placement: PageDocumentUnitPlacement,
        cx: &mut App,
    ) -> AnyElement {
        let PageDocumentUnitPlacement {
            inherited_text_color,
            render_depth,
            nesting_offset,
            render_gutter,
        } = placement;
        let unit = &render.data.document_units[render.document_unit_index];
        let visible_row_index = unit.owner_visible_row_index();
        let row = &render.data.visible_rows[visible_row_index];
        let block = &render.data.page.blocks[row.block_index];
        match unit {
            LoadedCardPageDocumentUnit::Block { .. } => self.render_page_block(
                PageBlockRenderContext {
                    data: render.data,
                    index: row.block_index,
                    block,
                    drag_selection: render.drag_selection,
                    scroll_target: render.scroll_target,
                    inherited_text_color,
                    render_depth,
                    nesting_offset,
                    row_spacing: block.editable_content().map(|_| {
                        render
                            .decoration_scope
                            .row_spacing(render.data, visible_row_index)
                    }),
                    render_gutter,
                    gutter_center: None,
                    observation: render.flow_observation,
                },
                cx,
            ),
            LoadedCardPageDocumentUnit::SimpleTableRow { .. } => self.render_page_simple_table_row(
                PageSimpleTableRowRender {
                    data: render.data,
                    document_unit_index: render.document_unit_index,
                    drag_selection: render.drag_selection,
                    scroll_target: render.scroll_target,
                    inherited_text_color,
                    render_depth,
                    nesting_offset,
                    render_gutter,
                    flow_observation: Some(render.flow_observation),
                },
                cx,
            ),
        }
    }
}

impl PageDocumentUnitDecorationScope {
    fn row_spacing(
        &self,
        data: &LoadedCardPageData,
        visible_row_index: usize,
    ) -> super::super::support::PageBlockRowSpacing {
        match self {
            Self::ExistingLinear => page_block_visible_row_spacing(data, visible_row_index),
            Self::FlowNode { adjacency, .. } => {
                page_block_flow_row_spacing(data, visible_row_index, *adjacency)
            }
        }
    }
}
