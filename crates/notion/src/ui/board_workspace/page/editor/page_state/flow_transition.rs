use std::sync::Arc;

use crate::ui::surface::{PageDocuments, PageEditorState, PageFlowSurfaceKey};
use crate::ui::{CardPeekState, LoadedCardPageData};

type LoadedPageFlowSurfaces = Vec<(PageFlowSurfaceKey, Arc<LoadedCardPageData>)>;

pub(super) struct LoadedPageFlowTransition {
    pub(super) surface: PageFlowSurfaceKey,
    pub(super) was_recursive: bool,
    pub(super) page: Arc<LoadedCardPageData>,
    pub(super) list_state: gpui::ListState,
}

impl PageEditorState {
    pub(super) fn apply_loaded_page_flow_transitions(
        &mut self,
        transitions: Vec<LoadedPageFlowTransition>,
    ) {
        let mut entered_linear = false;
        let mut virtualizer = self.flow.state().virtualizer.borrow_mut();
        for transition in transitions {
            if transition.page.has_column_structure() {
                virtualizer.reconcile_surface_projection(&transition.page, transition.surface);
                continue;
            }
            virtualizer.deactivate_surface(&transition.page.page.block_id, transition.surface);
            if transition.was_recursive {
                transition.list_state.remeasure();
                entered_linear = true;
            }
        }
        drop(virtualizer);
        if entered_linear {
            self.flow.set_committed_focus(None);
        }
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn reconcile_loaded_page_flow_surfaces(
        &mut self,
        page_id: &str,
        surfaces: LoadedPageFlowSurfaces,
    ) {
        let mut entered_linear = false;
        let mut virtualizer = self.flow.state().virtualizer.borrow_mut();
        for (surface, data) in surfaces {
            if data.has_column_structure() {
                virtualizer.reconcile_surface_projection(&data, surface);
            } else {
                virtualizer.deactivate_surface(page_id, surface);
                entered_linear = true;
            }
        }
        drop(virtualizer);
        if entered_linear {
            self.flow.set_committed_focus(None);
        }
    }
}

impl PageDocuments {
    pub(in crate::ui::board_workspace::page::editor) fn loaded_page_flow_surfaces(
        &self,
        page_id: &str,
    ) -> LoadedPageFlowSurfaces {
        let mut surfaces = Vec::new();
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.page.block_id == page_id {
                surfaces.push((PageFlowSurfaceKey::SelectedPage, page.data.clone()));
            }
        }
        if let Some(page) = self.standalone.as_ref() {
            if page.data.page.block_id == page_id {
                surfaces.push((PageFlowSurfaceKey::Standalone, page.data.clone()));
            }
        }
        surfaces
    }
}
