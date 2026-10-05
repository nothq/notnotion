use crate::model::{
    PageShellSearchBadge, RecentPageResult, SearchWorkspaceRequest, SearchWorkspaceScope,
};
use crate::ui::search::action::{
    quick_find_should_prefetch, QuickFindAcceptedQuery, QuickFindAction, QuickFindActionInput,
    QuickFindCompletion, QuickFindEffect, QuickFindFailure, QuickFindFailureRecovery,
    QuickFindNavigation, QuickFindPresentationEffect, QuickFindQueryCompletion, QuickFindQueryJob,
    QuickFindRequest, QuickFindTimer,
};
use crate::ui::surface::PageMutationCoordinator;

use super::{NotionSearchRequestToken, NotionSearchResultsState, NotionSearchState};

const QUERY_PAGE_SIZE: u32 = 20;

enum QueryDispatch {
    Debounced,
    Immediate,
}

impl NotionSearchState {
    pub(crate) fn reduce_quick_find_action(
        &mut self,
        action: QuickFindAction,
        input: QuickFindActionInput,
    ) -> Vec<QuickFindEffect> {
        match action {
            QuickFindAction::Close => vec![close_effect()],
            QuickFindAction::QueryChanged(query) => self.change_query(query),
            QuickFindAction::Submit { open_in_new_tab } => {
                self.open_selected(open_in_new_tab, input.visited_at_unix_millis)
            }
            QuickFindAction::MoveSelection(delta) => self.move_selection(delta),
            QuickFindAction::SelectResult(index) => self.select_result(index),
            QuickFindAction::SelectAndOpen(index) => {
                self.set_selected_index(index);
                self.open_selected(false, input.visited_at_unix_millis)
            }
            QuickFindAction::LoadMore => self.load_more(),
            QuickFindAction::ToggleTitleOnly => self.toggle_title_only(),
            QuickFindAction::CopySelectedLink => self.copy_selected_link(),
            QuickFindAction::OpenSelectedInNewTab => {
                self.open_selected(true, input.visited_at_unix_millis)
            }
        }
    }

    pub(crate) fn refresh_quick_find_after_page_mutation(
        &mut self,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        if !search_open {
            return Vec::new();
        }
        self.restart_query(QueryDispatch::Immediate, false)
    }

    pub(crate) fn finish_quick_find_timer(
        &mut self,
        timer: QuickFindTimer,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        match timer {
            QuickFindTimer::Query {
                request_token,
                query,
            } if search_open && self.request_is_current(request_token) && self.query == query => {
                vec![query_request(request_token)]
            }
            QuickFindTimer::Query { .. } => Vec::new(),
            timer => self.finish_quick_find_preview_timer(timer, search_open),
        }
    }

    pub(crate) fn begin_quick_find_query_job(
        &mut self,
        request_token: NotionSearchRequestToken,
        current_board_url: String,
    ) -> Option<QuickFindQueryJob> {
        if !self.request_is_current(request_token) {
            return None;
        }
        self.mark_query_request_dispatched(request_token);
        self.flow_number = self.flow_number.wrapping_add(1);
        Some(QuickFindQueryJob {
            request_token,
            request: SearchWorkspaceRequest {
                current_board_url,
                query: self.query.trim().to_string(),
                scope: self.scope,
                limit: self.result_limit,
                search_session_id: self.session_id.clone(),
                flow_number: self.flow_number,
                recent_pages_for_boosting: self.recent_pages_for_boosting(),
                excluded_block_ids: self.local_result_ids_for_query(request_token),
            },
        })
    }

    pub(crate) fn recover_quick_find_failure(
        &mut self,
        recovery: QuickFindFailureRecovery,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        match recovery {
            QuickFindFailureRecovery::Query(token) if self.request_is_current(token) => {
                self.finish_query_failure(token);
                let mut effects = self.preview_after_settle();
                effects.push(QuickFindEffect::Notify);
                effects
            }
            QuickFindFailureRecovery::Preview { block_id } => {
                self.fail_quick_find_preview(block_id, search_open)
            }
            QuickFindFailureRecovery::Recents(token) => {
                self.fail_quick_find_recents(token, search_open)
            }
            QuickFindFailureRecovery::Query(_) | QuickFindFailureRecovery::None => Vec::new(),
        }
    }

