use super::{
    Arc, CalendarDateAssignmentJob, DateUndatedAction, DateUndatedCountJob, DateUndatedCountRetry,
    DateUndatedEffect, DateUndatedItemsState, DateUndatedQueryJob, DateUndatedQueryToken,
    DateUndatedRow, DateViewContext, DateViewEffect, DateViewRequest, LoadCalendarItemsRequest,
    SetCalendarPageDateRequest, DATE_UNDATED_COUNT_MAX_RETRIES, DATE_UNDATED_MAX_LIMIT,
    DATE_UNDATED_PAGE_SIZE,
};
use crate::model::LoadCalendarItemsResult;

impl DateUndatedItemsState {
    pub(super) fn reduce(&mut self, action: DateUndatedAction) -> DateUndatedEffect {
        match action {
            DateUndatedAction::Dismiss => DateUndatedEffect::Close,
            DateUndatedAction::SearchChanged(query) => {
                if self.set_search_query(query) {
                    DateUndatedEffect::BeginQuery {
                        limit: DATE_UNDATED_PAGE_SIZE,
                        preserve_rows: false,
                    }
                } else {
                    DateUndatedEffect::None
                }
            }
            DateUndatedAction::ArmPagination => {
                if self.arm_pagination() {
                    DateUndatedEffect::Notify
                } else {
                    DateUndatedEffect::None
                }
            }
            DateUndatedAction::LoadMore => {
                self.next_page_limit()
                    .map_or(DateUndatedEffect::None, |limit| {
                        DateUndatedEffect::BeginQuery {
                            limit,
                            preserve_rows: true,
                        }
                    })
            }
            DateUndatedAction::Assign(item) => DateUndatedEffect::BeginAssignment(item),
            DateUndatedAction::Open(item) => DateUndatedEffect::CloseAndOpen(item),
            DateUndatedAction::DragStarted => DateUndatedEffect::NotifyDragHosts,
        }
    }

    pub(in crate::ui::board_workspace) fn reset_for_view(&mut self, view_identity: Option<String>) {
        self.count_generation = self
            .count_generation
            .checked_add(1)
            .expect("Notion no-date count request generation exhausted");
        self.query_generation = self
            .query_generation
            .checked_add(1)
            .expect("Notion no-date query generation exhausted");
        self.view_identity = view_identity;
        self.count_initialized = false;
        self.count_retry_attempts = 0;
        self.count_retry_scheduled = false;
        self.open = false;
        self.reset_open();
        self.input.borrow_mut().take();
    }

    pub(in crate::ui::board_workspace) fn open(&mut self) {
        self.open = true;
        self.reset_open();
    }

    pub(in crate::ui::board_workspace) fn close(&mut self) {
        self.open = false;
        self.query_generation = self
            .query_generation
            .checked_add(1)
            .expect("Notion no-date query generation exhausted");
        self.reset_open();
        self.input.borrow_mut().take();
    }

    fn reset_open(&mut self) {
        self.query.clear();
        self.limit = DATE_UNDATED_PAGE_SIZE;
        self.returned_block_count = 0;
        self.pending = false;
        self.pagination_armed = false;
        self.loaded = false;
        self.rows = Arc::default();
        self.scroll_handle = gpui::UniformListScrollHandle::new();
    }

    fn set_search_query(&mut self, query: String) -> bool {
        if !self.open || self.query == query {
            return false;
        }
        self.query = query;
        self.scroll_handle = gpui::UniformListScrollHandle::new();
        true
    }

    fn arm_pagination(&mut self) -> bool {
        if self.pagination_armed {
            return false;
        }
        self.pagination_armed = true;
        true
    }

    fn next_page_limit(&self) -> Option<usize> {
        if !self.open || self.pending || !self.pagination_armed || !self.can_load_more() {
            return None;
        }
        Some((self.limit + DATE_UNDATED_PAGE_SIZE).min(DATE_UNDATED_MAX_LIMIT))
    }

