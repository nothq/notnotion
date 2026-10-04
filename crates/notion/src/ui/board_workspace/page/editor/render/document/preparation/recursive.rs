use std::sync::Arc;

use gpui::{px, Bounds, Context, FocusHandle, ListOffset, Pixels};

use super::anchors::{recursive_anchor_baseline, PageFlowAnchorCaptures};
use super::outer_items::{PageDocumentListReconciliation, PageDocumentOuterReconcileMode};
use crate::ui::surface::{
    PageEditorState, PageFlowAnchorTarget, PageFlowCapturedSemanticAnchor,
    PageFlowLayoutFrameToken, PageFlowPinSet, PageFlowPinTarget, PageFlowSemanticAnchorBaseline,
    PageFlowSemanticAnchorIntent, PageFlowSemanticAnchorTransaction, PageFlowSurfaceKey,
    PageSimpleTableCellFocusMode,
};
use crate::ui::{LoadedCardPage, LoadedCardPageData, SurfaceState};

impl PageEditorState {
    pub(super) fn prepare_recursive_page_flow(
        &self,
        page: &LoadedCardPage,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        cx: &mut Context<SurfaceState>,
    ) -> PageRecursiveFlowPreparation {
        let viewport = page.list_state.viewport_bounds();
        let logical_scroll_top = page.list_state.logical_scroll_top();
        let table_pins = self.page_flow_table_pins(data);
        let captured = self.flow.state().observations.borrow().capture_true_top(
            surface,
            &page.list_allocation,
            viewport,
            logical_scroll_top,
        );
        let pointer = self.capture_pointer_table_anchor(page, surface, viewport, &table_pins);
        let outer = self.prepare_page_document_outer_items(
            data,
            surface,
            PageDocumentListReconciliation {
                list_state: &page.list_state,
                mode: PageDocumentOuterReconcileMode::RecursiveFlow,
            },
            cx,
        );
        let remap = outer
            .remap
            .clone()
            .expect("recursive Notion outer reconciliation must return a semantic remap");
        let mut pins = self.page_flow_pin_set(data, captured.as_ref(), &table_pins);
        let baseline = recursive_anchor_baseline(
            data,
            &pins,
            PageFlowAnchorCaptures {
                true_top: captured,
                pointer_table: pointer,
            },
            &page.list_allocation,
            &remap,
        );
        pins.viewport_anchor = baseline.as_ref().map(|anchor| anchor.target.clone());
        let transaction = baseline.clone().map(|baseline| {
            PageFlowSemanticAnchorTransaction::new(baseline, remap, outer.order_changed)
        });
        let frame = self
            .flow
            .state()
            .virtualizer
            .borrow_mut()
            .prepare_recursive_layout_frame(data, surface, &pins, transaction);
        if let Some(baseline) = baseline.as_ref().filter(|baseline| {
            baseline.intent != PageFlowSemanticAnchorIntent::ViewportPreservation
                || outer.order_changed
        }) {
            self.prepare_estimated_page_flow_anchor(&page.list_state, &frame, baseline);
        }
        PageRecursiveFlowPreparation {
            frame,
            focus_handles: outer.handles,
        }
    }

    fn capture_pointer_table_anchor(
        &self,
        page: &LoadedCardPage,
        surface: PageFlowSurfaceKey,
        viewport: Bounds<Pixels>,
        table_pins: &PageFlowTablePins,
    ) -> Option<PageFlowCapturedSemanticAnchor> {
        if !table_pins.pointer_pending {
            return None;
        }
        let target = table_pins
            .pending
            .as_ref()
            .expect("pointer table focus must retain its pending unit pin");
        Some(
            self.flow
                .state()
                .observations
                .borrow()
                .capture_target(
                    surface,
                    &page.list_allocation,
                    PageFlowAnchorTarget {
                        target,
                        intent: PageFlowSemanticAnchorIntent::PointerTableFocus,
                    },
                    viewport,
                )
                .expect("a pointer-focused Notion table row must already be mounted"),
        )
    }

