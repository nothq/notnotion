use gpui_components::text_input::TextInputSnapshot;

use crate::model::{CardPage, CardPageSimpleTableCellAddress, CardPageWritableSimpleTableCell};
use crate::ui::surface::{
    PageEditorState, PageSimpleTableCellCompositionBaseline, PageSimpleTableCellGeneration,
    PageSimpleTableCellObservedReplacement,
};

use super::super::super::editing::PageEditSession;
use super::{page_with_simple_table_cell, writable_simple_table_cell};

pub(super) struct SimpleTableCompositionLifecycle {
    pub(super) committed: bool,
    pub(super) record_history: bool,
    pub(super) persist: bool,
    pub(super) retire_after_commit: bool,
    pub(super) mark_composition_history: bool,
}

pub(crate) struct PageSimpleTableEditorEffect {
    address: CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    composing: bool,
    text_changed: bool,
    history_recorded: bool,
    committed: bool,
    mark_composition_history: bool,
    retire_after_commit: bool,
}

impl PageSimpleTableEditorEffect {
    pub(super) fn finish_change(
        target: SimpleTableCompositionTarget<'_>,
        text_changed: bool,
        history_recorded: bool,
        lifecycle: &SimpleTableCompositionLifecycle,
    ) -> Self {
        Self {
            address: target.address.clone(),
            generation: target.generation,
            composing: target.snapshot.is_composing,
            text_changed,
            history_recorded,
            committed: lifecycle.committed,
            mark_composition_history: lifecycle.mark_composition_history,
            retire_after_commit: lifecycle.retire_after_commit,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn apply(self, editor: &PageEditorState) {
        let mut table_editor = editor.tables.editor().borrow_mut();
        let Some(active) = table_editor.as_mut().filter(|active| {
            active.address == self.address && active.generation == self.generation
        }) else {
            return;
        };
        active.composition_active = self.composing;
        active.composition_dirty |= self.composing && self.text_changed;
        active.composition_history_recorded |=
            self.mark_composition_history && self.history_recorded;
        if self.committed {
            active.composition_history_recorded = false;
            active.composition_history_baseline = None;
            active.composition_dirty = false;
            active.optimistic_composition_cell = None;
            active.retire_after_composition = false;
        }
        drop(table_editor);
        if self.retire_after_commit
            && editor
                .tables
                .editor()
                .matches(&self.address, self.generation)
        {
            editor.tables.editor().clear();
        }
    }
}

impl PageEditorState {
    pub(super) fn take_simple_table_cell_replacement(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        current_text: &str,
        snapshot: &TextInputSnapshot,
    ) -> Result<Option<PageSimpleTableCellObservedReplacement>, String> {
        let mut editor = self.tables.editor().borrow_mut();
        let active = editor
            .as_mut()
            .filter(|active| active.address == *address && active.generation == generation)
            .ok_or_else(|| "active table-cell text target changed".to_string())?;
        let Some(plan) = active.pending_replacement.take() else {
            if snapshot.text != current_text {
                return Err("table-cell text changed without a typed replacement plan".to_string());
            }
            return Ok(None);
        };
        if plan.previous_text() != current_text {
            return Err("table-cell replacement source no longer matches the input".to_string());
        }
        plan.observe(snapshot)
    }

    pub(super) fn simple_table_composition_lifecycle(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: &TextInputSnapshot,
        text_changed: bool,
    ) -> SimpleTableCompositionLifecycle {
        let editor = self.tables.editor().borrow();
        let active = editor
            .as_ref()
            .filter(|active| active.address == *address && active.generation == generation)
            .expect("validated active table-cell editor must remain present");
        let committed = active.composition_active && !snapshot.is_composing;
        let composition_cycle = snapshot.is_composing || active.composition_active;
        let pending_edit = text_changed || active.composition_dirty;
        let record_history = if composition_cycle {
            pending_edit && !active.composition_history_recorded
        } else {
            text_changed
        };
        SimpleTableCompositionLifecycle {
            committed,
            record_history,
            persist: !snapshot.is_composing
                && (text_changed || active.composition_active && active.composition_dirty),
            retire_after_commit: committed && active.retire_after_composition,
            mark_composition_history: record_history && snapshot.is_composing,
        }
    }

    pub(super) fn simple_table_cell_editor_source(
        &self,
        visual_page: &CardPage,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    ) -> Result<CardPageWritableSimpleTableCell, String> {
        let local = self
            .tables
            .editor()
            .borrow()
            .as_ref()
            .filter(|active| active.address == *address && active.generation == generation)
            .and_then(|active| active.optimistic_composition_cell.clone());
        local.map_or_else(|| writable_simple_table_cell(visual_page, address), Ok)
    }

    pub(super) fn stage_simple_table_composition(
        &self,
        target: SimpleTableCompositionTarget<'_>,
        source: CardPageWritableSimpleTableCell,
        value: CardPageWritableSimpleTableCell,
        lifecycle: &SimpleTableCompositionLifecycle,
    ) {
        if !target.snapshot.is_composing && !lifecycle.committed {
            return;
        }
        let mut editor = self.tables.editor().borrow_mut();
        let Some(active) = editor.as_mut().filter(|active| {
            active.address == *target.address && active.generation == target.generation
        }) else {
            return;
        };
        if target.snapshot.is_composing {
            active.composition_active = true;
        }
        if let Some(replacement) = target.replacement {
            active.composition_dirty = true;
            active.composition_history_baseline.get_or_insert(
                PageSimpleTableCellCompositionBaseline {
                    cell: source,
                    cursor: replacement.previous_cursor,
                },
            );
        }
        active.optimistic_composition_cell = Some(value);
    }
}

impl PageEditSession<'_> {
    pub(super) fn record_simple_table_cell_change_history(
        &mut self,
        visual_page: &CardPage,
        target: SimpleTableCompositionTarget<'_>,
        lifecycle: &SimpleTableCompositionLifecycle,
    ) -> Result<bool, String> {
        if !lifecycle.record_history {
            return Ok(false);
        }
        let (cell, cursor) = self.simple_table_cell_history_baseline(&target)?;
        match page_with_simple_table_cell(visual_page, target.address, cell) {
            Ok(page) => {
                self.editor
                    .record_page_simple_table_cell_text_edit(&page, target.address, cursor);
                Ok(true)
            }
            Err(_) if target.snapshot.is_composing => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn simple_table_cell_history_baseline(
        &self,
        target: &SimpleTableCompositionTarget<'_>,
    ) -> Result<(CardPageWritableSimpleTableCell, usize), String> {
        let editor = self.editor.tables.editor().borrow();
        let active = editor
            .as_ref()
            .filter(|active| {
                active.address == *target.address && active.generation == target.generation
            })
            .ok_or_else(|| "active table-cell history target changed".to_string())?;
        if let Some(baseline) = active.composition_history_baseline.clone() {
            return Ok((baseline.cell, baseline.cursor));
        }
        let replacement = target
            .replacement
            .ok_or_else(|| "table-cell edit has no typed history baseline".to_string())?;
        let page = self
            .documents
            .page_with_id(&active.page_id)
            .ok_or_else(|| format!("loaded page {} disappeared during history", active.page_id))?;
        Ok((
            writable_simple_table_cell(&page, target.address)?,
            replacement.previous_cursor,
        ))
    }
}

impl PageEditorState {
    pub(super) fn mark_simple_table_composition_history_recorded(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    ) {
        let mut editor = self.tables.editor().borrow_mut();
        if let Some(active) = editor
            .as_mut()
            .filter(|active| active.address == *address && active.generation == generation)
        {
            active.composition_history_recorded = true;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct SimpleTableCompositionTarget<'a> {
    pub(super) address: &'a CardPageSimpleTableCellAddress,
    pub(super) generation: PageSimpleTableCellGeneration,
    pub(super) snapshot: &'a TextInputSnapshot,
    pub(super) replacement: Option<&'a PageSimpleTableCellObservedReplacement>,
}
