use super::types::{
    CalendarDateAssignmentJob, CalendarMonthLoadCompletion, CalendarPageCreation,
    CalendarRangeResizeJob, DateUndatedCountJob, DateUndatedCountRetry, DateUndatedQueryToken,
    DateViewCompletion, DateViewCompletionOutcome, DateViewContext, DateViewEffect,
    DateViewFailure, DateViewFailureRecovery,
};
use crate::model::{BoardItem, LoadCalendarItemsResult, NotionWorkspaceResult};
use crate::ui::surface::NotionDateViewState;
use crate::ui::Arc;

impl NotionDateViewState {
    pub(in crate::ui::board_workspace) fn finish_completion(
        &mut self,
        completion: DateViewCompletion,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> DateViewCompletionOutcome {
        match completion {
            DateViewCompletion::CalendarMonth { token, result } => {
                match self.complete_month_load(&token, context.active_identity(), result) {
                    CalendarMonthLoadCompletion::Stale => DateViewCompletionOutcome::Stale,
                    CalendarMonthLoadCompletion::Loaded => outcome(vec![DateViewEffect::Notify]),
                    CalendarMonthLoadCompletion::Failed(error) => failure(
                        "Calendar query failed",
                        error,
                        DateViewFailureRecovery::RollBackMonth(token.month),
                    ),
                }
            }
            DateViewCompletion::CalendarPageCreation { creation, result } => {
                self.finish_creation(creation, result, context, board_items)
            }
            DateViewCompletion::CalendarRangeResize { job, result } => {
                self.finish_range_resize(job, result, context, board_items)
            }
            DateViewCompletion::CalendarDateAssignment { job, result } => {
                self.finish_assignment(job, result, context, board_items)
            }
            DateViewCompletion::UndatedItems { token, result } => {
                self.finish_undated_query(token, result, context)
            }
            DateViewCompletion::UndatedCount { job, result } => {
                self.finish_undated_count(job, result, context)
            }
            DateViewCompletion::UndatedCountRetry(retry) => {
                self.finish_undated_count_retry(retry, context)
            }
        }
    }

    pub(in crate::ui::board_workspace) fn recover_failure(
        &mut self,
        recovery: DateViewFailureRecovery,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> Vec<DateViewEffect> {
        match recovery {
            DateViewFailureRecovery::None => Vec::new(),
            DateViewFailureRecovery::RollBackMonth(month) => {
                self.roll_back_failed_month_load(month);
                vec![DateViewEffect::Notify]
            }
            DateViewFailureRecovery::Notify => vec![DateViewEffect::Notify],
            DateViewFailureRecovery::RefreshQueries => {
                let mut effects = self.refresh_query_effects(context, board_items);
                effects.push(DateViewEffect::Notify);
                effects
            }
            DateViewFailureRecovery::RetryUndatedCount(job) => {
                let mut effects = self
                    .undated
                    .prepare_count_retry_effect(job)
                    .into_iter()
                    .collect::<Vec<_>>();
                effects.push(DateViewEffect::Notify);
                effects
            }
        }
    }

    fn finish_creation(
        &mut self,
        creation: CalendarPageCreation,
        result: NotionWorkspaceResult<String>,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> DateViewCompletionOutcome {
        let refresh = self.complete_page_creation(&creation, context.active_identity());
        let block_id = match result {
            Ok(block_id) => block_id,
            Err(error) => {
                return failure(
                    "Calendar page creation failed",
                    error,
                    DateViewFailureRecovery::None,
                );
            }
        };
        let mut generated = if refresh {
            self.refresh_query_effects(context, board_items)
        } else {
            Vec::new()
        };
        generated.push(DateViewEffect::OpenCard(block_id));
        outcome(generated)
    }

    fn finish_range_resize(
        &mut self,
        job: CalendarRangeResizeJob,
        result: NotionWorkspaceResult<()>,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> DateViewCompletionOutcome {
        if !self.complete_range_resize(&job.block_id, &job.session) {
            return DateViewCompletionOutcome::Stale;
        }
        if let Err(error) = result {
            return failure(
                "Calendar date resize failed",
                error,
                DateViewFailureRecovery::RefreshQueries,
            );
        }
        outcome(self.refresh_after_mutation(context, board_items))
    }

    fn finish_assignment(
        &mut self,
        job: CalendarDateAssignmentJob,
        result: NotionWorkspaceResult<()>,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> DateViewCompletionOutcome {
        if !Arc::ptr_eq(&self.query_session, &job.session) {
            return DateViewCompletionOutcome::Stale;
        }
        self.undated.complete_assignment(&job.block_id);
        if let Err(error) = result {
            return failure(
                "Calendar date assignment failed",
                error,
                DateViewFailureRecovery::Notify,
            );
        }
        outcome(self.refresh_after_mutation(context, board_items))
    }

    fn refresh_after_mutation(
        &mut self,
        context: &DateViewContext,
        board_items: &[BoardItem],
    ) -> Vec<DateViewEffect> {
        let mut effects = self.refresh_query_effects(context, board_items);
        effects.push(DateViewEffect::Notify);
        effects
    }

    fn finish_undated_query(
        &mut self,
        token: DateUndatedQueryToken,
        result: NotionWorkspaceResult<LoadCalendarItemsResult>,
        context: &DateViewContext,
    ) -> DateViewCompletionOutcome {
        if !self
            .undated
            .query_is_current(&token, &self.query_session, context.active_identity())
        {
            return DateViewCompletionOutcome::Stale;
        }
        match result {
            Ok(result) => {
                self.undated.complete_query(result);
                outcome(vec![DateViewEffect::Notify])
            }
            Err(error) => {
                self.undated.fail_query();
                failure(
                    "no-date item query failed",
                    error,
                    DateViewFailureRecovery::Notify,
                )
            }
        }
    }

    fn finish_undated_count(
        &mut self,
        job: DateUndatedCountJob,
        result: NotionWorkspaceResult<usize>,
        context: &DateViewContext,
    ) -> DateViewCompletionOutcome {
        if !self.undated.count_request_is_current(
            &job,
            &self.query_session,
            context.active_identity(),
        ) {
            return DateViewCompletionOutcome::Stale;
        }
        match result {
            Ok(count) => {
                self.undated.complete_count();
                outcome(vec![
                    DateViewEffect::SetUndatedCount(count),
                    DateViewEffect::Notify,
                ])
            }
            Err(error) => failure(
                "no-date count query failed",
                error,
                DateViewFailureRecovery::RetryUndatedCount(job),
            ),
        }
    }

    fn finish_undated_count_retry(
        &mut self,
        retry: DateUndatedCountRetry,
        context: &DateViewContext,
    ) -> DateViewCompletionOutcome {
        if !self.undated.count_retry_is_current(
            &retry,
            &self.query_session,
            context.active_identity(),
        ) {
            return DateViewCompletionOutcome::Stale;
        }
        self.undated.begin_count_retry();
        let generated = self
            .undated
            .prepare_count_effect(
                retry.view_identity,
                retry.session,
                context.workspace_available,
            )
            .into_iter()
            .collect();
        outcome(generated)
    }
}

fn outcome(effects: Vec<DateViewEffect>) -> DateViewCompletionOutcome {
    DateViewCompletionOutcome::Effects(effects)
}

fn failure(
    operation: &'static str,
    error: crate::model::NotionWorkspaceOperationFailure,
    recovery: DateViewFailureRecovery,
) -> DateViewCompletionOutcome {
    DateViewCompletionOutcome::Failure(Box::new(DateViewFailure {
        operation,
        error,
        recovery,
    }))
}