    pub(super) fn can_load_more(&self) -> bool {
        self.loaded
            && self.returned_block_count == self.limit
            && self.limit < DATE_UNDATED_MAX_LIMIT
    }

    pub(in crate::ui::board_workspace) fn prepare_query_effect(
        &mut self,
        context: &DateViewContext,
        session: Arc<()>,
        limit: usize,
        preserve_rows: bool,
    ) -> Option<DateViewEffect> {
        if !self.open {
            return None;
        }
        let view_identity = context.active_view_identity.clone()?;
        if !context.workspace_available {
            self.mark_query_unavailable();
            return Some(DateViewEffect::Notify);
        }
        let query = self.query.clone();
        let request = LoadCalendarItemsRequest::undated(limit, query.clone())
            .expect("Notion no-date query limits must remain valid");
        self.query_generation = self
            .query_generation
            .checked_add(1)
            .expect("Notion no-date query generation exhausted");
        self.limit = limit;
        self.pending = true;
        self.pagination_armed = false;
        if !preserve_rows {
            self.returned_block_count = 0;
            self.loaded = false;
            self.rows = Arc::default();
        }
        let token = DateUndatedQueryToken {
            view_identity,
            query,
            generation: self.query_generation,
            session,
            limit,
        };
        Some(DateViewEffect::Request(Box::new(
            DateViewRequest::UndatedItems(DateUndatedQueryJob { request, token }),
        )))
    }

    pub(super) fn mark_query_unavailable(&mut self) {
        self.pending = false;
        self.loaded = false;
    }

    pub(in crate::ui::board_workspace) fn query_is_current(
        &self,
        token: &DateUndatedQueryToken,
        session: &Arc<()>,
        active_view_identity: Option<&str>,
    ) -> bool {
        Arc::ptr_eq(session, &token.session)
            && self.open
            && self.query_generation == token.generation
            && self.limit == token.limit
            && self.query == token.query
            && self.view_identity.as_deref() == Some(token.view_identity.as_str())
            && active_view_identity == Some(token.view_identity.as_str())
    }

    pub(in crate::ui::board_workspace) fn complete_query(
        &mut self,
        result: LoadCalendarItemsResult,
    ) {
        self.pending = false;
        self.returned_block_count = result.returned_block_count;
        self.rows = result
            .items
            .into_iter()
            .map(|item| DateUndatedRow {
                element_id: format!("notion-date-undated-item-{}", item.block_id).into(),
                title: item.title.clone().into(),
                item,
            })
            .collect::<Vec<_>>()
            .into();
        self.loaded = true;
    }

    pub(in crate::ui::board_workspace) fn fail_query(&mut self) {
        self.pending = false;
        self.returned_block_count = 0;
    }

    pub(in crate::ui::board_workspace) fn prepare_assignment_effect(
        &mut self,
        request: SetCalendarPageDateRequest,
        context: &DateViewContext,
        session: Arc<()>,
    ) -> Option<DateViewEffect> {
        context.active_identity()?;
        if !context.workspace_available {
            return None;
        }
        let block_id = request.block_id().to_string();
        if !self.assignments_in_flight.insert(block_id.clone()) {
            return None;
        }
        Some(DateViewEffect::Request(Box::new(
            DateViewRequest::AssignCalendarDate(CalendarDateAssignmentJob {
                request,
                block_id,
                session,
            }),
        )))
    }

    pub(in crate::ui::board_workspace) fn complete_assignment(&mut self, block_id: &str) {
        self.assignments_in_flight.remove(block_id);
    }

    pub(in crate::ui::board_workspace) fn reset_count_request(&mut self) {
        self.count_generation = self
            .count_generation
            .checked_add(1)
            .expect("Notion no-date count request generation exhausted");
        self.count_initialized = false;
        self.count_retry_attempts = 0;
        self.count_retry_scheduled = false;
    }

