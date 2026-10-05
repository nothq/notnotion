use std::{cell::RefCell, rc::Rc};

use super::super::{
    LoadedCardPageData, PageSimpleTableCellEditorState, PageSimpleTableScrollState, ScrollHandle,
};

#[derive(Clone, Default)]
pub(crate) struct PageSimpleTableRuntime {
    editor: Rc<PageSimpleTableCellEditorState>,
    scroll: Rc<RefCell<PageSimpleTableScrollState>>,
}

impl PageSimpleTableRuntime {
    pub(crate) fn editor(&self) -> &PageSimpleTableCellEditorState {
        &self.editor
    }

    pub(crate) fn scroll_handle(&self, page_id: &str, table_block_id: &str) -> ScrollHandle {
        self.scroll.borrow_mut().handle(page_id, table_block_id)
    }

    pub(crate) fn retain_valid_scroll_handles(&self, data: &LoadedCardPageData) {
        self.scroll.borrow_mut().retain_valid(data);
    }
}
