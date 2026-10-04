use crate::ui::board_workspace::page::editor::editing::{
    PageEditEffect, PageEditHostEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use crate::ui::board_workspace::page::editor::PageMutationPlan;
use std::collections::HashMap;

use super::{normalize_page_focus, normalize_page_text_selection};
use crate::model::CardPage;
use crate::ui::surface::{PageDocuments, PageEditorState};
use crate::ui::{LoadedCardPageData, PageEditFocus, PageEditScope, PageEditSnapshot};

use super::column_compatibility::{
    prepare_column_ratio_history, rebase_column_ratio_history_target,
    rebase_whole_page_column_history_target, validate_whole_page_column_history_compatibility,
};
use super::table_compatibility::{
    rebase_simple_table_cell_history_target, validate_simple_table_history_compatibility,
};

pub(in crate::ui::board_workspace::page) struct PageHistoryRestoreState {
    page_id: String,
    text_selection: Option<crate::ui::PageTextSelection>,
    snapshot_had_selection: bool,
    snapshot_focus: Option<PageEditFocus>,
    current_focus: Option<PageEditFocus>,
}

impl PageEditSession<'_> {
    pub(crate) fn restore_page_edit_history_for_block(
        &mut self,
        block_id: &str,
        focus_offset: usize,
        redo: bool,
    ) -> bool {
        let Some(page) = self.documents.page_containing_block(block_id) else {
            return false;
        };
        self.restore_page_edit_history_for_page(
            &page.block_id,
            redo,
            Some(PageEditFocus::Block {
                block_id: block_id.to_string(),
                offset: focus_offset,
            }),
        )
    }

    pub(in crate::ui::board_workspace::page::editor) fn restore_page_edit_history_for_page(
        &mut self,
        page_id: &str,
        redo: bool,
        current_focus: Option<PageEditFocus>,
    ) -> bool {
        let page_link_commit_in_flight = self
            .editor
            .page_link_icons
            .commit_in_flight_for_page(page_id);
        if page_link_commit_in_flight {
            return false;
        }
        let Some(current_page) = self.documents.page_with_id(page_id) else {
            return false;
        };
        let current_focus = self
            .documents
            .loaded_page_data_with_id(page_id)
            .and_then(|data| normalize_page_focus(&data, current_focus));
        let Some(target_snapshot) = self
            .editor
            .history_target_snapshot(&current_page.block_id, redo)
        else {
            return false;
        };
        let current_snapshot = self.editor.page_edit_snapshot(
            &current_page,
            current_focus.clone(),
            target_snapshot.scope.clone(),
        );
        let rebased_cell_page = match self.documents.prepare_page_history_target(
            page_id,
            &current_page,
            &target_snapshot,
        ) {
            Ok(page) => page,
            Err(error) => {
                self.editor
                    .invalidate_page_edit_history(&current_page.block_id);
                self.effects.push(PageEditEffect::Error(format!(
                    "could not restore page history: {error}"
                )));
                self.effects.push(PageEditEffect::Notify);
                return false;
            }
        };
        let Some(mut target) = self.editor.pop_page_history_target(current_snapshot, redo) else {
            return false;
        };
        if let Some(page) = rebased_cell_page {
            target.page = page;
        }
        synchronize_snapshot_metadata(&current_page, &mut target.page);
        self.queue_page_history_restore(current_page, target, current_focus)
    }

    fn queue_page_history_restore(
        &mut self,
        current_page: CardPage,
        target: PageEditSnapshot,
        current_focus: Option<PageEditFocus>,
    ) -> bool {
        let plan = match PageMutationPlan::snapshot_transition(&current_page, &target.page) {
            Ok(plan) => plan,
            Err(error) => {
                self.editor
                    .invalidate_page_edit_history(&current_page.block_id);
                self.effects.push(PageEditEffect::Error(format!(
                    "could not persist page history: {error}"
                )));
                self.effects.push(PageEditEffect::Notify);
                return false;
            }
        };
        let page_id = target.page.block_id.clone();
        let snapshot_had_selection = target.text_selection.is_some();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(plan),
        ));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearPageHistoryFocus,
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(target.page));
        self.effects.push(PageEditEffect::Host(
            PageEditHostEffect::RestorePageHistoryState(PageHistoryRestoreState {
                page_id,
                text_selection: target.text_selection,
                snapshot_had_selection,
                snapshot_focus: target.focus,
                current_focus,
            }),
        ));
        self.effects.push(PageEditEffect::Notify);
        true
    }

    pub(crate) fn retain_compatible_page_edit_histories(&mut self) {
        let incompatible = self
            .editor
            .incompatible_page_edit_history_ids(self.documents);
        for page_id in incompatible {
            self.editor.invalidate_page_edit_history(&page_id);
            self.effects.push(PageEditEffect::Error(format!(
                "discarded page {page_id} history after incompatible live authority"
            )));
        }
    }
}

impl PageEditorState {
    fn page_edit_snapshot(
        &self,
        page: &CardPage,
        focus: Option<PageEditFocus>,
        scope: PageEditScope,
    ) -> PageEditSnapshot {
        PageEditSnapshot {
            page: page.clone(),
            focus,
            text_selection: self.page_edit_text_selection(page),
            scope,
        }
    }

    fn history_target_snapshot(&self, page_id: &str, redo: bool) -> Option<PageEditSnapshot> {
        let history = self.page_edit_histories.get(page_id)?;
        let stack = if redo {
            &history.redo_stack
        } else {
            &history.undo_stack
        };
        stack.last().cloned()
    }

