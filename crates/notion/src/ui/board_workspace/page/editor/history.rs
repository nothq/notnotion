use std::time::{Duration, Instant};

use gpui_components::text_input::floor_grapheme_boundary;

use super::{CardPage, Context, LoadedCardPageData, SurfaceState};
use crate::model::{
    CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex, CardPageWritableSimpleTableCell,
    PageColumnPair,
};
use crate::ui::surface::PageEditorState;
use crate::ui::{
    LoadedCardPageSimpleTableCellAccess, PageEditFocus, PageEditScope, PageEditSnapshot,
    PageTextEditGroup, PageTextEditGroupTarget, PageTextSelection,
};

use super::editing::PageEditSession;

mod column_compatibility;
mod restore;
mod table_compatibility;

pub(in crate::ui::board_workspace::page) use restore::PageHistoryRestoreState;

const PAGE_EDIT_HISTORY_LIMIT: usize = 100;
const PAGE_TEXT_EDIT_GROUP_INTERVAL: Duration = Duration::from_millis(750);

impl SurfaceState {
    pub(crate) fn handle_page_history_key_down(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if event.keystroke.key != "z"
            || !modifiers.platform
            || modifiers.alt
            || modifiers.control
            || modifiers.function
        {
            return false;
        }
        let page_id = match self.page_documents.selected_page.as_ref() {
            Some(crate::ui::CardPeekState::Loaded(page)) => page.data.page.block_id.clone(),
            _ => match self.page_documents.standalone.as_ref() {
                Some(page) => page.data.page.block_id.clone(),
                None => return false,
            },
        };
        if self
            .page_editor
            .page_link_icons
            .commit_in_flight_for_page(&page_id)
        {
            return true;
        }
        let current_focus = self.page_editor.current_page_edit_focus(cx);
        let transition = {
            let mut session = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            let restored = session.restore_page_edit_history_for_page(
                &page_id,
                modifiers.shift,
                current_focus,
            );
            session.finish(restored)
        };
        self.apply_page_edit_transition(transition, cx)
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page) fn record_page_structural_edit(
        &mut self,
        page: &CardPage,
        focus: Option<PageEditFocus>,
    ) {
        let text_selection = self.page_edit_text_selection(page);
        let history = self
            .page_edit_histories
            .entry(page.block_id.clone())
            .or_default();
        push_page_undo_snapshot(
            history,
            PageEditSnapshot {
                page: page.clone(),
                focus,
                text_selection,
                scope: PageEditScope::WholePage,
            },
        );
        history.text_group = None;
    }

    pub(super) fn record_page_text_edit(
        &mut self,
        page: &CardPage,
        block_id: &str,
        focus_offset: usize,
    ) {
        let now = Instant::now();
        let text_selection = self.page_edit_text_selection(page);
        let history = self
            .page_edit_histories
            .entry(page.block_id.clone())
            .or_default();
        let continues_group = history.text_group.as_ref().is_some_and(|group| {
            group.target == PageTextEditGroupTarget::Block(block_id.to_owned())
                && now.saturating_duration_since(group.last_edit_at)
                    <= PAGE_TEXT_EDIT_GROUP_INTERVAL
        });
        if !continues_group {
            push_page_undo_snapshot(
                history,
                PageEditSnapshot {
                    page: page.clone(),
                    focus: Some(PageEditFocus::Block {
                        block_id: block_id.to_string(),
                        offset: focus_offset,
                    }),
                    text_selection,
                    scope: PageEditScope::WholePage,
                },
            );
        }
        history.text_group = Some(PageTextEditGroup {
            target: PageTextEditGroupTarget::Block(block_id.to_string()),
            last_edit_at: now,
        });
    }

    pub(super) fn current_page_edit_focus(&self, cx: &gpui::App) -> Option<PageEditFocus> {
        if let Some(active) = self.tables.editor().borrow().as_ref() {
            let offset = active.input.as_ref()?.read(cx).snapshot().cursor;
            return Some(PageEditFocus::SimpleTableCell {
                address: active.address.clone(),
                offset,
            });
        }
        let block_id = self.active_page_block.as_ref()?.clone();
        let input = self
            .input
            .resource_state()
            .block_inputs
            .borrow()
            .get(&block_id)?
            .clone();
        let selection = input.read(cx).selection_range();
        Some(PageEditFocus::Block {
            block_id,
            offset: selection.end,
        })
    }

