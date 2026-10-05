use std::time::Duration;

use crate::model::{
    CardPage, LoadRecentPagesRequest, LoadRecentPagesResult, NotionWorkspaceOperationFailure,
    NotionWorkspaceResult, QuickFindLocalSearchCache, RecentPageResult, SearchWorkspaceRequest,
    SearchWorkspaceResult,
};
use crate::ui::surface::PageMutationRunToken;
use crate::ui::NotionSearchRequestToken;

const QUERY_DEBOUNCE: Duration = Duration::from_millis(225);
const PREVIEW_SETTLE: Duration = Duration::from_millis(75);
const KEYBOARD_PAGINATION_PREFETCH_RESULTS: usize = 3;

/// User intent emitted by the Quick Find view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QuickFindAction {
    Close,
    QueryChanged(String),
    Submit { open_in_new_tab: bool },
    MoveSelection(isize),
    SelectResult(usize),
    SelectAndOpen(usize),
    LoadMore,
    ToggleTitleOnly,
    CopySelectedLink,
    OpenSelectedInNewTab,
}

pub(crate) struct QuickFindActionInput {
    pub(crate) visited_at_unix_millis: u64,
}

pub(crate) enum QuickFindEffect {
    Presentation(QuickFindPresentationEffect),
    Request(QuickFindRequest),
    Schedule(QuickFindTimer),
    RecordAcceptedQuery(Box<QuickFindAcceptedQuery>),
    Failure(Box<QuickFindFailure>),
    Notify,
}

pub(crate) enum QuickFindPresentationEffect {
    Close,
    FocusInput,
    Navigate(QuickFindNavigation),
    CopyLink(String),
}

pub(crate) enum QuickFindNavigation {
    Workspace { board_url: String, title: String },
    NewTab(String),
}

pub(crate) enum QuickFindRequest {
    RefreshRecents,
    Query(NotionSearchRequestToken),
    Preview(QuickFindPreviewRequest),
    Recents(Box<QuickFindRecentsJob>),
    PersistVisit(Box<QuickFindVisitJob>),
}

pub(crate) struct QuickFindPreviewRequest {
    pub(crate) block_id: String,
    pub(crate) show_loading: bool,
}

pub(crate) enum QuickFindTimer {
    Query {
        request_token: NotionSearchRequestToken,
        query: String,
    },
    PreviewSelection {
        block_id: String,
    },
    PreviewRevalidation {
        block_id: String,
    },
}

impl QuickFindTimer {
    pub(crate) const fn delay(&self) -> Duration {
        match self {
            Self::Query { .. } => QUERY_DEBOUNCE,
            Self::PreviewSelection { .. } | Self::PreviewRevalidation { .. } => PREVIEW_SETTLE,
        }
    }
}

pub(crate) const fn quick_find_should_prefetch(
    selected_result_index: usize,
    result_count: usize,
) -> bool {
    result_count > 0
        && result_count.saturating_sub(selected_result_index.saturating_add(1))
            <= KEYBOARD_PAGINATION_PREFETCH_RESULTS
}

pub(crate) struct QuickFindQueryJob {
    pub(crate) request_token: NotionSearchRequestToken,
    pub(crate) request: SearchWorkspaceRequest,
}

pub(crate) struct QuickFindQueryCompletion {
    pub(crate) request_token: NotionSearchRequestToken,
    pub(crate) request: Option<SearchWorkspaceRequest>,
    pub(crate) result: NotionWorkspaceResult<SearchWorkspaceResult>,
}

pub(crate) struct QuickFindAcceptedQuery {
    pub(crate) request: SearchWorkspaceRequest,
    pub(crate) result: SearchWorkspaceResult,
}

pub(crate) struct QuickFindPreviewJob {
    pub(crate) block_id: String,
    pub(crate) request_token: NotionSearchRequestToken,
    pub(crate) authority: Option<PageMutationRunToken>,
    pub(crate) show_loading: bool,
}

pub(crate) struct QuickFindPreviewCompletion {
    pub(crate) job: QuickFindPreviewJob,
    pub(crate) result: NotionWorkspaceResult<CardPage>,
}

pub(crate) struct QuickFindRecentsSource {
    pub(crate) cache_scope: Option<String>,
    pub(crate) current_page_id: Option<String>,
    pub(crate) current_board_url: String,
    pub(crate) persisted_results: Option<Vec<RecentPageResult>>,
    pub(crate) local_search: QuickFindLocalSearchCache,
}

pub(crate) struct QuickFindRecentsJob {
    pub(crate) request_token: NotionSearchRequestToken,
    pub(crate) request: LoadRecentPagesRequest,
}

pub(crate) struct QuickFindRecentsCompletion {
    pub(crate) request_token: NotionSearchRequestToken,
    pub(crate) result: NotionWorkspaceResult<LoadRecentPagesResult>,
}

pub(crate) struct QuickFindVisitJob {
    pub(crate) recent_page: RecentPageResult,
}

pub(crate) struct QuickFindVisitCompletion {
    pub(crate) result: NotionWorkspaceResult<()>,
}

pub(crate) enum QuickFindCompletion {
    Query(Box<QuickFindQueryCompletion>),
    Preview(Box<QuickFindPreviewCompletion>),
    Recents(Box<QuickFindRecentsCompletion>),
    Visit(Box<QuickFindVisitCompletion>),
}

impl QuickFindCompletion {
    pub(crate) fn query(
        job: QuickFindQueryJob,
        result: NotionWorkspaceResult<SearchWorkspaceResult>,
    ) -> Self {
        Self::Query(Box::new(QuickFindQueryCompletion {
            request_token: job.request_token,
            request: Some(job.request),
            result,
        }))
    }

    #[cfg(test)]
    pub(crate) fn query_for_test(
        request_token: NotionSearchRequestToken,
        result: NotionWorkspaceResult<SearchWorkspaceResult>,
    ) -> Self {
        Self::Query(Box::new(QuickFindQueryCompletion {
            request_token,
            request: None,
            result,
        }))
    }

    pub(crate) fn preview(
        job: QuickFindPreviewJob,
        result: NotionWorkspaceResult<CardPage>,
    ) -> Self {
        Self::Preview(Box::new(QuickFindPreviewCompletion { job, result }))
    }

    pub(crate) fn recents(
        request_token: NotionSearchRequestToken,
        result: NotionWorkspaceResult<LoadRecentPagesResult>,
    ) -> Self {
        Self::Recents(Box::new(QuickFindRecentsCompletion {
            request_token,
            result,
        }))
    }

    pub(crate) fn visit(result: NotionWorkspaceResult<()>) -> Self {
        Self::Visit(Box::new(QuickFindVisitCompletion { result }))
    }
}

pub(crate) struct QuickFindFailure {
    pub(crate) operation: &'static str,
    pub(crate) error: NotionWorkspaceOperationFailure,
    pub(crate) recovery: QuickFindFailureRecovery,
}

pub(crate) enum QuickFindFailureRecovery {
    Query(NotionSearchRequestToken),
    Preview { block_id: String },
    Recents(NotionSearchRequestToken),
    None,
}