    fn pop_page_history_target(
        &mut self,
        current: PageEditSnapshot,
        redo: bool,
    ) -> Option<PageEditSnapshot> {
        let history = self.page_edit_histories.get_mut(&current.page.block_id)?;
        let target = if redo {
            let snapshot = history.redo_stack.pop()?;
            history.undo_stack.push(current);
            snapshot
        } else {
            let snapshot = history.undo_stack.pop()?;
            history.redo_stack.push(current);
            snapshot
        };
        history.text_group = None;
        Some(target)
    }

    fn invalidate_page_edit_history(&mut self, page_id: &str) {
        let Some(history) = self.page_edit_histories.get_mut(page_id) else {
            return;
        };
        history.undo_stack.clear();
        history.redo_stack.clear();
        history.text_group = None;
    }

    pub(in crate::ui::board_workspace::page::editor) fn clear_page_history_focus(&mut self) {
        self.page_text_selection = None;
        self.active_page_block = None;
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
        self.tables.editor().clear();
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_block_context_menu = None;
        self.page_link_icons.clear_picker();
    }

    pub(in crate::ui::board_workspace::page::editor) fn restore_page_history_state(
        &mut self,
        documents: &PageDocuments,
        state: PageHistoryRestoreState,
    ) -> (String, Option<PageEditFocus>) {
        let data = documents.loaded_page_data_with_id(&state.page_id);
        let selection = data.as_deref().and_then(|data| {
            state
                .text_selection
                .and_then(|selection| normalize_page_text_selection(data, selection))
        });
        let focus = restored_page_focus(
            data.as_deref(),
            selection.as_ref(),
            state.snapshot_had_selection,
            state.snapshot_focus,
            state.current_focus,
        );
        self.page_text_selection = selection;
        (state.page_id, focus)
    }
}

fn validate_history_compatibility(
    current: &CardPage,
    target: &CardPage,
    scope: &PageEditScope,
) -> Result<(), String> {
    match scope {
        PageEditScope::ColumnRatios(pair) => {
            prepare_column_ratio_history(current, target, pair).map(|_| ())
        }
        PageEditScope::WholePage => {
            validate_simple_table_history_compatibility(current, target, scope)?;
            validate_whole_page_column_history_compatibility(current, target)
        }
        PageEditScope::SimpleTableCell(_) => {
            validate_simple_table_history_compatibility(current, target, scope)
        }
    }
}

fn synchronize_snapshot_metadata(current: &CardPage, target: &mut CardPage) {
    target.title.clone_from(&current.title);
    let current_last_edited = current
        .blocks
        .iter()
        .map(|block| (block.block_id.as_str(), block.last_edited.clone()))
        .collect::<HashMap<_, _>>();
    for block in &mut target.blocks {
        if let Some(last_edited) = current_last_edited.get(block.block_id.as_str()) {
            block.last_edited.clone_from(last_edited);
        }
    }
}

fn restored_page_focus(
    data: Option<&LoadedCardPageData>,
    selection: Option<&crate::ui::PageTextSelection>,
    snapshot_had_selection: bool,
    snapshot_focus: Option<PageEditFocus>,
    current_focus: Option<PageEditFocus>,
) -> Option<PageEditFocus> {
    if let Some(selection) = selection {
        return Some(PageEditFocus::Block {
            block_id: selection.focus_block_id.clone(),
            offset: selection.focus_offset,
        });
    }
    let data = data?;
    if snapshot_had_selection {
        return normalize_page_focus(data, current_focus);
    }
    normalize_page_focus(data, snapshot_focus).or_else(|| normalize_page_focus(data, current_focus))
}

impl PageEditorState {
    fn incompatible_page_edit_history_ids(&self, documents: &PageDocuments) -> Vec<String> {
        self.page_edit_histories
            .iter()
            .filter_map(|(page_id, history)| {
                let Some(current) = documents.page_with_id(page_id) else {
                    return Some(page_id.clone());
                };
                let Some(authority) = documents.page_authority_with_id(page_id) else {
                    return Some(page_id.clone());
                };
                history
                    .undo_stack
                    .iter()
                    .chain(&history.redo_stack)
                    .any(|snapshot| {
                        validate_history_compatibility(&authority, &snapshot.page, &snapshot.scope)
                            .is_err()
                            || validate_history_compatibility(
                                &current,
                                &snapshot.page,
                                &snapshot.scope,
                            )
                            .is_err()
                    })
                    .then(|| page_id.clone())
            })
            .collect()
    }
}

impl PageDocuments {
    fn prepare_page_history_target(
        &self,
        page_id: &str,
        current: &CardPage,
        target: &PageEditSnapshot,
    ) -> Result<Option<CardPage>, String> {
        self.validate_history_authority(page_id, &target.page, &target.scope)?;
        match &target.scope {
            PageEditScope::SimpleTableCell(address) => {
                validate_simple_table_history_compatibility(current, &target.page, &target.scope)?;
                rebase_simple_table_cell_history_target(current, &target.page, address).map(Some)
            }
            PageEditScope::ColumnRatios(pair) => {
                let prepared = prepare_column_ratio_history(current, &target.page, pair)?;
                rebase_column_ratio_history_target(current, pair, prepared).map(Some)
            }
            PageEditScope::WholePage => {
                validate_simple_table_history_compatibility(current, &target.page, &target.scope)?;
                rebase_whole_page_column_history_target(current, &target.page).map(Some)
            }
        }
    }

    fn validate_history_authority(
        &self,
        page_id: &str,
        target: &CardPage,
        scope: &PageEditScope,
    ) -> Result<(), String> {
        let authority = self
            .page_authority_with_id(page_id)
            .ok_or_else(|| format!("live authority for page {page_id} is unavailable"))?;
        validate_history_compatibility(&authority, target, scope)
    }
}
