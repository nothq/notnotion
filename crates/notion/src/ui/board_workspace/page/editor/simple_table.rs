use gpui::App;

use crate::model::{CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex};
use crate::ui::surface::{PageEditorState, PageSimpleTableCellFocusMode};
use crate::ui::LoadedCardPageSimpleTableCellAccess;

use super::editing::{PageEditEffect, PageEditSession};

mod actions;
mod change;
mod focus;
mod formatting;
mod input;
pub(super) mod navigation;

pub(in crate::ui::board_workspace) use actions::PageSimpleTableEditAction;
pub(crate) use change::PageSimpleTableEditorEffect;
pub(in crate::ui::board_workspace::page::editor) use change::PageSimpleTableHostEffect;
pub(in crate::ui::board_workspace::page::editor) use focus::{
    PageSimpleTableCellMountRequest, PageSimpleTableColumnWidthSpec,
};
pub(in crate::ui::board_workspace) use formatting::PageSimpleTableFormatAction;
pub(crate) use formatting::PageSimpleTableShortcut;
pub(super) use input::simple_table_on_key_down;
pub(in crate::ui::board_workspace::page::editor) use input::PageSimpleTableCellInputRequest;

impl PageEditSession<'_> {
    pub(crate) fn activate_page_simple_table_cell(
        &mut self,
        page_id: &str,
        address: CardPageSimpleTableCellAddress,
        focus: PageSimpleTableCellFocusMode,
        cx: &mut App,
    ) -> bool {
        let Some(data) = self.documents.active_page_data_with_id(page_id) else {
            return false;
        };
        if !simple_table_cell_is_writable(self.documents, &data, page_id, &address) {
            return false;
        }
        if self
            .editor
            .tables
            .editor()
            .activate(page_id.to_owned(), address.clone(), focus)
            .is_none()
        {
            return false;
        }
        self.editor
            .clear_page_interactions_for_simple_table_cell(cx);
        self.documents
            .reveal_page_simple_table_cell(&data, &address);
        self.effects.push(PageEditEffect::Notify);
        true
    }
}

fn simple_table_cell_is_writable(
    documents: &crate::ui::surface::PageDocuments,
    data: &crate::ui::LoadedCardPageData,
    page_id: &str,
    address: &CardPageSimpleTableCellAddress,
) -> bool {
    data.projected_simple_table_cell(address)
        .is_some_and(|cell| cell.access == LoadedCardPageSimpleTableCellAccess::Writable)
        && writable_model_cell(&data.page, address)
        && documents
            .page_authority_with_id(page_id)
            .is_some_and(|authority| writable_model_cell(&authority, address))
}

fn writable_model_cell(
    page: &crate::model::CardPage,
    address: &CardPageSimpleTableCellAddress,
) -> bool {
    CardPageSimpleTableCellIndex::new(page)
        .and_then(|index| index.cell(page, address).cloned())
        .and_then(crate::model::CardPageWritableSimpleTableCell::try_from)
        .is_ok()
}

impl PageEditorState {
    fn clear_page_interactions_for_simple_table_cell(&mut self, cx: &mut App) {
        self.active_page_block = None;
        self.input
            .resource_state()
            .focus_request
            .borrow_mut()
            .take();
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_block_context_menu = None;
        self.page_link_icons.clear_picker();
        self.page_forced_text_annotations = None;
        self.page_rich_text_dialog = None;
        self.clear_page_document_selection(cx);
    }
}
