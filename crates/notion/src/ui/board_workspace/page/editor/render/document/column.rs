use crate::model::CardPageFormat;
use crate::ui::board_workspace::selected_page_overlay::SelectedPageOverlayLayout;

use super::PageDocumentLayout;

const CENTERED_COLUMN_MAX_WIDTH: f32 = 720.0;
const STANDALONE_MARGIN: f32 = 24.0;
const FULL_WIDTH_MARGIN: f32 = 96.0;
const SELECTED_PAGE_MARGIN: f32 = 56.0;

/// The column a page's blocks occupy in the pane that shows the page.
///
/// Notion centers a page in a column at most 720px wide. A Full width page
/// open as a full page fills its pane inside fixed 96px side margins instead;
/// Notion's peeks lay out every page the same way.
#[derive(Clone, Copy)]
pub(crate) struct PageDocumentColumn {
    pane_width: f32,
    margin: f32,
    maximum_width: Option<f32>,
}

impl PageDocumentColumn {
    /// The column of a page shown with `layout` in a main pane
    /// `main_pane_width` wide.
    pub(crate) fn new(
        layout: PageDocumentLayout,
        format: CardPageFormat,
        main_pane_width: f32,
    ) -> Self {
        assert!(main_pane_width.is_finite() && main_pane_width >= 0.0);
        let (pane_width, margin) = match layout {
            PageDocumentLayout::Standalone if format.full_width => {
                (main_pane_width, FULL_WIDTH_MARGIN)
            }
            PageDocumentLayout::Standalone => (main_pane_width, STANDALONE_MARGIN),
            PageDocumentLayout::SelectedPage => (
                SelectedPageOverlayLayout::panel_width(main_pane_width),
                SELECTED_PAGE_MARGIN,
            ),
        };
        Self {
            pane_width,
            margin,
            maximum_width: Self::maximum_width_for(layout, format),
        }
    }

    /// The widest a page's column grows, or `None` when it fills its pane.
    pub(crate) fn maximum_width_for(
        layout: PageDocumentLayout,
        format: CardPageFormat,
    ) -> Option<f32> {
        match layout {
            PageDocumentLayout::Standalone if format.full_width => None,
            PageDocumentLayout::Standalone | PageDocumentLayout::SelectedPage => {
                Some(CENTERED_COLUMN_MAX_WIDTH)
            }
        }
    }

    pub(crate) const fn pane_width(self) -> f32 {
        self.pane_width
    }

    /// The pane's horizontal padding on each side of the column.
    pub(crate) const fn margin(self) -> f32 {
        self.margin
    }

    pub(crate) const fn maximum_width(self) -> Option<f32> {
        self.maximum_width
    }

    pub(crate) fn width(self) -> f32 {
        self.width_in(self.pane_width)
    }

    /// The column's width in a pane `pane_width` wide.
    pub(crate) fn width_in(self, pane_width: f32) -> f32 {
        let available = (pane_width - self.margin * 2.0).max(0.0);
        self.maximum_width
            .map_or(available, |maximum| available.min(maximum))
    }

    /// The distance from the pane's leading edge to the column.
    pub(crate) fn leading_inset(self) -> f32 {
        (self.pane_width - self.width()) / 2.0
    }
}