    fn change_query(&mut self, query: String) -> Vec<QuickFindEffect> {
        if self.query == query {
            return Vec::new();
        }
        self.query = query;
        self.restart_query(QueryDispatch::Debounced, true)
    }

    fn restart_query(
        &mut self,
        dispatch: QueryDispatch,
        reset_limit: bool,
    ) -> Vec<QuickFindEffect> {
        let request_token = self.begin_request();
        self.reset_selected_index();
        if reset_limit {
            self.result_limit = QUERY_PAGE_SIZE;
        }
        if self.query.trim().is_empty() {
            return vec![QuickFindEffect::Request(QuickFindRequest::RefreshRecents)];
        }
        let mut effects = self.seed_query_preview();
        effects.push(match dispatch {
            QueryDispatch::Debounced => QuickFindEffect::Schedule(QuickFindTimer::Query {
                request_token,
                query: self.query.clone(),
            }),
            QueryDispatch::Immediate => query_request(request_token),
        });
        effects.push(QuickFindEffect::Notify);
        effects
    }

    fn seed_query_preview(&mut self) -> Vec<QuickFindEffect> {
        if self.show_query_seed() {
            return self.preview_after_settle();
        }
        self.results = NotionSearchResultsState::Loading;
        self.show_loading_rows(6);
        Vec::new()
    }

    fn move_selection(&mut self, delta: isize) -> Vec<QuickFindEffect> {
        let result_count = self.visible_results().len();
        if result_count == 0 {
            return Vec::new();
        }
        let last = result_count.saturating_sub(1) as isize;
        self.set_selected_index((self.selected_index as isize + delta).clamp(0, last) as usize);
        if let Some(row) = self.list_row_index_for_result(self.selected_index) {
            self.list_state.scroll_to_reveal_item(row);
        }
        let mut effects = self.preview_after_settle();
        if self.should_prefetch_for_selection(result_count) {
            if let Some(token) = self.begin_pagination_request(QUERY_PAGE_SIZE) {
                effects.push(query_request(token));
            }
        }
        effects.push(QuickFindEffect::Notify);
        effects
    }

    fn select_result(&mut self, index: usize) -> Vec<QuickFindEffect> {
        if index >= self.visible_results().len() {
            return Vec::new();
        }
        self.set_selected_index(index);
        let mut effects = self.preview_after_settle();
        effects.push(QuickFindEffect::Notify);
        effects
    }

    fn should_prefetch_for_selection(&self, result_count: usize) -> bool {
        matches!(
            &self.results,
            NotionSearchResultsState::Loaded { has_more: true, .. }
        ) && !self.pagination_is_loading()
            && quick_find_should_prefetch(self.selected_index, result_count)
    }

    fn load_more(&mut self) -> Vec<QuickFindEffect> {
        let NotionSearchResultsState::Loaded { has_more: true, .. } = &self.results else {
            return Vec::new();
        };
        let Some(token) = self.begin_pagination_request(QUERY_PAGE_SIZE) else {
            return Vec::new();
        };
        vec![query_request(token), QuickFindEffect::Notify]
    }

    fn toggle_title_only(&mut self) -> Vec<QuickFindEffect> {
        self.scope = match self.scope {
            SearchWorkspaceScope::AllContent => SearchWorkspaceScope::TitleOnly,
            SearchWorkspaceScope::TitleOnly => SearchWorkspaceScope::AllContent,
        };
        self.restart_query(QueryDispatch::Immediate, true)
    }

    fn copy_selected_link(&self) -> Vec<QuickFindEffect> {
        self.selected_result_link().map_or_else(Vec::new, |link| {
            vec![QuickFindEffect::Presentation(
                QuickFindPresentationEffect::CopyLink(link),
            )]
        })
    }

