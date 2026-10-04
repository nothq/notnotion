use crate::ui::board_workspace::PageDocumentAction;
use gpui::{Context, ScrollHandle, Window};
use gpui_components::text_input::TextInputSnapshot;

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::board_workspace::PageEditSession;
use crate::ui::surface::PageSimpleTableCellGeneration;
use crate::ui::{PageEditFocus, SurfaceState};

use super::super::super::simple_table::{
    PageSimpleTableCellMountRequest, PageSimpleTableColumnWidthSpec, PageSimpleTableEditAction,
    PageSimpleTableHostEffect,
};

pub(in crate::ui::board_workspace) enum PageTableRenderAction {
    Edit(PageSimpleTableEditAction),
    CompleteMount {
        address: CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        scroll_handle: ScrollHandle,
        column_widths: Vec<PageSimpleTableColumnWidthSpec>,
        column_index: usize,
    },
    SelectionChanged {
        address: CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        snapshot: TextInputSnapshot,
    },
    RestoreHistory(PageTableHistoryRestore),
}

/// An undo or redo from inside a mounted table cell, which refocuses the cell
/// at `cursor`.
pub(in crate::ui::board_workspace) struct PageTableHistoryRestore {
    pub(in crate::ui::board_workspace) page_id: String,
    pub(in crate::ui::board_workspace) address: CardPageSimpleTableCellAddress,
    pub(in crate::ui::board_workspace) generation: PageSimpleTableCellGeneration,
    pub(in crate::ui::board_workspace) cursor: usize,
    pub(in crate::ui::board_workspace) redo: bool,
}

pub(super) fn handle(
    surface: &mut SurfaceState,
    action: PageTableRenderAction,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageTableRenderAction::Edit(action) => {
            let transition = {
                let mut edit =
                    PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                let host = edit.apply_page_simple_table_action(action, cx);
                edit.finish(host)
            };
            let host = surface.apply_page_edit_transition(transition, cx);
            finish_table_host_effect(surface, host, cx);
        }
        PageTableRenderAction::CompleteMount {
            address,
            generation,
            scroll_handle,
            column_widths,
            column_index,
        } => surface.page_editor.complete_page_simple_table_cell_mount(
            PageSimpleTableCellMountRequest::new(
                &address,
                generation,
                &scroll_handle,
                &column_widths,
                column_index,
            ),
            window,
            cx,
        ),
        PageTableRenderAction::SelectionChanged {
            address,
            generation,
            snapshot,
        } => {
            if surface
                .page_editor
                .update_page_simple_table_cell_selection(&address, generation, snapshot)
            {
                cx.notify();
            }
        }
        PageTableRenderAction::RestoreHistory(restore) => restore_history(surface, restore, cx),
    }
}

fn finish_table_host_effect(
    surface: &mut SurfaceState,
    effect: Option<PageSimpleTableHostEffect>,
    cx: &mut Context<SurfaceState>,
) {
    match effect {
        Some(PageSimpleTableHostEffect::Error(error)) => surface.print_notion_error(error),
        Some(PageSimpleTableHostEffect::ReconcileAuthority { authority, error }) => {
            if let Some(mut page) = authority {
                let authority = surface
                    .page_mutations
                    .reconcile_external_load(&mut page, None);
                surface.dispatch_page_document_action(
                    PageDocumentAction::replace_loaded_and_authority(page, authority),
                    cx,
                );
            }
            cx.notify();
            surface.print_notion_error(error);
        }
        None => {}
    }
}

fn restore_history(
    surface: &mut SurfaceState,
    restore: PageTableHistoryRestore,
    cx: &mut Context<SurfaceState>,
) {
    let PageTableHistoryRestore {
        page_id,
        address,
        generation,
        cursor,
        redo,
    } = restore;
    if !surface
        .page_editor
        .tables
        .editor()
        .matches(&address, generation)
    {
        return;
    }
    let transition = {
        let mut edit = PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
        let result = edit.restore_page_edit_history_for_page(
            &page_id,
            redo,
            Some(PageEditFocus::SimpleTableCell {
                address,
                offset: cursor,
            }),
        );
        edit.finish(result)
    };
    surface.apply_page_edit_transition(transition, cx);
}
