use crate::model::CardPage;
use crate::ui::surface::PageDocuments;
use crate::ui::CardPeekState;

impl PageDocuments {
    pub(super) fn loaded_comment_page(&self, page_id: &str) -> Option<&CardPage> {
        if let Some(CardPeekState::Loaded(page)) = self.selected_page.as_ref() {
            if page.data.page.block_id == page_id {
                return Some(&page.data.page);
            }
        }
        self.standalone
            .as_ref()
            .map(|page| &page.data.page)
            .filter(|page| page.block_id == page_id)
    }

    pub(crate) fn loaded_writable_comment_target(
        &self,
        target_id: &str,
    ) -> Option<(String, String)> {
        let page = match self.selected_page.as_ref() {
            Some(CardPeekState::Loaded(page)) => Some(&page.data.page),
            _ => self.standalone.as_ref().map(|page| &page.data.page),
        }?;
        if !page.comments_writable || !page.blocks.iter().any(|block| block.block_id == target_id) {
            return None;
        }
        Some((page.block_id.clone(), target_id.to_string()))
    }
}
