use crate::ui::board_workspace::PageFocusSession;
use gpui::App;

use crate::ui::board_workspace::PageEditSession;

use super::support::page_block_subtree_end;
use super::{
    CardPageBlockKind, CardPeekState, Context, LoadedCardPageData, SurfaceState, TextInputSnapshot,
    Window,
};
use crate::ui::surface::{PageDocuments, PageEditorState};

struct PageToggleTarget {
    block_index: usize,
    text_len: usize,
}

struct PageToggleCompositionSettlement {
    block_id: String,
    snapshot: TextInputSnapshot,
}

struct PreparedPageToggleDisclosure {
    page_id: String,
    block_id: String,
    text_len: usize,
    settlements: Vec<PageToggleCompositionSettlement>,
}

struct PageToggleProjectionChange {
    page_id: String,
    block_id: String,
    text_len: usize,
    expanded: bool,
}

impl SurfaceState {
    pub(super) fn toggle_page_block_disclosure(
        &mut self,
        page_id: &str,
        block_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let prepared = {
            let session = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            session.prepare_page_toggle_disclosure(page_id, block_id, cx)
        };
        match prepared {
            Ok(Some(prepared)) => apply_prepared_page_toggle_disclosure(self, prepared, window, cx),
            Ok(None) => {}
            Err(error) => {
                self.print_notion_error(format!(
                    "could not collapse Notion toggle {block_id}: {error}"
                ));
            }
        }
    }
}

impl PageEditSession<'_> {
    fn prepare_page_toggle_disclosure(
        &self,
        page_id: &str,
        block_id: &str,
        cx: &App,
    ) -> Result<Option<PreparedPageToggleDisclosure>, String> {
        let Some(data) = self.documents.active_page_data_with_id(page_id) else {
            return Ok(None);
        };
        let Some(target) = page_toggle_target(&data, block_id) else {
            return Ok(None);
        };
        let was_expanded = self
            .editor
            .page_toggle_disclosure
            .disclosure(page_id, block_id)
            .is_expanded();
        let settlements = if was_expanded {
            self.editor
                .page_toggle_composition_settlements(&data, target.block_index, cx)?
        } else {
            Vec::new()
        };
        Ok(Some(PreparedPageToggleDisclosure {
            page_id: page_id.to_string(),
            block_id: block_id.to_string(),
            text_len: target.text_len,
            settlements,
        }))
    }

    fn finish_page_toggle_disclosure(
        &mut self,
        page_id: String,
        block_id: String,
        text_len: usize,
    ) -> PageToggleProjectionChange {
        let expanded = self
            .editor
            .page_toggle_disclosure
            .toggle(&page_id, &block_id)
            .is_expanded();
        PageToggleProjectionChange {
            page_id,
            block_id,
            text_len,
            expanded,
        }
    }
}

impl PageDocuments {
    pub(in crate::ui) fn rebuild_loaded_page_disclosure_projections(
        &mut self,
        editor: &mut PageEditorState,
    ) {
        let mut changed_pages = Vec::new();
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_mut() {
            let expanded = editor
                .page_toggle_disclosure
                .expanded_block_ids(&page.data.page.block_id);
            if page.rebuild_visible_projection(expanded) {
                changed_pages.push(page.data.page.block_id.clone());
            }
        }
        if let Some(page) = self.standalone.as_mut() {
            let expanded = editor
                .page_toggle_disclosure
                .expanded_block_ids(&page.data.page.block_id);
            if page.rebuild_visible_projection(expanded) {
                changed_pages.push(page.data.page.block_id.clone());
            }
        }
        changed_pages.sort();
        changed_pages.dedup();
        for page_id in changed_pages {
            editor.reconcile_loaded_page_flow_surfaces(
                &page_id,
                self.loaded_page_flow_surfaces(&page_id),
            );
        }
    }

