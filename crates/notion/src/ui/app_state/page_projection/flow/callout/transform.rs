use super::super::super::{LoadedCardPageVisibleRow, PageVisibleRowLayouts, PageVisibleRowSpacing};
use super::{PageFlowCalloutPresentationSpec, PageFlowDecoratorLayer};

pub(crate) const PAGE_FLOW_BLOCK_INDENT: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PageFlowDecoratorLayerLayout {
    pub(crate) render_depth: usize,
    pub(crate) nesting_offset: f32,
    pub(crate) spacing: PageVisibleRowSpacing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PageFlowDecoratorContentOriginTransform {
    block_offset: f32,
    inline_start: f32,
    inline_end: f32,
}

impl PageFlowDecoratorLayerLayout {
    pub(super) fn from_row(
        rows: &[LoadedCardPageVisibleRow],
        layouts: &PageVisibleRowLayouts,
        row_index: usize,
    ) -> Self {
        let row = rows[row_index];
        let (render_depth, nesting_offset) = row.callout_parent_row_index.map_or_else(
            || (row.visual_depth, layouts.nesting_offset(row_index)),
            |parent| {
                let parent_row = rows[parent];
                let parent_spacing = layouts
                    .spacing(parent)
                    .expect("visible Callout parents must retain editable row spacing");
                let origin = layouts.nesting_offset(parent) + parent_spacing.bottom();
                (
                    row.visual_depth
                        .saturating_sub(parent_row.visual_depth.saturating_add(1)),
                    (layouts.nesting_offset(row_index) - origin).max(0.0),
                )
            },
        );
        let spacing = layouts
            .spacing(row_index)
            .expect("visible Callout decorators must retain editable row spacing");
        Self {
            render_depth,
            nesting_offset,
            spacing,
        }
    }
}

impl PageFlowDecoratorContentOriginTransform {
    pub(super) fn from_layers(layers: &[PageFlowDecoratorLayer], root_leading: f32) -> Self {
        assert!(root_leading.is_finite() && root_leading >= 0.0);
        layers.iter().fold(
            Self {
                block_offset: root_leading,
                ..Self::default()
            },
            Self::wrapped_by,
        )
    }

    pub(crate) fn block_offset(self) -> f32 {
        self.block_offset
    }

    pub(crate) fn horizontal_insets(self) -> f32 {
        self.inline_start + self.inline_end
    }

    fn wrapped_by(mut self, layer: &PageFlowDecoratorLayer) -> Self {
        let presentation = layer.presentation;
        let layout = layer.layout;
        self.block_offset += first_block_inset(presentation) - layout.nesting_offset;
        self.inline_start += layout.render_depth as f32 * PAGE_FLOW_BLOCK_INDENT
            + presentation.outer_horizontal_inset
            + presentation.border_extent
            + presentation.frame_horizontal_inset
            + presentation.marker_extent;
        self.inline_end += presentation.outer_horizontal_inset
            + presentation.border_extent
            + presentation.frame_horizontal_inset;
        assert!(self.block_offset.is_finite());
        assert!(self.inline_start.is_finite() && self.inline_start >= 0.0);
        assert!(self.inline_end.is_finite() && self.inline_end >= 0.0);
        self
    }
}

fn first_block_inset(presentation: PageFlowCalloutPresentationSpec) -> f32 {
    if presentation.segment.is_first() {
        presentation.row_spacing + presentation.border_extent + presentation.frame_vertical_inset
    } else {
        0.0
    }
}
