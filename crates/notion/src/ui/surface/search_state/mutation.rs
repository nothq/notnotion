use gpui::ScrollHandle;

use crate::model::{notion_page_identity_key, PageShellSearchResult};
use crate::ui::{Arc, NotionWorkspaceApi};

use super::{
    next_notion_search_request_token, NotionSearchPreviewState, NotionSearchRecentsCache,
    NotionSearchResultsState, NotionSearchState,
};
impl NotionSearchState {
    pub(crate) fn invalidate_page_mutation(&mut self, block_id: &str) {
        let identity = notion_page_identity_key(block_id);
        self.invalidate_page_cache(&identity);
    }

    pub(crate) fn begin_page_mutation(&mut self, lane_page_id: &str, block_id: &str) -> bool {
        let lane_identity = notion_page_identity_key(lane_page_id);
        let page_identity = notion_page_identity_key(block_id);
        if !self
            .dirty_page_identities
            .insert((lane_identity, page_identity.clone()))
        {
            return false;
        }
        self.invalidate_page_cache(&page_identity);
        true
    }

    pub(crate) fn dirty_page_identities_for_lane(&self, lane_page_id: &str) -> Vec<String> {
        let lane_identity = notion_page_identity_key(lane_page_id);
        self.dirty_page_identities
            .iter()
            .filter(|(lane, _)| lane == &lane_identity)
            .map(|(_, identity)| identity.clone())
            .collect()
    }

    pub(crate) fn finish_page_mutation_lane(&mut self, lane_page_id: &str) -> Vec<String> {
        let lane_identity = notion_page_identity_key(lane_page_id);
        let identities = self.dirty_page_identities_for_lane(&lane_identity);
        for identity in &identities {
            self.invalidate_page_cache(identity);
        }
        self.dirty_page_identities
            .retain(|(lane, _)| lane != &lane_identity);
        identities
    }

    pub(crate) fn page_mutation_is_dirty(&self, block_id: &str) -> bool {
        let identity = notion_page_identity_key(block_id);
        self.dirty_page_identities
            .iter()
            .any(|(_, dirty_identity)| dirty_identity == &identity)
    }

    pub(crate) fn mutation_lane_is_dirty(&self, lane_page_id: &str) -> bool {
        let lane_identity = notion_page_identity_key(lane_page_id);
        self.dirty_page_identities
            .iter()
            .any(|(lane, _)| lane == &lane_identity)
    }

    pub(crate) fn persist_page_mutation(
        &mut self,
        lane_page_id: &str,
        workspace_api: &dyn NotionWorkspaceApi,
    ) {
        let page_ids = self.dirty_page_identities_for_lane(lane_page_id);
        for page_id in page_ids {
            self.invalidate_page_mutation(&page_id);
            workspace_api.invalidate_quick_find_page_mutation(&page_id);
        }
    }

    fn invalidate_page_cache(&mut self, identity: &str) {
        self.invalidate_query_cache(identity);
        self.invalidate_recents_cache(identity);
        self.invalidate_preview_cache(identity);
        self.invalidate_visible_results(identity);
    }

    fn invalidate_query_cache(&mut self, identity: &str) {
        self.request_token = next_notion_search_request_token();
        self.query_request_dispatched = false;
        self.local_request = None;
        if let Some((_, previous_limit)) = self.pagination_request.take() {
            self.result_limit = previous_limit;
        }
        self.query_cache.clear();
        self.local_search_cache.invalidate_page_mutation(identity);
    }

    fn invalidate_recents_cache(&mut self, identity: &str) {
        self.recents_refresh_token = None;
        self.recents_dirty_during_refresh.clear();
        if let NotionSearchRecentsCache::Loaded {
            refreshed_at,
            results,
        } = &mut self.recents_cache
        {
            *refreshed_at = None;
            *results = results
                .iter()
                .filter(|result| notion_page_identity_key(&result.page.block_id) != identity)
                .cloned()
                .collect();
        }
    }

    fn invalidate_preview_cache(&mut self, identity: &str) {
        self.preview_requests
            .retain(|pending_id, _| notion_page_identity_key(pending_id) != identity);
        self.preview_cache
            .retain(|entry| notion_page_identity_key(&entry.block_id) != identity);
        let preview_is_stale = match &self.preview {
            NotionSearchPreviewState::Loading {
                block_id: pending, ..
            }
            | NotionSearchPreviewState::Loaded {
                block_id: pending, ..
            }
            | NotionSearchPreviewState::Failed { block_id: pending } => {
                notion_page_identity_key(pending.as_ref()) == identity
            }
            NotionSearchPreviewState::Idle => false,
        };
        if preview_is_stale {
            self.preview = NotionSearchPreviewState::Idle;
            self.preview_scroll_handle = ScrollHandle::new();
        }
    }

    fn invalidate_visible_results(&mut self, identity: &str) {
        if self.query.trim().is_empty() {
            if !self.show_cached_recents() {
                self.results = NotionSearchResultsState::Idle;
                self.clear_list();
            }
            return;
        }

        let NotionSearchResultsState::Loaded {
            total,
            has_more,
            results,
        } = self.results.clone()
        else {
            self.results = NotionSearchResultsState::Idle;
            self.clear_list();
            return;
        };
        let previous_count = results.len();
        let results: Arc<[PageShellSearchResult]> = results
            .iter()
            .filter(|result| notion_page_identity_key(&result.block_id) != identity)
            .cloned()
            .collect();
        let removed_count = previous_count.saturating_sub(results.len()) as u32;
        self.reconcile_selected_result(&results);
        self.show_query_rows(results.len(), true);
        self.results = NotionSearchResultsState::Loaded {
            total: total.saturating_sub(removed_count),
            has_more,
            results,
        };
    }
}
