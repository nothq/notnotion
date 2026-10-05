use std::rc::{Rc, Weak};

use super::super::{
    PageBlockFocusRequest, PageBlockInputPropsKey, PageCodeSyntaxCache, PageComposerHandoff,
    PageComposerState, PageTextInputRegistry,
};
use std::{cell::RefCell, collections::HashMap};

#[derive(Default)]
pub(crate) struct PageInputResourceState {
    pub(crate) composer_inputs: PageTextInputRegistry,
    pub(crate) composer_focus_request: RefCell<Option<String>>,
    pub(crate) block_inputs: PageTextInputRegistry,
    pub(crate) block_input_props: RefCell<HashMap<String, PageBlockInputPropsKey>>,
    pub(crate) code_syntax: RefCell<PageCodeSyntaxCache>,
    pub(crate) mermaid_diagrams: crate::ui::board_workspace::PageMermaidDiagrams,
    pub(crate) title_inputs: PageTextInputRegistry,
    pub(crate) focus_request: RefCell<Option<PageBlockFocusRequest>>,
}

/// Cloneable access to the live input entity and projection caches.
///
/// Render closures retain this handle, so replacing a `PageEditorState` does not
/// leave mounted inputs writing into a detached cache snapshot.
#[derive(Clone, Default)]
pub(crate) struct PageInputResources(Rc<PageInputResourceState>);

#[derive(Clone)]
pub(crate) struct PageInputResourcesWeak(Weak<PageInputResourceState>);

impl PageInputResources {
    pub(crate) fn state(&self) -> &PageInputResourceState {
        &self.0
    }

    pub(crate) fn downgrade(&self) -> PageInputResourcesWeak {
        PageInputResourcesWeak(Rc::downgrade(&self.0))
    }

    pub(crate) fn cancel_pending_syntax(&self) {
        self.0.code_syntax.borrow_mut().cancel_pending();
    }

    pub(crate) fn clear_for_route(&self) {
        self.0.composer_inputs.borrow_mut().clear();
        self.0.composer_focus_request.borrow_mut().take();
        self.0.block_inputs.borrow_mut().clear();
        self.0.block_input_props.borrow_mut().clear();
        self.0.code_syntax.borrow_mut().clear();
        self.0.title_inputs.borrow_mut().clear();
        self.0.focus_request.borrow_mut().take();
    }
}

impl PageInputResourcesWeak {
    pub(crate) fn upgrade(&self) -> Option<PageInputResources> {
        self.0.upgrade().map(PageInputResources)
    }
}

#[derive(Default)]
pub(crate) struct PageInputRuntime {
    pub(crate) composer: PageComposerState,
    pub(crate) composer_handoff: Option<PageComposerHandoff>,
    resources: PageInputResources,
}

impl PageInputRuntime {
    pub(crate) fn resources(&self) -> PageInputResources {
        self.resources.clone()
    }

    pub(crate) fn resource_state(&self) -> &PageInputResourceState {
        self.resources.state()
    }

    pub(crate) fn take_cached_state_from(&mut self, previous: &mut Self) {
        previous.resources.cancel_pending_syntax();
        self.composer = std::mem::take(&mut previous.composer);
        self.composer_handoff = previous.composer_handoff.take();
        self.resources = previous.resources.clone();
    }

    pub(crate) fn reset_composer(&mut self) {
        self.composer = PageComposerState::default();
        self.composer_handoff = None;
        self.resources.0.composer_inputs.borrow_mut().clear();
        self.resources.0.composer_focus_request.borrow_mut().take();
    }

    pub(crate) fn reset_for_route(&mut self) {
        self.reset_composer();
        self.resources.clear_for_route();
    }
}
