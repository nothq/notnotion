use std::sync::Arc;

use crate::{model::CardPageColumnEffectiveShare, ui::PageFlowColumnsPresentationSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageFlowLayoutWidth(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageFlowExactLayoutWidth(PageFlowLayoutWidth);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageFlowSequenceWidth {
    Provisional(PageFlowLayoutWidth),
    Exact(PageFlowExactLayoutWidth),
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowLaneWidths(Arc<[PageFlowLayoutWidth]>);

impl PageFlowLayoutWidth {
    pub(crate) fn from_estimated_pixels(pixels: f32) -> Self {
        Self::from_pixels(pixels)
    }

    fn from_pixels(pixels: f32) -> Self {
        assert!(pixels.is_finite() && pixels >= 0.0);
        Self(if pixels == 0.0 {
            0.0_f32.to_bits()
        } else {
            pixels.to_bits()
        })
    }

    pub(crate) fn pixels(self) -> f32 {
        f32::from_bits(self.0)
    }
}

impl PageFlowExactLayoutWidth {
    pub(crate) fn from_observed_content_box(pixels: f32) -> Self {
        Self(PageFlowLayoutWidth::from_pixels(pixels))
    }

    pub(crate) const fn layout_width(self) -> PageFlowLayoutWidth {
        self.0
    }
}

impl PageFlowSequenceWidth {
    pub(crate) const fn layout_width(self) -> PageFlowLayoutWidth {
        match self {
            Self::Provisional(width) => width,
            Self::Exact(width) => width.layout_width(),
        }
    }

    pub(crate) const fn exact_width(self) -> Option<PageFlowExactLayoutWidth> {
        match self {
            Self::Provisional(_) => None,
            Self::Exact(width) => Some(width),
        }
    }

    pub(crate) const fn into_provisional(self) -> Self {
        Self::Provisional(self.layout_width())
    }

    pub(crate) fn with_retained_exact(self, retained: Option<Self>) -> Self {
        match (self, retained) {
            (Self::Provisional(width), Some(Self::Exact(exact)))
                if exact.layout_width() == width =>
            {
                Self::Exact(exact)
            }
            _ => self,
        }
    }
}

impl PageFlowLaneWidths {
    pub(crate) fn as_slice(&self) -> &[PageFlowLayoutWidth] {
        &self.0
    }
}

pub(crate) fn allocate_page_flow_lane_widths(
    content_width: PageFlowLayoutWidth,
    shares: &[CardPageColumnEffectiveShare],
) -> PageFlowLaneWidths {
    if shares.is_empty() {
        return PageFlowLaneWidths(Arc::<[PageFlowLayoutWidth]>::from([]));
    }
    let gap = PageFlowColumnsPresentationSpec::NOTION.lane_gap;
    let total_gap = gap * shares.len().saturating_sub(1) as f32;
    let gap_free = (content_width.pixels() - total_gap).max(0.0);
    let mut remaining = gap_free;
    let last = shares.len() - 1;
    let mut widths = Vec::with_capacity(shares.len());
    for share in &shares[..last] {
        let requested = (f64::from(gap_free) * share.fraction()) as f32;
        let width = requested.clamp(0.0, remaining);
        widths.push(PageFlowLayoutWidth::from_pixels(width));
        remaining = (remaining - width).max(0.0);
    }
    widths.push(PageFlowLayoutWidth::from_pixels(remaining));
    PageFlowLaneWidths(widths.into())
}
