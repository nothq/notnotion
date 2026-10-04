use crate::model::{LoadRecentPagesRequest, SearchWorkspaceRequest};
use crate::ui::search::action::{
    QuickFindEffect, QuickFindFailure, QuickFindFailureRecovery, QuickFindRecentsCompletion,
    QuickFindRecentsJob, QuickFindRecentsSource, QuickFindRequest,
};

use super::controller::query_request;
use super::{NotionSearchRecentsCommit, NotionSearchRequestToken, NotionSearchState};

impl NotionSearchState {
    pub(crate) fn begin_quick_find_recents(
        &mut self,
        source: QuickFindRecentsSource,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        self.prepare_recents_cache(
            source.cache_scope,
            source.current_page_id,
            Some(source.current_board_url.clone()),
            source.persisted_results,
        );
        self.prepare_local_search_cache(source.local_search);
        let (request_token, showing_cached) = self.begin_recents_refresh(search_open);
        let mut effects = if showing_cached && search_open {
            self.load_selected_quick_find_preview()
        } else {
            Vec::new()
        };
        if let Some(request_token) = request_token {
            effects.push(recents_request(request_token, source.current_board_url));
        }
        effects.push(QuickFindEffect::Notify);
        effects
    }

    pub(super) fn complete_recents(
        &mut self,
        completion: QuickFindRecentsCompletion,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        let token = completion.request_token;
        let result = match completion.result {
            Ok(result) => result,
            Err(error) if self.recents_refresh_is_current(token) => {
                return vec![recents_failure(token, error)];
            }
            Err(_) => return Vec::new(),
        };
        let mut effects = match self.commit_recents_refresh(token, result.results, search_open) {
            NotionSearchRecentsCommit::Ignored => return Vec::new(),
            NotionSearchRecentsCommit::ShowingRecents => self.load_selected_quick_find_preview(),
            NotionSearchRecentsCommit::AwaitingDebouncedQuery => self.preview_after_settle(),
            NotionSearchRecentsCommit::RequeryImmediately => {
                let token = self.begin_request();
                let mut effects = self.preview_after_settle();
                effects.push(query_request(token));
                effects
            }
        };
        effects.push(QuickFindEffect::Notify);
        effects
    }

    pub(super) fn fail_quick_find_recents(
        &mut self,
        token: NotionSearchRequestToken,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        self.finish_recents_failure(token, search_open)
            .then_some(QuickFindEffect::Notify)
            .into_iter()
            .collect()
    }
}

fn recents_request(token: NotionSearchRequestToken, board_url: String) -> QuickFindEffect {
    QuickFindEffect::Request(QuickFindRequest::Recents(Box::new(QuickFindRecentsJob {
        request_token: token,
        request: LoadRecentPagesRequest {
            current_board_url: board_url,
            limit: u8::try_from(SearchWorkspaceRequest::MAX_RECENT_PAGES_FOR_BOOSTING)
                .expect("Notion recent-page boost limit must fit in a u8"),
        },
    })))
}

fn recents_failure(
    token: NotionSearchRequestToken,
    error: crate::model::NotionWorkspaceOperationFailure,
) -> QuickFindEffect {
    QuickFindEffect::Failure(Box::new(QuickFindFailure {
        operation: "recent pages failed",
        error,
        recovery: QuickFindFailureRecovery::Recents(token),
    }))
}
