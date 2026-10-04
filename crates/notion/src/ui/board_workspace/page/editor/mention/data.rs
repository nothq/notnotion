use crate::model::{
    NotionCommentWorkspaceUsers, NotionWorkspaceOperationFailure, NotionWorkspaceResult,
    PageShellSearchResult,
};

use super::state::PageMentionController;

pub(super) const MENTION_PAGE_RESULT_LIMIT: u32 = 10;
pub(super) const MENTION_SEARCH_DEBOUNCE: std::time::Duration =
    std::time::Duration::from_millis(225);

pub(super) enum PageMentionPageSchedule {
    Recents { generation: u64 },
    Search { generation: u64, query: String },
}

pub(super) enum PageMentionCompletion {
    People(NotionWorkspaceResult<NotionCommentWorkspaceUsers>),
    Pages {
        generation: u64,
        query: String,
        result: NotionWorkspaceResult<Vec<PageShellSearchResult>>,
    },
}

pub(super) enum PageMentionCompletionResolution {
    Stale,
    Notify,
    Failure(Box<PageMentionCompletionFailure>),
}

pub(super) struct PageMentionCompletionFailure {
    pub(super) operation: &'static str,
    pub(super) error: NotionWorkspaceOperationFailure,
    pub(super) recovery: Option<PageMentionFailureRecovery>,
}

pub(super) enum PageMentionFailureRecovery {
    Pages { query: String },
}

pub(super) struct PageMentionSearchSequence {
    pub(super) search_session_id: String,
    pub(super) flow_number: u32,
}

impl PageMentionController {
    pub(super) fn can_begin_people_load(&self) -> bool {
        self.people.users.is_none() && !self.people.loading
    }

    pub(super) fn begin_people_load(&mut self) -> bool {
        if !self.can_begin_people_load() {
            return false;
        }
        self.people.loading = true;
        true
    }

    pub(super) fn complete_people_load(&mut self, users: NotionCommentWorkspaceUsers) {
        self.people.loading = false;
        self.people.users = Some(users.users.into());
    }

    pub(super) fn fail_people_load(&mut self) {
        self.people.loading = false;
    }

    pub(super) fn refresh_pages(&mut self, query: &str) -> Option<PageMentionPageSchedule> {
        let query = query.trim().to_string();
        if self.pages.query == query
            && (self.pages.loading || self.pages.loaded_query.as_deref() == Some(&query))
        {
            return None;
        }
        self.pages.query.clone_from(&query);
        self.pages.generation = self.pages.generation.wrapping_add(1);
        let generation = self.pages.generation;
        if query.is_empty() {
            Some(PageMentionPageSchedule::Recents { generation })
        } else {
            Some(PageMentionPageSchedule::Search { generation, query })
        }
    }

    pub(super) fn page_search_timer_is_current(&self, generation: u64) -> bool {
        self.menu.is_some() && self.pages.generation == generation
    }

    pub(super) fn begin_recent_pages_load(&mut self, generation: u64) -> bool {
        if self.pages.generation != generation {
            return false;
        }
        self.pages.loading = true;
        true
    }

    pub(super) fn begin_page_search(
        &mut self,
        generation: u64,
    ) -> Option<PageMentionSearchSequence> {
        if self.pages.generation != generation {
            return None;
        }
        self.pages.loading = true;
        self.pages.flow_number = self.pages.flow_number.wrapping_add(1);
        Some(PageMentionSearchSequence {
            search_session_id: self.pages.search_session_id.clone(),
            flow_number: self.pages.flow_number,
        })
    }

    pub(super) fn begin_pages_completion(&mut self, generation: u64) -> bool {
        if self.pages.generation != generation {
            return false;
        }
        self.pages.loading = false;
        true
    }

    pub(super) fn complete_pages_load(
        &mut self,
        query: String,
        results: Vec<PageShellSearchResult>,
    ) {
        self.pages.results = results.into();
        self.pages.loaded_query = Some(query);
    }

    pub(super) fn fail_pages_load(&mut self, query: String) {
        self.pages.results = Vec::new().into();
        self.pages.loaded_query = Some(query);
    }

    pub(super) fn resolve_completion(
        &mut self,
        completion: PageMentionCompletion,
    ) -> PageMentionCompletionResolution {
        match completion {
            PageMentionCompletion::People(result) => match result {
                Ok(users) => {
                    self.complete_people_load(users);
                    PageMentionCompletionResolution::Notify
                }
                Err(error) => {
                    self.fail_people_load();
                    PageMentionCompletionResolution::Failure(Box::new(
                        PageMentionCompletionFailure {
                            operation: "mention people failed to load",
                            error,
                            recovery: None,
                        },
                    ))
                }
            },
            PageMentionCompletion::Pages {
                generation,
                query,
                result,
            } => {
                if !self.begin_pages_completion(generation) {
                    return PageMentionCompletionResolution::Stale;
                }
                match result {
                    Ok(results) => {
                        self.complete_pages_load(query, results);
                        PageMentionCompletionResolution::Notify
                    }
                    Err(error) => PageMentionCompletionResolution::Failure(Box::new(
                        PageMentionCompletionFailure {
                            operation: "mention pages failed to load",
                            error,
                            recovery: Some(PageMentionFailureRecovery::Pages { query }),
                        },
                    )),
                }
            }
        }
    }

    pub(super) fn recover_completion_failure(&mut self, recovery: PageMentionFailureRecovery) {
        match recovery {
            PageMentionFailureRecovery::Pages { query } => self.fail_pages_load(query),
        }
    }
}