    pub(crate) fn selected_result_link(&self) -> Option<String> {
        self.visible_results()
            .get(self.selected_index)
            .map(|result| result.target_board_url.clone())
    }

    fn open_selected(&mut self, new_tab: bool, visited_at: u64) -> Vec<QuickFindEffect> {
        let Some(result) = self.visible_results().get(self.selected_index).cloned() else {
            return Vec::new();
        };
        let recent_page = recent_page_for_visit(result.clone(), visited_at);
        self.record_recent_page(recent_page.clone());
        let persist = QuickFindEffect::Request(QuickFindRequest::PersistVisit(Box::new(
            crate::ui::search::action::QuickFindVisitJob { recent_page },
        )));
        let close = close_effect();
        let navigate =
            QuickFindEffect::Presentation(QuickFindPresentationEffect::Navigate(if new_tab {
                QuickFindNavigation::NewTab(result.target_board_url)
            } else {
                QuickFindNavigation::Workspace {
                    board_url: result.target_board_url,
                    title: result.title,
                }
            }));
        if new_tab {
            vec![persist, navigate, close]
        } else {
            vec![persist, close, navigate]
        }
    }

    fn complete_query(
        &mut self,
        completion: QuickFindQueryCompletion,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        let token = completion.request_token;
        if !search_open || !self.request_is_current(token) {
            return Vec::new();
        }
        let mut result = match completion.result {
            Ok(result) => result,
            Err(error) => return vec![query_failure(token, error)],
        };
        result
            .results
            .retain(|result| !self.page_mutation_is_dirty(&result.block_id));
        if !self.commit_query_results(
            token,
            result.total,
            result.consumed_result_count,
            result.results.clone(),
        ) {
            return Vec::new();
        }
        let mut effects = completion.request.map_or_else(Vec::new, |request| {
            vec![QuickFindEffect::RecordAcceptedQuery(Box::new(
                QuickFindAcceptedQuery { request, result },
            ))]
        });
        effects.extend(self.preview_after_settle());
        effects.push(QuickFindEffect::Notify);
        effects
    }
}

impl QuickFindCompletion {
    pub(crate) fn finish(
        self,
        state: &mut NotionSearchState,
        page_mutations: &mut PageMutationCoordinator,
        search_open: bool,
    ) -> Vec<QuickFindEffect> {
        match self {
            Self::Query(completion) => state.complete_query(*completion, search_open),
            Self::Preview(completion) => {
                state.complete_quick_find_preview(*completion, page_mutations, search_open)
            }
            Self::Recents(completion) => state.complete_recents(*completion, search_open),
            Self::Visit(completion) => match completion.result {
                Ok(()) => Vec::new(),
                Err(error) => vec![QuickFindEffect::Failure(Box::new(QuickFindFailure {
                    operation: "Quick Find recent-page cache update failed",
                    error,
                    recovery: QuickFindFailureRecovery::None,
                }))],
            },
        }
    }
}

fn close_effect() -> QuickFindEffect {
    QuickFindEffect::Presentation(QuickFindPresentationEffect::Close)
}

pub(super) fn query_request(token: NotionSearchRequestToken) -> QuickFindEffect {
    QuickFindEffect::Request(QuickFindRequest::Query(token))
}

fn recent_page_for_visit(
    mut page: crate::model::PageShellSearchResult,
    visited_at_unix_millis: u64,
) -> RecentPageResult {
    page.match_snippet = None;
    page.editor_display_name = None;
    page.edited_label = None;
    page.badges
        .retain(|badge| *badge == PageShellSearchBadge::Database);
    RecentPageResult {
        page,
        visited_at_unix_millis,
    }
}

fn query_failure(
    token: NotionSearchRequestToken,
    error: crate::model::NotionWorkspaceOperationFailure,
) -> QuickFindEffect {
    QuickFindEffect::Failure(Box::new(QuickFindFailure {
        operation: "search failed",
        error,
        recovery: QuickFindFailureRecovery::Query(token),
    }))
}
