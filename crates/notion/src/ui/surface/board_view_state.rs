use super::{InlineDatabaseViewTabsLayout, ToolbarDialogKind};
use crate::ui::{
    point, px, timeline_initial_scroll_x, BoardSnapshot, DragState, HoveredCard, ScrollHandle,
    Viewport,
};
use gpui::{Bounds, Pixels};

#[derive(Clone, Copy, Default)]
pub(crate) struct InlineToolbarDialogAnchors {
    pub(crate) filter: Option<Bounds<Pixels>>,
    pub(crate) sort: Option<Bounds<Pixels>>,
    pub(crate) automations: Option<Bounds<Pixels>>,
    pub(crate) properties: Option<Bounds<Pixels>>,
    pub(crate) templates: Option<Bounds<Pixels>>,
}

impl InlineToolbarDialogAnchors {
    pub(crate) fn get(self, dialog: ToolbarDialogKind) -> Option<Bounds<Pixels>> {
        match dialog {
            ToolbarDialogKind::Filter => self.filter,
            ToolbarDialogKind::Sort => self.sort,
            ToolbarDialogKind::Automations => self.automations,
            ToolbarDialogKind::Properties => self.properties,
            ToolbarDialogKind::Templates => self.templates,
            ToolbarDialogKind::Actions => None,
        }
    }
}

pub(crate) struct BoardViewState {
    pub(crate) scroll_handle: ScrollHandle,
    pub(crate) toolbar_bounds: Option<Bounds<Pixels>>,
    pub(crate) inline_toolbar_dialog_anchors: InlineToolbarDialogAnchors,
    pub(crate) inline_database_view_tabs_layout: InlineDatabaseViewTabsLayout,
    pub(crate) inline_database_view_more_bounds: Option<Bounds<Pixels>>,
    pub(crate) timeline_scroll_handle: ScrollHandle,
    pub(crate) drag: Option<DragState>,
    pub(crate) hovered_card: Option<HoveredCard>,
    pub(crate) hovered_column_header: Option<usize>,
}

impl BoardViewState {
    pub(crate) fn clear_hover(&mut self) {
        self.hovered_card = None;
        self.hovered_column_header = None;
    }

    pub(crate) fn new_timeline_scroll_handle(
        board: &BoardSnapshot,
        viewport: Viewport,
    ) -> ScrollHandle {
        let timeline_scroll_handle = ScrollHandle::new();
        timeline_scroll_handle.set_offset(point(
            px(-timeline_initial_scroll_x(board, viewport)),
            px(0.0),
        ));
        timeline_scroll_handle
    }

    pub(crate) fn new(timeline_scroll_handle: ScrollHandle) -> Self {
        Self {
            scroll_handle: ScrollHandle::new(),
            toolbar_bounds: None,
            inline_toolbar_dialog_anchors: InlineToolbarDialogAnchors::default(),
            inline_database_view_tabs_layout: InlineDatabaseViewTabsLayout::default(),
            inline_database_view_more_bounds: None,
            timeline_scroll_handle,
            drag: None,
            hovered_card: None,
            hovered_column_header: None,
        }
    }

    pub(crate) fn scroll_x(&self, max_scroll_x: f32) -> f32 {
        (-self.scroll_handle.offset().x.as_f32()).clamp(0.0, max_scroll_x)
    }

    pub(crate) fn reset_for_projection(&mut self, timeline_scroll_handle: ScrollHandle) {
        self.drag = None;
        self.clear_hover();
        self.timeline_scroll_handle = timeline_scroll_handle;
    }

    pub(crate) fn set_scroll_x(&self, scroll_x: f32, max_scroll_x: f32) {
        let clamped_scroll_x = scroll_x.clamp(0.0, max_scroll_x);
        let offset = self.scroll_handle.offset();
        self.scroll_handle
            .set_offset(point(px(-clamped_scroll_x), offset.y));
    }
}
