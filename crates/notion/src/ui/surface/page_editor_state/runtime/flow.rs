use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use super::super::{
    PageDocumentFocusRegistryEntry, PageFlowCommittedFocus, PageFlowObservations,
    PageFlowSurfaceKey, PageFlowVirtualizer,
};

/// Focus registrations keyed by block id and flow surface.
type PageDocumentFocusRegistry =
    HashMap<(String, PageFlowSurfaceKey), PageDocumentFocusRegistryEntry>;

#[derive(Default)]
pub(crate) struct PageDocumentFlowState {
    pub(crate) focus_handles: RefCell<PageDocumentFocusRegistry>,
    pub(crate) observations: RefCell<PageFlowObservations>,
    pub(crate) virtualizer: RefCell<PageFlowVirtualizer>,
    pub(crate) committed_focus: RefCell<Option<PageFlowCommittedFocus>>,
    pub(crate) layout_generation: Cell<u64>,
}

/// Live document-flow state retained by virtualized list and prepaint closures.
#[derive(Clone, Default)]
pub(crate) struct PageDocumentFlowRuntime(Rc<PageDocumentFlowState>);

impl PageDocumentFlowRuntime {
    pub(crate) fn state(&self) -> &PageDocumentFlowState {
        &self.0
    }

    pub(crate) fn next_layout_generation(&self) -> u64 {
        let next = self.0.layout_generation.get().wrapping_add(1);
        self.0.layout_generation.set(next);
        next
    }

    pub(crate) fn committed_focus(&self) -> Option<PageFlowCommittedFocus> {
        self.0.committed_focus.borrow().clone()
    }

    pub(crate) fn set_committed_focus(&self, focus: Option<PageFlowCommittedFocus>) {
        self.0.committed_focus.replace(focus);
    }

    pub(crate) fn inherit_cached_state(&mut self, previous: &Self) {
        *self = previous.clone();
        self.0.observations.replace(PageFlowObservations::default());
        self.0.committed_focus.borrow_mut().take();
    }

    pub(crate) fn clear_for_route(&self) {
        self.0.focus_handles.borrow_mut().clear();
        self.0.observations.replace(PageFlowObservations::default());
        self.0.virtualizer.borrow_mut().clear();
        self.0.committed_focus.borrow_mut().take();
    }
}
