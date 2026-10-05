use std::collections::{HashMap, VecDeque};

use super::quick_find::{
    next_quick_find_authority_version_after, quick_find_unix_timestamp_millis,
};
use super::{
    QuickFindLocalSearchState, QuickFindQueryAuthority, QuickFindQueryCacheWrite,
    QuickFindQueryIdentity, QUICK_FIND_PENDING_QUERY_LIMIT,
};
use crate::model::{
    quick_find_local_query_key, PageShellSearchResult, QuickFindLocalSearchCache,
    SearchWorkspaceRequest,
};

impl QuickFindQueryIdentity {
    fn from_request(request: &SearchWorkspaceRequest) -> Self {
        Self {
            session_id: request.search_session_id.clone(),
            flow_number: request.flow_number,
        }
    }
}

impl QuickFindLocalSearchState {
    #[cfg(test)]
    pub(super) fn new(cache: QuickFindLocalSearchCache) -> Self {
        Self::new_after(cache, 0)
    }

    pub(super) fn new_after(
        cache: QuickFindLocalSearchCache,
        persisted_authority_version: u64,
    ) -> Self {
        Self {
            cache,
            authority_version: next_quick_find_authority_version_after(persisted_authority_version),
            pending_queries: HashMap::new(),
            pending_query_order: VecDeque::new(),
        }
    }

    pub(super) fn begin_query(&mut self, request: &SearchWorkspaceRequest) {
        let identity = QuickFindQueryIdentity::from_request(request);
        self.pending_query_order
            .retain(|pending| pending != &identity);
        self.pending_queries.insert(
            identity.clone(),
            QuickFindQueryAuthority {
                version: self.authority_version,
                started_at_unix_millis: quick_find_unix_timestamp_millis(),
            },
        );
        self.pending_query_order.push_back(identity);
        while self.pending_query_order.len() > QUICK_FIND_PENDING_QUERY_LIMIT {
            let evicted = self
                .pending_query_order
                .pop_front()
                .expect("an over-capacity Quick Find request queue must have a front entry");
            self.pending_queries.remove(&evicted);
        }
    }

    pub(super) fn cancel_query(&mut self, request: &SearchWorkspaceRequest) {
        let identity = QuickFindQueryIdentity::from_request(request);
        self.pending_queries.remove(&identity);
        self.pending_query_order
            .retain(|pending| pending != &identity);
    }

    pub(super) fn accept_query_response(
        &mut self,
        request: &SearchWorkspaceRequest,
        results: Vec<PageShellSearchResult>,
        response_is_complete: bool,
        accepted_at_unix_millis: u64,
    ) -> Option<QuickFindQueryCacheWrite> {
        let identity = QuickFindQueryIdentity::from_request(request);
        let authority = self.pending_queries.remove(&identity)?;
        self.pending_query_order
            .retain(|pending| pending != &identity);
        if authority.version != self.authority_version {
            return None;
        }
        let accepted_at_unix_millis = accepted_at_unix_millis.max(authority.started_at_unix_millis);
        let query_key = quick_find_local_query_key(request.scope, &request.query);
        if response_is_complete {
            self.cache
                .record_complete_query_results(&query_key, results.iter().cloned());
        } else {
            self.cache
                .record_partial_query_results(&query_key, results.iter().cloned());
        }
        Some(QuickFindQueryCacheWrite {
            query_key,
            results,
            response_is_complete,
            authority_version: authority.version,
            accepted_at_unix_millis,
        })
    }

    pub(super) fn reset_authority(
        &mut self,
        pages: impl IntoIterator<Item = PageShellSearchResult>,
    ) {
        self.advance_authority();
        self.cache.invalidate_query_authority(pages);
    }

    fn advance_authority(&mut self) {
        self.advance_authority_after(0);
    }

    fn advance_authority_after(&mut self, minimum: u64) {
        self.authority_version =
            next_quick_find_authority_version_after(self.authority_version.max(minimum));
        self.pending_queries.clear();
        self.pending_query_order.clear();
    }

    pub(super) fn invalidate_page_mutation(&mut self, page_id: &str) -> u64 {
        self.advance_authority();
        self.cache.invalidate_page_mutation(page_id);
        self.authority_version
    }
}