    fn prepare_estimated_page_flow_anchor(
        &self,
        list_state: &gpui::ListState,
        frame: &PageFlowLayoutFrameToken,
        baseline: &PageFlowSemanticAnchorBaseline,
    ) {
        let mut virtualizer = self.flow.state().virtualizer.borrow_mut();
        let Some(projection) = virtualizer.estimate_semantic_projection(frame, &baseline.target)
        else {
            return;
        };
        if !virtualizer.update_semantic_anchor_projection(frame, projection.clone())
            || list_state.is_scrollbar_dragging()
            || baseline.intent == PageFlowSemanticAnchorIntent::PointerTableFocus
        {
            return;
        }
        let desired = projection.outer_local_y.pixels() - baseline.viewport_relative_y.pixels();
        list_state.scroll_to(ListOffset {
            item_ix: projection.outer_index,
            offset_in_item: px(desired.max(0.0)),
        });
    }

    fn page_flow_pin_set(
        &self,
        data: &LoadedCardPageData,
        captured: Option<&PageFlowCapturedSemanticAnchor>,
        table_pins: &PageFlowTablePins,
    ) -> PageFlowPinSet {
        let unit = |block_id: Option<&str>| {
            block_id
                .and_then(|block_id| data.document_unit_key_for_block(block_id))
                .map(PageFlowPinTarget::Unit)
        };
        let pending_focus = self.input.resource_state().focus_request.borrow();
        let block_selection = &self.page_block_selection.block_ids;
        let (text_selection_anchor, text_selection_focus) =
            self.page_flow_text_selection_pins(data);
        PageFlowPinSet {
            viewport_anchor: captured.map(|anchor| anchor.baseline.target.clone()),
            active_input: unit(self.active_page_block.as_deref()),
            pending_focus: unit(pending_focus.as_ref().map(|focus| focus.block_id.as_str())),
            rich_text_composition: unit(
                self.page_pending_rich_text_composition
                    .as_ref()
                    .map(|composition| composition.block_id.as_str()),
            ),
            cross_block_composition: unit(
                self.page_pending_cross_block_composition
                    .as_ref()
                    .map(|composition| composition.survivor_block_id()),
            ),
            active_table_cell: table_pins.active.clone(),
            pending_table_focus: table_pins.pending.clone(),
            text_selection_anchor,
            text_selection_focus,
            block_selection_first: unit(block_selection.first().map(String::as_str)),
            block_selection_last: unit(block_selection.last().map(String::as_str)),
            context_menu: unit(
                self.page_block_context_menu
                    .as_ref()
                    .map(|menu| menu.block_id.as_str()),
            ),
            page_link_icon_picker: unit(
                self.page_link_icons
                    .picker()
                    .filter(|picker| picker.page_id == data.page.block_id)
                    .map(|picker| picker.block_id.as_str()),
            ),
            slash_menu: unit(
                self.page_slash_menu
                    .as_ref()
                    .map(|menu| menu.block_id.as_str()),
            ),
        }
    }

    fn page_flow_text_selection_pins(
        &self,
        data: &LoadedCardPageData,
    ) -> (Option<PageFlowPinTarget>, Option<PageFlowPinTarget>) {
        let Some(selection) = self.page_text_selection.as_ref() else {
            return (None, None);
        };
        let pin = |block_id: &str| {
            data.document_unit_key_for_block(block_id)
                .map(PageFlowPinTarget::Unit)
        };
        (
            pin(&selection.anchor_block_id),
            pin(&selection.focus_block_id),
        )
    }

    fn page_flow_table_pins(&self, data: &LoadedCardPageData) -> PageFlowTablePins {
        let editor = self.tables.editor().borrow();
        let Some(editor) = editor
            .as_ref()
            .filter(|editor| editor.page_id == data.page.block_id)
        else {
            return PageFlowTablePins::default();
        };
        let active = data
            .document_unit_key_for_table_cell(&editor.address)
            .map(PageFlowPinTarget::Unit);
        PageFlowTablePins {
            active: active.clone(),
            pending: editor.pending_focus.as_ref().and(active),
            pointer_pending: editor.pending_focus.as_ref().is_some_and(|pending| {
                matches!(&pending.mode, PageSimpleTableCellFocusMode::Pointer(_))
            }),
        }
    }
}

#[derive(Default)]
struct PageFlowTablePins {
    active: Option<PageFlowPinTarget>,
    pending: Option<PageFlowPinTarget>,
    pointer_pending: bool,
}

pub(super) struct PageRecursiveFlowPreparation {
    pub(super) frame: PageFlowLayoutFrameToken,
    pub(super) focus_handles: Arc<[FocusHandle]>,
}