    pub(super) fn record_page_simple_table_cell_text_edit(
        &mut self,
        page: &CardPage,
        address: &CardPageSimpleTableCellAddress,
        focus_offset: usize,
    ) {
        let now = Instant::now();
        let target = PageTextEditGroupTarget::SimpleTableCell(address.clone());
        let history = self
            .page_edit_histories
            .entry(page.block_id.clone())
            .or_default();
        let continues_group = history.text_group.as_ref().is_some_and(|group| {
            group.target == target
                && now.saturating_duration_since(group.last_edit_at)
                    <= PAGE_TEXT_EDIT_GROUP_INTERVAL
        });
        if !continues_group {
            push_page_undo_snapshot(
                history,
                PageEditSnapshot {
                    page: page.clone(),
                    focus: Some(PageEditFocus::SimpleTableCell {
                        address: address.clone(),
                        offset: focus_offset,
                    }),
                    text_selection: None,
                    scope: PageEditScope::SimpleTableCell(address.clone()),
                },
            );
        }
        history.text_group = Some(PageTextEditGroup {
            target,
            last_edit_at: now,
        });
    }

    pub(super) fn record_page_simple_table_cell_structural_edit(
        &mut self,
        page: &CardPage,
        address: &CardPageSimpleTableCellAddress,
        focus_offset: usize,
    ) {
        let history = self
            .page_edit_histories
            .entry(page.block_id.clone())
            .or_default();
        push_page_undo_snapshot(
            history,
            PageEditSnapshot {
                page: page.clone(),
                focus: Some(PageEditFocus::SimpleTableCell {
                    address: address.clone(),
                    offset: focus_offset,
                }),
                text_selection: None,
                scope: PageEditScope::SimpleTableCell(address.clone()),
            },
        );
        history.text_group = None;
    }

    pub(in crate::ui::board_workspace::page) fn record_page_column_ratio_edit(
        &mut self,
        page: &CardPage,
        pair: PageColumnPair,
        focus: Option<PageEditFocus>,
    ) -> Result<(), String> {
        let mut baseline = page.clone();
        pair.materialize_source_weights(&mut baseline)?;
        let text_selection = self.page_edit_text_selection(page);
        let history = self
            .page_edit_histories
            .entry(page.block_id.clone())
            .or_default();
        push_page_undo_snapshot(
            history,
            PageEditSnapshot {
                page: baseline,
                focus,
                text_selection,
                scope: PageEditScope::ColumnRatios(pair),
            },
        );
        history.text_group = None;
        Ok(())
    }

    pub(super) fn page_edit_text_selection(
        &self,
        page: &CardPage,
    ) -> Option<crate::ui::PageTextSelection> {
        self.page_text_selection
            .as_ref()
            .filter(|selection| {
                page.blocks
                    .iter()
                    .any(|block| block.block_id == selection.anchor_block_id)
                    && page
                        .blocks
                        .iter()
                        .any(|block| block.block_id == selection.focus_block_id)
            })
            .cloned()
    }
}

fn normalize_page_text_selection(
    data: &LoadedCardPageData,
    mut selection: PageTextSelection,
) -> Option<PageTextSelection> {
    if !data.text_selection_is_visible(&selection) {
        return None;
    }
    selection.anchor_offset =
        normalize_page_offset(data, &selection.anchor_block_id, selection.anchor_offset)?;
    selection.focus_offset =
        normalize_page_offset(data, &selection.focus_block_id, selection.focus_offset)?;
    Some(selection)
}

fn normalize_page_focus(
    data: &LoadedCardPageData,
    focus: Option<PageEditFocus>,
) -> Option<PageEditFocus> {
    match focus? {
        PageEditFocus::Block { block_id, offset } => {
            let offset = normalize_page_offset(data, &block_id, offset)?;
            Some(PageEditFocus::Block { block_id, offset })
        }
        PageEditFocus::SimpleTableCell { address, offset } => {
            let projected = data.projected_simple_table_cell(&address)?;
            if projected.access != LoadedCardPageSimpleTableCellAccess::Writable {
                return None;
            }
            let index = CardPageSimpleTableCellIndex::new(&data.page).ok()?;
            let cell = index.cell(&data.page, &address).ok()?.clone();
            let writable = CardPageWritableSimpleTableCell::try_from(cell).ok()?;
            let offset = floor_grapheme_boundary(writable.as_cell().text(), offset);
            Some(PageEditFocus::SimpleTableCell { address, offset })
        }
    }
}

fn normalize_page_offset(
    data: &LoadedCardPageData,
    block_id: &str,
    offset: usize,
) -> Option<usize> {
    let index = *data.editable_block_indices.get(block_id)?;
    if !data.visible_block_mask.get(index).copied()? {
        return None;
    }
    let editable = data.page.blocks.get(index)?.editable_content()?;
    Some(floor_grapheme_boundary(&editable.text, offset))
}

fn push_page_undo_snapshot(history: &mut crate::ui::PageEditHistory, snapshot: PageEditSnapshot) {
    history.undo_stack.push(snapshot);
    if history.undo_stack.len() > PAGE_EDIT_HISTORY_LIMIT {
        history.undo_stack.remove(0);
    }
    history.redo_stack.clear();
}
