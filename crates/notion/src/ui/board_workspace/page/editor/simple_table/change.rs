use gpui_components::text_input::TextInputSnapshot;

use crate::model::{
    CardPage, CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex,
    CardPageWritableSimpleTableCell, PageMutation, ReplacePageSimpleTableCellRequest,
};
use crate::ui::surface::PageSimpleTableCellGeneration;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};

mod composition;
mod replacement;
mod target;

pub(crate) use composition::PageSimpleTableEditorEffect;
use target::{
    PreparedSimpleTableCellChange, SimpleTableCellChangeAttempt, SimpleTableCellChangeTarget,
};

pub(in crate::ui::board_workspace::page::editor) enum PageSimpleTableHostEffect {
    Error(String),
    ReconcileAuthority {
        authority: Option<CardPage>,
        error: String,
    },
}

impl PageEditSession<'_> {
    pub(super) fn finish_page_simple_table_cell_blur(
        &mut self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
    ) -> Option<PageSimpleTableHostEffect> {
        let (page_id, dirty, composing) = {
            let mut active = self.editor.tables.editor().borrow_mut();
            let active = active
                .as_mut()
                .filter(|active| active.address == *address && active.generation == generation)?;
            if active.composition_active {
                active.retire_after_composition = true;
            }
            (
                active.page_id.clone(),
                active.composition_dirty,
                active.composition_active,
            )
        };
        if composing {
            return None;
        }
        if dirty {
            if let Err(error) = self.persist_deferred_simple_table_cell(&page_id, address) {
                return Some(self.reject_page_simple_table_cell_composition(
                    &page_id, address, generation, error,
                ));
            }
        }
        if self.editor.tables.editor().matches(address, generation) {
            self.editor.tables.editor().clear();
            self.effects.push(PageEditEffect::Notify);
        }
        None
    }

    fn persist_deferred_simple_table_cell(
        &mut self,
        page_id: &str,
        address: &CardPageSimpleTableCellAddress,
    ) -> Result<(), String> {
        let target = self
            .editor
            .tables
            .editor()
            .borrow()
            .as_ref()
            .filter(|active| active.page_id == page_id && active.address == *address)
            .and_then(|active| active.optimistic_composition_cell.clone())
            .ok_or_else(|| "deferred table-cell edit lost its local value".to_string())?;
        let authority = self
            .documents
            .page_authority_with_id(page_id)
            .ok_or_else(|| {
                format!("live authority for page {page_id} disappeared before IME persistence")
            })?;
        let request = ReplacePageSimpleTableCellRequest::new(
            &authority,
            address.clone(),
            target.into_cell(),
        )?;
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page_id.to_string(),
                mutation: PageMutation::ReplaceSimpleTableCell(request),
            },
        ));
        Ok(())
    }

    pub(in crate::ui::board_workspace::page::editor) fn apply_page_simple_table_cell_change(
        &mut self,
        page_id: &str,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: TextInputSnapshot,
    ) -> Option<PageSimpleTableHostEffect> {
        let attempt = self.simple_table_cell_change_attempt(address, generation, &snapshot);
        if !self.editor.tables.editor().matches(address, generation) {
            return None;
        }
        match self.try_apply_page_simple_table_cell_change(page_id, address, generation, snapshot) {
            Ok(()) => None,
            Err(error) if attempt == SimpleTableCellChangeAttempt::CompositionCommit => Some(
                self.reject_page_simple_table_cell_composition(page_id, address, generation, error),
            ),
            Err(error) => Some(PageSimpleTableHostEffect::Error(error)),
        }
    }

    fn simple_table_cell_change_attempt(
        &self,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: &TextInputSnapshot,
    ) -> SimpleTableCellChangeAttempt {
        let composition_commit = !snapshot.is_composing
            && self
                .editor
                .tables
                .editor()
                .borrow()
                .as_ref()
                .is_some_and(|active| {
                    active.address == *address
                        && active.generation == generation
                        && active.composition_active
                });
        if composition_commit {
            SimpleTableCellChangeAttempt::CompositionCommit
        } else {
            SimpleTableCellChangeAttempt::Ordinary
        }
    }

    fn reject_page_simple_table_cell_composition(
        &mut self,
        page_id: &str,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        error: String,
    ) -> PageSimpleTableHostEffect {
        if self.editor.tables.editor().matches(address, generation) {
            self.editor.tables.editor().clear();
        }
        if let Some(history) = self.editor.page_edit_histories.get_mut(page_id) {
            history.undo_stack.clear();
            history.redo_stack.clear();
            history.text_group = None;
        }
        let authority = self
            .documents
            .page_authority_with_id(page_id)
            .map(|page| page.as_ref().clone());
        PageSimpleTableHostEffect::ReconcileAuthority { authority, error }
    }

    fn try_apply_page_simple_table_cell_change(
        &mut self,
        page_id: &str,
        address: &CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: TextInputSnapshot,
    ) -> Result<(), String> {
        let target = SimpleTableCellChangeTarget {
            page_id,
            address,
            generation,
            snapshot: &snapshot,
        };
        let prepared = self.prepare_simple_table_cell_change(target)?;
        self.apply_prepared_simple_table_cell_change(target, prepared)
    }

    fn apply_prepared_simple_table_cell_change(
        &mut self,
        target: SimpleTableCellChangeTarget<'_>,
        prepared: PreparedSimpleTableCellChange,
    ) -> Result<(), String> {
        let text_changed = prepared.replacement.is_some();
        let composition_target = target.composition(prepared.replacement.as_ref());
        if !text_changed && !prepared.lifecycle.persist && !prepared.lifecycle.retire_after_commit {
            PageSimpleTableEditorEffect::finish_change(
                composition_target,
                text_changed,
                false,
                &prepared.lifecycle,
            )
            .apply(self.editor);
            self.effects.push(PageEditEffect::Notify);
            return Ok(());
        }
        let history_recorded = self.record_simple_table_cell_change_history(
            &prepared.visual_page,
            composition_target,
            &prepared.lifecycle,
        )?;
        if history_recorded && (target.snapshot.is_composing || prepared.lifecycle.committed) {
            self.editor
                .mark_simple_table_composition_history_recorded(target.address, target.generation);
        }
        let request = self.simple_table_cell_write_request(
            target.page_id,
            target.address,
            &prepared.target,
            prepared.lifecycle.persist,
        )?;
        self.install_simple_table_cell_visual(
            &prepared.visual_page,
            target.address,
            prepared.target,
            target.snapshot.is_composing,
        )?;
        if let Some(request) = request {
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::EnqueueMutation {
                    page_id: target.page_id.to_string(),
                    mutation: PageMutation::ReplaceSimpleTableCell(request),
                },
            ));
        }
        let completion = PageSimpleTableEditorEffect::finish_change(
            composition_target,
            text_changed,
            history_recorded,
            &prepared.lifecycle,
        );
        self.effects
            .push(PageEditEffect::Editor(PageEditorEffect::SimpleTable(
                completion,
            )));
        self.effects.push(PageEditEffect::Notify);
        Ok(())
    }

    fn prepare_simple_table_cell_change(
        &self,
        target: SimpleTableCellChangeTarget<'_>,
    ) -> Result<PreparedSimpleTableCellChange, String> {
        let visual_page = self.documents.page_with_id(target.page_id).ok_or_else(|| {
            format!(
                "loaded page {} disappeared during table-cell edit",
                target.page_id
            )
        })?;
        let editor_source = self.editor.simple_table_cell_editor_source(
            &visual_page,
            target.address,
            target.generation,
        )?;
        let replacement = self.editor.take_simple_table_cell_replacement(
            target.address,
            target.generation,
            editor_source.as_cell().text(),
            target.snapshot,
        )?;
        let text_changed = replacement.is_some();
        let lifecycle = self.editor.simple_table_composition_lifecycle(
            target.address,
            target.generation,
            target.snapshot,
            text_changed,
        );
        let target_cell = match replacement.as_ref() {
            Some(replacement) => self.editor.changed_simple_table_cell(
                target.address,
                target.generation,
                editor_source.as_cell(),
                replacement,
            )?,
            None => editor_source.clone(),
        };
        self.editor.stage_simple_table_composition(
            target.composition(replacement.as_ref()),
            editor_source,
            target_cell.clone(),
            &lifecycle,
        );
        Ok(PreparedSimpleTableCellChange {
            visual_page,
            replacement,
            target: target_cell,
            lifecycle,
        })
    }

    fn simple_table_cell_write_request(
        &self,
        page_id: &str,
        address: &CardPageSimpleTableCellAddress,
        target: &CardPageWritableSimpleTableCell,
        persist: bool,
    ) -> Result<Option<ReplacePageSimpleTableCellRequest>, String> {
        if !persist {
            return Ok(None);
        }
        let authority = self
            .documents
            .page_authority_with_id(page_id)
            .ok_or_else(|| {
                format!("live authority for page {page_id} disappeared during table-cell edit")
            })?;
        ReplacePageSimpleTableCellRequest::new(
            &authority,
            address.clone(),
            target.as_cell().clone(),
        )
        .map(Some)
    }

    fn install_simple_table_cell_visual(
        &mut self,
        visual_page: &CardPage,
        address: &CardPageSimpleTableCellAddress,
        target: CardPageWritableSimpleTableCell,
        composing: bool,
    ) -> Result<(), String> {
        if composing || !simple_table_cell_differs(visual_page, address, &target)? {
            return Ok(());
        }
        let optimistic = page_with_simple_table_cell(visual_page, address, target)?;
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(optimistic));
        Ok(())
    }
}

fn writable_simple_table_cell(
    page: &CardPage,
    address: &CardPageSimpleTableCellAddress,
) -> Result<CardPageWritableSimpleTableCell, String> {
    let index = CardPageSimpleTableCellIndex::new(page)?;
    CardPageWritableSimpleTableCell::try_from(index.cell(page, address)?.clone())
}

fn page_with_simple_table_cell(
    page: &CardPage,
    address: &CardPageSimpleTableCellAddress,
    cell: CardPageWritableSimpleTableCell,
) -> Result<CardPage, String> {
    let index = CardPageSimpleTableCellIndex::new(page)?;
    let mut page = page.clone();
    index.replace(&mut page, address, cell)?;
    Ok(page)
}

fn simple_table_cell_differs(
    page: &CardPage,
    address: &CardPageSimpleTableCellAddress,
    target: &CardPageWritableSimpleTableCell,
) -> Result<bool, String> {
    let index = CardPageSimpleTableCellIndex::new(page)?;
    Ok(index.cell(page, address)? != target.as_cell())
}
