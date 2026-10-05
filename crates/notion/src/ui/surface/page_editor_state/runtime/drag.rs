use std::{cell::RefCell, rc::Rc};

use super::super::{
    PageBlockDragAutoScroll, PageBlockDragLayouts, PageBlockDragTarget, PageColumnResizeSession,
};

#[derive(Default)]
pub(crate) struct PageDocumentDragState {
    pub(crate) auto_scroll: Option<PageBlockDragAutoScroll>,
    pub(crate) auto_scroll_epoch: usize,
    pub(crate) layouts: PageBlockDragLayouts,
    pub(crate) target: Option<PageBlockDragTarget>,
    pub(crate) cancelled: bool,
    pub(crate) column_resize: Option<PageColumnResizeSession>,
}

/// Shared drag geometry used by synchronous GPUI `can_drop` and prepaint hooks.
#[derive(Clone, Default)]
pub(crate) struct PageDocumentDragRuntime(Rc<RefCell<PageDocumentDragState>>);

impl PageDocumentDragRuntime {
    pub(crate) fn borrow(&self) -> std::cell::Ref<'_, PageDocumentDragState> {
        self.0.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> std::cell::RefMut<'_, PageDocumentDragState> {
        self.0.borrow_mut()
    }

    pub(crate) fn reset_for_route(&self) {
        let mut state = self.0.borrow_mut();
        state.auto_scroll = None;
        state.cancelled = false;
        state.auto_scroll_epoch = state.auto_scroll_epoch.wrapping_add(1);
        state.layouts = PageBlockDragLayouts::default();
        state.target = None;
        state.column_resize = None;
    }
}
