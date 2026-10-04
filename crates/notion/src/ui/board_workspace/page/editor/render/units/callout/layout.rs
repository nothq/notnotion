use std::ops::Range;

use super::super::super::super::support::{
    page_block_editable_input_spec, page_block_first_line_center, page_block_visible_row_spacing,
    PageBlockRowSpacing, PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT,
};
use super::super::super::super::{CardPageBlock, CardPageBlockKind, LoadedCardPageData};
use crate::model::CardPageFormat;
use crate::ui::{PageFlowCalloutSegment, PageFlowDecoratorLayer};

pub(super) struct PageCalloutSegmentLayout {
    pub(super) render_depth: usize,
    pub(super) nesting_offset: f32,
    pub(super) spacing: PageBlockRowSpacing,
}

pub(super) fn page_callout_segment_position(
    range: &Range<usize>,
    document_unit_index: usize,
) -> PageFlowCalloutSegment {
    assert!(
        range.contains(&document_unit_index),
        "Callout segment render unit must belong to its Callout subtree"
    );
    match (
        document_unit_index == range.start,
        document_unit_index + 1 == range.end,
    ) {
        (true, true) => PageFlowCalloutSegment::Single,
        (true, false) => PageFlowCalloutSegment::First,
        (false, true) => PageFlowCalloutSegment::Last,
        (false, false) => PageFlowCalloutSegment::Middle,
    }
}

pub(super) fn page_callout_segment_layout(
    data: &LoadedCardPageData,
    visible_row_index: usize,
    parent_callout_row_index: Option<usize>,
    nesting_offsets: &[f32],
) -> PageCalloutSegmentLayout {
    let row = &data.visible_rows[visible_row_index];
    let (render_depth, nesting_offset) = parent_callout_row_index.map_or_else(
        || (row.visual_depth, nesting_offsets[visible_row_index]),
        |parent_row_index| {
            let parent_row = &data.visible_rows[parent_row_index];
            let nesting_origin = nesting_offsets[parent_row_index]
                + page_block_visible_row_spacing(data, parent_row_index).bottom;
            (
                row.visual_depth
                    .saturating_sub(parent_row.visual_depth.saturating_add(1)),
                (nesting_offsets[visible_row_index] - nesting_origin).max(0.0),
            )
        },
    );
    PageCalloutSegmentLayout {
        render_depth,
        nesting_offset,
        spacing: page_block_visible_row_spacing(data, visible_row_index),
    }
}

pub(super) fn page_flow_decorator_segment_layout(
    layer: &PageFlowDecoratorLayer,
) -> PageCalloutSegmentLayout {
    PageCalloutSegmentLayout {
        render_depth: layer.layout.render_depth,
        nesting_offset: layer.layout.nesting_offset,
        spacing: PageBlockRowSpacing {
            top: layer.layout.spacing.top(),
            bottom: layer.layout.spacing.bottom(),
        },
    }
}

pub(super) fn page_callout_gutter_center(
    block: &CardPageBlock,
    spacing: PageBlockRowSpacing,
    format: CardPageFormat,
) -> f32 {
    let base = block
        .editable_content()
        .map_or(PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT / 2.0, |editable| {
            page_block_first_line_center(spacing, page_block_editable_input_spec(editable, format))
        });
    let frame_offset = if block
        .editable_content()
        .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
    {
        19.0
    } else {
        13.0
    };
    base + frame_offset
}