    pub(in crate::ui::board_workspace) fn prepare_count_effect(
        &mut self,
        view_identity: String,
        session: Arc<()>,
        workspace_available: bool,
    ) -> Option<DateViewEffect> {
        if self.count_initialized || self.count_retry_scheduled {
            return None;
        }
        self.count_initialized = true;
        if !workspace_available {
            return None;
        }
        self.count_generation = self
            .count_generation
            .checked_add(1)
            .expect("Notion no-date count request generation exhausted");
        Some(DateViewEffect::Request(Box::new(
            DateViewRequest::UndatedCount(DateUndatedCountJob {
                view_identity,
                generation: self.count_generation,
                session,
            }),
        )))
    }

    pub(in crate::ui::board_workspace) fn count_request_is_current(
        &self,
        job: &DateUndatedCountJob,
        current_session: &Arc<()>,
        active_view_identity: Option<&str>,
    ) -> bool {
        Arc::ptr_eq(current_session, &job.session)
            && self.count_generation == job.generation
            && self.view_identity.as_deref() == Some(job.view_identity.as_str())
            && active_view_identity == Some(job.view_identity.as_str())
    }

    pub(in crate::ui::board_workspace) fn complete_count(&mut self) {
        self.count_retry_attempts = 0;
        self.count_retry_scheduled = false;
    }

    pub(in crate::ui::board_workspace) fn prepare_count_retry_effect(
        &mut self,
        failed: DateUndatedCountJob,
    ) -> Option<DateViewEffect> {
        if self.count_retry_attempts >= DATE_UNDATED_COUNT_MAX_RETRIES {
            return None;
        }
        self.count_retry_attempts += 1;
        self.count_retry_scheduled = true;
        Some(DateViewEffect::Request(Box::new(
            DateViewRequest::UndatedCountRetry(DateUndatedCountRetry {
                view_identity: failed.view_identity,
                failed_generation: failed.generation,
                retry_attempt: self.count_retry_attempts,
                session: failed.session,
            }),
        )))
    }

    pub(in crate::ui::board_workspace) fn count_retry_is_current(
        &self,
        retry: &DateUndatedCountRetry,
        current_session: &Arc<()>,
        active_view_identity: Option<&str>,
    ) -> bool {
        Arc::ptr_eq(current_session, &retry.session)
            && self.count_generation == retry.failed_generation
            && self.count_retry_attempts == retry.retry_attempt
            && self.count_retry_scheduled
            && self.view_identity.as_deref() == Some(retry.view_identity.as_str())
            && active_view_identity == Some(retry.view_identity.as_str())
    }

    pub(in crate::ui::board_workspace) fn begin_count_retry(&mut self) {
        self.count_retry_scheduled = false;
        self.count_initialized = false;
    }
}

impl crate::ui::surface::NotionDateViewState {
    pub(super) fn resolve_undated_effect(
        &mut self,
        effect: DateUndatedEffect,
        context: &DateViewContext,
    ) -> Vec<DateViewEffect> {
        match effect {
            DateUndatedEffect::None => Vec::new(),
            DateUndatedEffect::Notify => vec![DateViewEffect::Notify],
            DateUndatedEffect::BeginQuery {
                limit,
                preserve_rows,
            } => self
                .undated
                .prepare_query_effect(context, self.query_session.clone(), limit, preserve_rows)
                .into_iter()
                .collect(),
            DateUndatedEffect::BeginAssignment(item) => {
                let target = self.default_assignment_date(context.timeline_is_active());
                let request = super::query::date_undated_click_assignment_request(&item, target);
                self.assignment_effects(request, context)
            }
            DateUndatedEffect::Close => vec![DateViewEffect::CloseUndatedDialogFromSource],
            DateUndatedEffect::CloseAndOpen(item) => vec![
                DateViewEffect::CloseUndatedDialogFromSource,
                DateViewEffect::OpenBoardItem(Box::new(item)),
            ],
            DateUndatedEffect::NotifyDragHosts => vec![DateViewEffect::NotifyDateDragHosts],
        }
    }
}
