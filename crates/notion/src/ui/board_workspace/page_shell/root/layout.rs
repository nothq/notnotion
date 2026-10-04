use crate::ui::{
    surface::NotionChromeState, NotionAiMode, NOTION_PAGE_MAIN_MIN_WIDTH,
    NOTION_PAGE_SIDEBAR_MIN_WIDTH,
};

use super::super::ai::{AI_PANEL_MAX_WIDTH, AI_PANEL_MIN_WIDTH, AI_PANEL_WIDTH_RATIO};

/// The page shell's dimensions resolved from one chrome and viewport snapshot.
#[derive(Clone, Copy)]
pub(crate) struct NotionPageLayout {
    preview_width: f32,
    leading_width: f32,
    sidebar_width: f32,
    sidebar_visible: bool,
    sidebar_bounds: (f32, f32),
    ai_panel_width: f32,
    consuming_ai_width: f32,
}

impl NotionPageLayout {
    pub(super) fn new(chrome: &NotionChromeState, preview_width: f32, app_width: f32) -> Self {
        let ai_panel_width =
            (preview_width * AI_PANEL_WIDTH_RATIO).clamp(AI_PANEL_MIN_WIDTH, AI_PANEL_MAX_WIDTH);
        let consuming_ai_width =
            if chrome.notion_ai_open && chrome.notion_ai_mode == NotionAiMode::Sidebar {
                ai_panel_width
            } else {
                0.0
            };
        let total_width = (preview_width - consuming_ai_width).max(0.0);
        let max_width = (total_width - NOTION_PAGE_MAIN_MIN_WIDTH)
            .max(NOTION_PAGE_SIDEBAR_MIN_WIDTH)
            .min(total_width);
        let min_width = NOTION_PAGE_SIDEBAR_MIN_WIDTH.min(max_width);
        Self {
            preview_width,
            leading_width: (app_width - preview_width).max(0.0),
            sidebar_width: chrome.notion_sidebar_width.clamp(min_width, max_width),
            sidebar_visible: chrome.notion_sidebar_visible,
            sidebar_bounds: (min_width, max_width),
            ai_panel_width,
            consuming_ai_width,
        }
    }

    pub(crate) fn sidebar_width(self) -> f32 {
        self.sidebar_width
    }

    pub(crate) fn sidebar_layout_width(self) -> f32 {
        if self.sidebar_visible {
            self.sidebar_width
        } else {
            0.0
        }
    }

    pub(crate) fn main_pane_width(self) -> f32 {
        (self.preview_width - self.consuming_ai_width - self.sidebar_layout_width()).max(0.0)
    }

    pub(crate) fn surface_leading_width(self) -> f32 {
        self.leading_width
    }

    pub(crate) fn consuming_ai_width(self) -> f32 {
        self.consuming_ai_width
    }

    pub(crate) fn ai_panel_width(self) -> f32 {
        self.ai_panel_width
    }
}

impl NotionChromeState {
    pub(super) fn set_sidebar_width(&mut self, width: f32, layout: NotionPageLayout) -> bool {
        let (min_width, max_width) = layout.sidebar_bounds;
        let next = width.clamp(min_width, max_width);
        if (self.notion_sidebar_width - next).abs() < f32::EPSILON {
            return false;
        }
        self.notion_sidebar_width = next;
        true
    }
}
