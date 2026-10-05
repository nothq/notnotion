const CALLOUT_ROW_SPACING: f32 = 8.0;
const CALLOUT_OUTER_HORIZONTAL_INSET: f32 = 8.0;
const CALLOUT_FRAME_HORIZONTAL_INSET: f32 = 12.0;
const CALLOUT_FRAME_VERTICAL_INSET: f32 = 12.0;
const CALLOUT_BORDER_EXTENT: f32 = 1.0;
const CALLOUT_MARKER_EXTENT: f32 = 24.0;
const CALLOUT_MARKER_TOP_INSET: f32 = 7.5;
const CALLOUT_SINGLE_MINIMUM_EXTENT: f32 = 66.0;
const CALLOUT_CORNER_RADIUS: f32 = 10.0;
pub(super) const ROOT_LEADING_INSET: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PageFlowExtentEnvelope {
    pub(crate) additive_insets: f32,
    pub(crate) minimum_extent: f32,
}

impl PageFlowExtentEnvelope {
    pub(crate) fn new(additive_insets: f32, minimum_extent: f32) -> Self {
        assert!(additive_insets.is_finite() && additive_insets >= 0.0);
        assert!(minimum_extent.is_finite() && minimum_extent >= 0.0);
        Self {
            additive_insets,
            minimum_extent,
        }
    }

    pub(crate) fn apply(self, child_extent: f32) -> f32 {
        assert!(child_extent.is_finite() && child_extent >= 0.0);
        self.minimum_extent.max(child_extent + self.additive_insets)
    }

    pub(crate) fn wrapped_by(self, outer: Self) -> Self {
        Self::new(
            self.additive_insets + outer.additive_insets,
            outer
                .minimum_extent
                .max(self.minimum_extent + outer.additive_insets),
        )
    }
}

impl Default for PageFlowExtentEnvelope {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageFlowCalloutSegment {
    Single,
    First,
    Middle,
    Last,
}

impl PageFlowCalloutSegment {
    pub(crate) const fn is_first(self) -> bool {
        matches!(self, Self::Single | Self::First)
    }

    pub(crate) const fn is_last(self) -> bool {
        matches!(self, Self::Single | Self::Last)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PageFlowCalloutPresentationSpec {
    pub(crate) segment: PageFlowCalloutSegment,
    pub(crate) row_spacing: f32,
    pub(crate) outer_horizontal_inset: f32,
    pub(crate) frame_horizontal_inset: f32,
    pub(crate) frame_vertical_inset: f32,
    pub(crate) border_extent: f32,
    pub(crate) marker_extent: f32,
    pub(crate) marker_top_inset: f32,
    pub(crate) single_minimum_extent: f32,
    pub(crate) corner_radius: f32,
}

impl PageFlowCalloutPresentationSpec {
    pub(crate) fn notion(segment: PageFlowCalloutSegment) -> Self {
        Self {
            segment,
            row_spacing: CALLOUT_ROW_SPACING,
            outer_horizontal_inset: CALLOUT_OUTER_HORIZONTAL_INSET,
            frame_horizontal_inset: CALLOUT_FRAME_HORIZONTAL_INSET,
            frame_vertical_inset: CALLOUT_FRAME_VERTICAL_INSET,
            border_extent: CALLOUT_BORDER_EXTENT,
            marker_extent: CALLOUT_MARKER_EXTENT,
            marker_top_inset: CALLOUT_MARKER_TOP_INSET,
            single_minimum_extent: CALLOUT_SINGLE_MINIMUM_EXTENT,
            corner_radius: CALLOUT_CORNER_RADIUS,
        }
    }

    pub(crate) fn extent_envelope(self) -> PageFlowExtentEnvelope {
        let ends = usize::from(self.segment.is_first()) + usize::from(self.segment.is_last());
        let row_insets = ends as f32 * self.row_spacing;
        let frame_insets = ends as f32 * (self.frame_vertical_inset + self.border_extent);
        let marker_minimum = if self.segment.is_first() {
            self.marker_top_inset + self.marker_extent + frame_insets
        } else {
            0.0
        };
        let frame_minimum = if self.segment == PageFlowCalloutSegment::Single {
            self.single_minimum_extent
        } else {
            0.0
        }
        .max(marker_minimum);
        PageFlowExtentEnvelope::new(row_insets + frame_insets, row_insets + frame_minimum)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PageFlowColumnsPresentationSpec {
    pub(crate) lane_gap: f32,
    pub(crate) lane_minimum: f32,
    pub(crate) top_inset: f32,
    pub(crate) bottom_inset: f32,
}

impl PageFlowColumnsPresentationSpec {
    pub(crate) const NOTION: Self = Self {
        lane_gap: 46.0,
        lane_minimum: 28.0,
        top_inset: 12.0,
        bottom_inset: 12.0,
    };

    pub(crate) fn extent_envelope(self) -> PageFlowExtentEnvelope {
        let additive = self.top_inset + self.bottom_inset;
        PageFlowExtentEnvelope::new(additive, additive + self.lane_minimum)
    }
}