    fn rebuild_page_disclosure_projections(
        &mut self,
        editor: &mut PageEditorState,
        page_id: &str,
    ) -> bool {
        let expanded = editor.page_toggle_disclosure.expanded_block_ids(page_id);
        let selected_changed = match self.selected_page.as_mut() {
            Some(CardPeekState::Loaded(page)) if page.data.page.block_id == page_id => {
                page.rebuild_visible_projection(expanded)
            }
            _ => false,
        };
        let expanded = editor.page_toggle_disclosure.expanded_block_ids(page_id);
        let standalone_changed = self
            .standalone
            .as_mut()
            .filter(|page| page.data.page.block_id == page_id)
            .is_some_and(|page| page.rebuild_visible_projection(expanded));
        let changed = selected_changed || standalone_changed;
        if changed {
            editor.reconcile_loaded_page_flow_surfaces(
                page_id,
                self.loaded_page_flow_surfaces(page_id),
            );
        }
        changed
    }
}

fn apply_prepared_page_toggle_disclosure(
    surface: &mut SurfaceState,
    prepared: PreparedPageToggleDisclosure,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let PreparedPageToggleDisclosure {
        page_id,
        block_id,
        text_len,
        settlements,
    } = prepared;
    for settlement in settlements {
        let transition = {
            let mut edit = PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
            edit.apply_page_block_text_change(&settlement.block_id, settlement.snapshot);
            edit.finish(())
        };
        surface.apply_page_edit_transition(transition, cx);
    }
    let transition = {
        let mut edit = PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
        let change = edit.finish_page_toggle_disclosure(page_id, block_id, text_len);
        edit.finish(change)
    };
    let change = surface.apply_page_edit_transition(transition, cx);
    if !surface
        .page_documents
        .rebuild_page_disclosure_projections(&mut surface.page_editor, &change.page_id)
    {
        cx.notify();
        return;
    }
    let data = surface
        .page_documents
        .active_page_data_with_id(&change.page_id)
        .expect("toggled page projection must remain loaded");
    let fallback = (!change.expanded).then_some((change.block_id.as_str(), change.text_len));
    PageFocusSession::new(&mut surface.page_editor, &mut surface.page_documents)
        .reconcile_projection(&data, fallback, cx);
    if !change.expanded && cx.has_active_drag() {
        cx.stop_active_drag(window);
        surface.page_editor.drag.borrow_mut().cancelled = true;
    }
    cx.notify();
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn expand_page_toggle_destination(
        &mut self,
        documents: &PageDocuments,
        page_id: &str,
        block_id: &str,
    ) {
        let Some(data) = documents.active_page_data_with_id(page_id) else {
            return;
        };
        let Some(index) = data.editable_block_indices.get(block_id).copied() else {
            return;
        };
        if data.page.blocks[index]
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
        {
            self.page_toggle_disclosure.expand(page_id, block_id);
        }
    }

    fn page_toggle_composition_settlements(
        &self,
        data: &LoadedCardPageData,
        toggle_index: usize,
        cx: &App,
    ) -> Result<Vec<PageToggleCompositionSettlement>, String> {
        let subtree_end = page_block_subtree_end(&data.page.blocks, toggle_index);
        let inputs = self.input.resource_state().block_inputs.borrow();
        data.page.blocks[toggle_index + 1..subtree_end]
            .iter()
            .filter(|block| self.page_block_compositions.contains(&block.block_id))
            .map(|block| {
                if block.editable_content().is_none() {
                    return Err(format!(
                        "composing descendant {} is no longer editable",
                        block.block_id
                    ));
                }
                if !data.editable_block_is_visible(&block.block_id) {
                    return Err(format!(
                        "composing descendant {} is not visible",
                        block.block_id
                    ));
                }
                let input = inputs.get(&block.block_id).ok_or_else(|| {
                    format!(
                        "missing text input for composing descendant {}",
                        block.block_id
                    )
                })?;
                let mut snapshot = input.read(cx).snapshot();
                snapshot.is_composing = false;
                Ok(PageToggleCompositionSettlement {
                    block_id: block.block_id.clone(),
                    snapshot,
                })
            })
            .collect()
    }
}

fn page_toggle_target(data: &LoadedCardPageData, block_id: &str) -> Option<PageToggleTarget> {
    let block_index = data.editable_block_indices.get(block_id).copied()?;
    let editable = data.page.blocks[block_index].editable_content()?;
    (editable.kind == CardPageBlockKind::ToggleList).then_some(PageToggleTarget {
        block_index,
        text_len: editable.text.len(),
    })
}
