use super::{LoadedCardPageData, PageEditorDragReset};
use crate::ui::board_workspace::PageBlockDragLayouts;
use crate::ui::surface::PageEditorState;
use gpui::App;

impl PageEditorState {
    pub(super) fn prune_projection_overlays(&mut self, data: &LoadedCardPageData) {
        if self
            .hovered_page_block
            .as_ref()
            .is_some_and(|block_id| !data.block_is_visible(block_id))
        {
            self.hovered_page_block = None;
        }
        if self
            .page_block_context_menu
            .as_ref()
            .is_some_and(|menu| !data.block_is_visible(&menu.block_id))
        {
            self.page_block_context_menu = None;
        }
        if self
            .page_slash_menu
            .as_ref()
            .is_some_and(|menu| !data.editable_block_is_visible(&menu.block_id))
        {
            self.page_slash_menu = None;
            self.mention.clear_menu();
        }
        if self
            .input
            .composer_handoff
            .as_ref()
            .is_some_and(|handoff| !data.editable_block_is_visible(&handoff.block_id))
        {
            self.input.composer_handoff = None;
        }
    }

    pub(super) fn reconcile_projection_text_selection(
        &mut self,
        data: &LoadedCardPageData,
        cx: &mut App,
    ) -> bool {
        let selection_hidden = self
            .page_text_selection
            .as_ref()
            .is_some_and(|selection| !data.text_selection_is_visible(selection));
        if selection_hidden {
            self.clear_projection_text_input_selections(data, cx);
            self.page_text_selection = None;
        }
        selection_hidden
    }

    pub(super) fn prune_projection_inputs(&mut self, data: &LoadedCardPageData) {
        self.page_block_selection
            .block_ids
            .retain(|block_id| data.block_is_visible(block_id));
        self.page_block_compositions.retain(|block_id| {
            block_id == &data.page.block_id
                || (data.editable_block_indices.contains_key(block_id)
                    && data.editable_block_is_visible(block_id))
        });
        let compositions = &self.page_block_compositions;
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .retain(|block_id, _| projection_input_is_retained(data, compositions, block_id));
        self.input
            .resource_state()
            .block_input_props
            .borrow_mut()
            .retain(|block_id, _| projection_input_is_retained(data, compositions, block_id));
        self.input
            .resource_state()
            .code_syntax
            .borrow_mut()
            .retain(|block_id| projection_input_is_retained(data, compositions, block_id));
    }

    pub(super) fn prune_projection_rich_text_state(&mut self, data: &LoadedCardPageData) -> bool {
        let forced_annotations_hidden = self.prune_projection_forced_annotations(data);
        if self
            .page_pending_rich_text_typing
            .as_ref()
            .is_some_and(|typing| !data.editable_block_is_visible(&typing.block_id))
        {
            self.page_pending_rich_text_typing = None;
        }
        self.prune_projection_composition_state(data);
        forced_annotations_hidden
    }

    pub(super) fn prune_projection_forced_annotations(
        &mut self,
        data: &LoadedCardPageData,
    ) -> bool {
        let hidden = self
            .page_forced_text_annotations
            .as_ref()
            .is_some_and(|annotations| {
                !data.editable_block_is_visible(&annotations.block_id)
                    && !self.page_block_compositions.contains(&annotations.block_id)
            });
        if hidden {
            self.page_forced_text_annotations = None;
        }
        hidden
    }

    pub(super) fn prune_projection_composition_state(&mut self, data: &LoadedCardPageData) {
        let compositions = &self.page_block_compositions;
        if self
            .page_pending_rich_text_composition
            .as_ref()
            .is_some_and(|composition| {
                !data
                    .editable_block_indices
                    .contains_key(&composition.block_id)
                    || !compositions.contains(&composition.block_id)
            })
        {
            self.page_pending_rich_text_composition = None;
        }
        if self
            .page_pending_cross_block_composition
            .as_ref()
            .is_some_and(|composition| {
                !compositions.iter().any(|block_id| {
                    data.editable_block_indices.contains_key(block_id)
                        && composition.matches(block_id)
                })
            })
        {
            self.page_pending_cross_block_composition = None;
        }
    }

    pub(super) fn clear_projection_rich_text_dialog(&mut self) {
        self.page_rich_text_dialog = None;
        self.page_rich_text_link_value.clear();
        self.page_rich_text_link_input.borrow_mut().take();
    }

    pub(super) fn clear_projection_text_input_selections(
        &self,
        data: &LoadedCardPageData,
        cx: &mut App,
    ) {
        let inputs = self
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .iter()
            .filter(|(block_id, _)| {
                data.editable_block_indices.contains_key(*block_id)
                    && !self.page_block_compositions.contains(*block_id)
            })
            .map(|(_, input)| input.clone())
            .collect::<Vec<_>>();
        for input in inputs {
            let cursor = input.read(cx).selection_range().end;
            input.update(cx, |input, cx| {
                input.set_selection(cursor..cursor, false, cx);
            });
        }
    }

    pub(in crate::ui::board_workspace) fn clear_projection_state(&mut self) {
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .clear();
        self.input
            .resource_state()
            .block_input_props
            .borrow_mut()
            .clear();
        self.input.resource_state().code_syntax.borrow_mut().clear();
        self.input
            .resource_state()
            .title_inputs
            .borrow_mut()
            .clear();
        self.flow.state().focus_handles.borrow_mut().clear();
        self.flow.state().virtualizer.borrow_mut().clear();

        self.tables.editor().clear();
        self.active_page_block = None;
        self.hovered_page_block = None;
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
        self.flow.set_committed_focus(None);
        self.page_block_compositions.clear();
        self.page_block_context_menu = None;
        self.page_link_icons.clear_picker();
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_text_selection = None;

        self.page_forced_text_annotations = None;
        self.page_pending_rich_text_typing = None;
        self.page_pending_rich_text_composition = None;
        self.page_pending_cross_block_composition = None;
        self.page_rich_text_dialog = None;
        self.page_rich_text_link_value.clear();
        self.page_rich_text_link_input.borrow_mut().take();
        self.page_block_selection.block_ids.clear();
        self.reset_drag(PageEditorDragReset::CancelHandleInteraction);
    }

    pub(in crate::ui::board_workspace::page::editor) fn reset_drag(
        &mut self,
        reset: PageEditorDragReset,
    ) {
        let mut drag = self.drag.borrow_mut();
        if drag.auto_scroll.take().is_some() {
            drag.auto_scroll_epoch = drag.auto_scroll_epoch.wrapping_add(1);
        }
        drag.layouts = PageBlockDragLayouts::default();
        drag.target = None;
        drag.column_resize = None;
        if matches!(reset, PageEditorDragReset::CancelHandleInteraction) {
            drag.cancelled = true;
        }
    }
}

fn projection_input_is_retained(
    data: &LoadedCardPageData,
    compositions: &std::collections::HashSet<String>,
    block_id: &str,
) -> bool {
    (data.block_has_text_input(block_id) && data.editable_block_is_visible(block_id))
        || compositions.contains(block_id)
}
