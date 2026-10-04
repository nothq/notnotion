use crate::ui::surface::PageDocuments;
use crate::ui::BoardSnapshot;

pub(super) fn has_main_content(board: &BoardSnapshot, documents: &PageDocuments) -> bool {
    board.has_collection_content() || documents.standalone.is_some()
}

impl BoardSnapshot {
    pub(super) fn has_collection_content(&self) -> bool {
        !self.columns.is_empty() || !self.view_tabs.is_empty() || !self.items.is_empty()
    }
}
