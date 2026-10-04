use crate::ui::LoadedCardPage;

use super::{NotionSearchPreviewCacheEntry, NotionSearchState, NOTION_SEARCH_PREVIEW_CACHE_LIMIT};
impl NotionSearchState {
    pub(crate) fn cached_preview(&mut self, block_id: &str) -> Option<LoadedCardPage> {
        if self.page_mutation_is_dirty(block_id) {
            return None;
        }
        let index = self
            .preview_cache
            .iter()
            .position(|entry| entry.block_id == block_id)?;
        let entry = self
            .preview_cache
            .remove(index)
            .expect("located Quick Find preview cache entry must exist");
        let page = entry.page.clone();
        self.preview_cache.push_front(entry);
        Some(page)
    }

    pub(crate) fn preview_is_cached(&self, block_id: &str) -> bool {
        !self.page_mutation_is_dirty(block_id)
            && self
                .preview_cache
                .iter()
                .any(|entry| entry.block_id == block_id)
    }

    pub(crate) fn cache_preview(&mut self, block_id: String, page: LoadedCardPage) {
        if self.page_mutation_is_dirty(&block_id) {
            return;
        }
        self.preview_cache
            .retain(|entry| entry.block_id != block_id);
        self.preview_cache
            .push_front(NotionSearchPreviewCacheEntry { block_id, page });
        self.preview_cache
            .truncate(NOTION_SEARCH_PREVIEW_CACHE_LIMIT);
    }
}
