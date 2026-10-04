use gpui::{Context, FocusHandle};

use super::super::super::support::page_block_visible_nesting_offsets;
use super::super::super::{
    Arc, LoadedCardPage, LoadedCardPageData, PageBlockDragSelection, SurfaceState,
};
use super::PageDocumentLayout;
use crate::ui::surface::{PageEditorState, PageFlowLayoutFrameToken, PageFlowObservationToken};

mod anchors;
mod outer_items;
mod recursive;

use outer_items::{PageDocumentListReconciliation, PageDocumentOuterReconcileMode};

pub(super) struct PageDocumentRenderState {
    pub(super) data: Arc<LoadedCardPageData>,
    pub(super) layout: PageDocumentLayout,
    pub(super) focus_handles: Arc<[FocusHandle]>,
    pub(super) nesting_offsets: Arc<[f32]>,
    pub(super) drag_selection: Arc<PageBlockDragSelection>,
    pub(super) generation: u64,
    pub(super) observation: PageFlowObservationToken,
    pub(super) section_count: usize,
    pub(super) render_mode: PageDocumentRenderMode,
}

/// The parts of a document render state that each preparation path supplies.
struct PageDocumentPreparation {
    data: Arc<LoadedCardPageData>,
    layout: PageDocumentLayout,
    focus_handles: Arc<[FocusHandle]>,
    generation: u64,
    observation: PageFlowObservationToken,
    render_mode: PageDocumentRenderMode,
}

#[derive(Clone)]
pub(super) enum PageDocumentRenderMode {
    ExistingLinear,
    RecursiveFlow { frame: PageFlowLayoutFrameToken },
}

impl PageDocumentRenderMode {
    pub(super) fn recursive_frame(&self) -> Option<&PageFlowLayoutFrameToken> {
        match self {
            Self::ExistingLinear => None,
            Self::RecursiveFlow { frame } => Some(frame),
        }
    }
}

impl PageEditorState {
    pub(super) fn prepare_page_document(
        &self,
        page: &LoadedCardPage,
        layout: PageDocumentLayout,
        cx: &mut Context<SurfaceState>,
    ) -> PageDocumentRenderState {
        let data = page.data.clone();
        self.prune_page_block_inputs(&data);
        if data.has_column_structure() {
            return self.prepare_recursive_page_document(page, data, layout, cx);
        }
        self.prepare_existing_page_document(page, data, layout, cx)
    }

    fn prepare_existing_page_document(
        &self,
        page: &LoadedCardPage,
        data: Arc<LoadedCardPageData>,
        layout: PageDocumentLayout,
        cx: &mut Context<SurfaceState>,
    ) -> PageDocumentRenderState {
        let render_mode = PageDocumentRenderMode::ExistingLinear;
        let focus_handles = self
            .prepare_page_document_outer_items(
                &data,
                layout.flow_surface_key(),
                PageDocumentListReconciliation {
                    list_state: &page.list_state,
                    mode: PageDocumentOuterReconcileMode::ExistingLinear,
                },
                cx,
            )
            .into_existing_linear_handles();
        let generation = self.next_page_block_layout_generation();
        let observation = PageFlowObservationToken::existing_linear(
            &data,
            &page.list_allocation,
            layout.flow_surface_key(),
            generation,
        );
        self.flow
            .state()
            .observations
            .borrow_mut()
            .begin_surface_generation(observation.clone(), page.list_state.logical_scroll_top());
        self.finish_page_document_preparation(PageDocumentPreparation {
            data,
            layout,
            focus_handles,
            generation,
            observation,
            render_mode,
        })
    }

    fn prepare_recursive_page_document(
        &self,
        page: &LoadedCardPage,
        data: Arc<LoadedCardPageData>,
        layout: PageDocumentLayout,
        cx: &mut Context<SurfaceState>,
    ) -> PageDocumentRenderState {
        let surface = layout.flow_surface_key();
        let flow = self.prepare_recursive_page_flow(page, &data, surface, cx);
        let generation = self.next_page_block_layout_generation();
        let observation = PageFlowObservationToken::recursive(
            &data,
            &page.list_allocation,
            surface,
            generation,
            flow.frame.clone(),
        );
        self.flow
            .state()
            .observations
            .borrow_mut()
            .begin_surface_generation(observation.clone(), page.list_state.logical_scroll_top());
        self.finish_page_document_preparation(PageDocumentPreparation {
            data,
            layout,
            focus_handles: flow.focus_handles,
            generation,
            observation,
            render_mode: PageDocumentRenderMode::RecursiveFlow { frame: flow.frame },
        })
    }

    fn finish_page_document_preparation(
        &self,
        preparation: PageDocumentPreparation,
    ) -> PageDocumentRenderState {
        let PageDocumentPreparation {
            data,
            layout,
            focus_handles,
            generation,
            observation,
            render_mode,
        } = preparation;
        let nesting_offsets = page_block_visible_nesting_offsets(&data);
        let drag_selection = Arc::new(PageBlockDragSelection {
            block_selection: self.page_block_selection.block_ids.clone(),
            text_selection: self.page_text_selection.clone(),
        });
        let section_count = focus_handles.len() - 2;
        PageDocumentRenderState {
            data,
            layout,
            focus_handles,
            nesting_offsets,
            drag_selection,
            generation,
            observation,
            section_count,
            render_mode,
        }
    }

    fn prune_page_block_inputs(&self, data: &LoadedCardPageData) {
        let compositions = &self.page_block_compositions;
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .retain(|block_id, _| {
                (data.block_has_text_input(block_id) && data.editable_block_is_visible(block_id))
                    || compositions.contains(block_id)
            });
        self.input
            .resource_state()
            .block_input_props
            .borrow_mut()
            .retain(|block_id, _| {
                (data.block_has_text_input(block_id) && data.editable_block_is_visible(block_id))
                    || compositions.contains(block_id)
            });
        self.input
            .resource_state()
            .code_syntax
            .borrow_mut()
            .retain(|block_id| {
                data.block_has_text_input(block_id) && data.editable_block_is_visible(block_id)
            });
    }

    fn next_page_block_layout_generation(&self) -> u64 {
        self.flow.next_layout_generation()
    }
}
