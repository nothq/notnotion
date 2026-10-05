use super::{
    normalized_local_search_query, notion_page_identity_key, PageShellSearchBadge,
    PageShellSearchResult, QuickFindIndexedQuery, QuickFindLocalSearchCache,
};

impl QuickFindLocalSearchCache {
    pub const MAX_PAGES: usize = 512;
    pub const MAX_INDEXED_QUERIES: usize = 64;

    pub fn query_is_indexed(&self, query: &str) -> bool {
        self.indexed_result_ids(query).is_some()
    }

    pub fn indexed_result_ids(&self, query: &str) -> Option<&[String]> {
        let query = normalized_local_search_query(query);
        self.indexed_queries
            .iter()
            .find(|indexed| indexed.query == query)
            .map(|indexed| indexed.result_ids.as_slice())
    }

    pub(crate) fn add_provisional_pages(
        &mut self,
        pages: impl IntoIterator<Item = PageShellSearchResult>,
    ) {
        for page in pages {
            let identity = notion_page_identity_key(&page.block_id);
            if self
                .pages
                .iter()
                .any(|existing| notion_page_identity_key(&existing.block_id) == identity)
            {
                continue;
            }
            self.pages.push(sanitize_local_search_page(page));
        }
        self.pages.truncate(Self::MAX_PAGES);
    }

    pub(crate) fn merge_from(&mut self, incoming: Self) {
        for indexed_query in incoming.indexed_queries.into_iter().rev() {
            self.indexed_queries
                .retain(|indexed| indexed.query != indexed_query.query);
            self.indexed_queries.insert(0, indexed_query);
        }
        self.indexed_queries.truncate(Self::MAX_INDEXED_QUERIES);
        for page in incoming.pages.into_iter().rev() {
            let identity = notion_page_identity_key(&page.block_id);
            self.pages
                .retain(|existing| notion_page_identity_key(&existing.block_id) != identity);
            self.pages.insert(0, sanitize_local_search_page(page));
        }
        self.pages.truncate(Self::MAX_PAGES);
    }

    pub(crate) fn merge_query_results(
        &mut self,
        pages: impl IntoIterator<Item = PageShellSearchResult>,
    ) {
        let incoming = pages
            .into_iter()
            .map(sanitize_local_search_page)
            .collect::<Vec<_>>();
        for page in incoming.into_iter().rev() {
            let identity = notion_page_identity_key(&page.block_id);
            self.pages
                .retain(|existing| notion_page_identity_key(&existing.block_id) != identity);
            self.pages.insert(0, page);
        }
        self.pages.truncate(Self::MAX_PAGES);
    }

    pub(crate) fn record_complete_query_results(
        &mut self,
        query: &str,
        pages: impl IntoIterator<Item = PageShellSearchResult>,
    ) {
        let query = normalized_local_search_query(query);
        let pages = pages.into_iter().collect::<Vec<_>>();
        if !query.is_empty() {
            let mut seen = std::collections::HashSet::new();
            let result_ids = pages
                .iter()
                .map(|page| notion_page_identity_key(&page.block_id))
                .filter(|identity| seen.insert(identity.clone()))
                .take(Self::MAX_PAGES)
                .collect();
            self.indexed_queries
                .retain(|indexed| indexed.query != query);
            self.indexed_queries
                .insert(0, QuickFindIndexedQuery { query, result_ids });
            self.indexed_queries.truncate(Self::MAX_INDEXED_QUERIES);
        }
        self.merge_query_results(pages);
    }

    pub(crate) fn record_partial_query_results(
        &mut self,
        query: &str,
        pages: impl IntoIterator<Item = PageShellSearchResult>,
    ) {
        let query = normalized_local_search_query(query);
        let pages = pages.into_iter().collect::<Vec<_>>();
        if !query.is_empty() {
            let mut result_ids = self
                .indexed_queries
                .iter()
                .find(|indexed| indexed.query == query)
                .map(|indexed| indexed.result_ids.clone())
                .unwrap_or_default();
            let mut seen = result_ids
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<_>>();
            result_ids.extend(
                pages
                    .iter()
                    .map(|page| notion_page_identity_key(&page.block_id))
                    .filter(|identity| seen.insert(identity.clone())),
            );
            result_ids.truncate(Self::MAX_PAGES);
            self.indexed_queries
                .retain(|indexed| indexed.query != query);
            self.indexed_queries
                .insert(0, QuickFindIndexedQuery { query, result_ids });
            self.indexed_queries.truncate(Self::MAX_INDEXED_QUERIES);
        }
        self.merge_query_results(pages);
    }

    pub(crate) fn invalidate_query_authority(
        &mut self,
        provisional_pages: impl IntoIterator<Item = PageShellSearchResult>,
    ) {
        self.indexed_queries.clear();
        self.merge_query_results(provisional_pages);
    }

    /// Revokes every query membership after a page mutation and drops that page's stale details.
    ///
    /// A content edit can make the page enter or leave any previously indexed query, so no indexed
    /// query remains authoritative. Other cached pages remain useful as provisional candidates.
    pub(crate) fn invalidate_page_mutation(&mut self, page_id: &str) {
        let identity = notion_page_identity_key(page_id);
        self.pages
            .retain(|page| notion_page_identity_key(&page.block_id) != identity);
        self.indexed_queries.clear();
    }
}

fn sanitize_local_search_page(mut page: PageShellSearchResult) -> PageShellSearchResult {
    page.match_snippet = None;
    page.edited_label = None;
    page.badges
        .retain(|badge| *badge != PageShellSearchBadge::CurrentPage);
    page
}
