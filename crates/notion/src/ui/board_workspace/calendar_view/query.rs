use super::{Context, SurfaceState};

mod actions;
mod completion;
mod controller;
mod jobs;
mod state;
mod types;

pub(super) use actions::handle_calendar_action;
pub(in crate::ui::board_workspace) use types::{
    CalendarDateAssignmentJob, DateUndatedCountJob, DateUndatedCountRetry,
    DateUndatedDialogCommand, DateUndatedQueryJob, DateUndatedQueryToken, DateViewContext,
    DateViewEffect, DateViewRequest,
};
use types::{DateViewCompletion, DateViewCompletionOutcome};

impl SurfaceState {
    pub(crate) fn ensure_date_view_queries(&mut self, cx: &mut Context<Self>) {
        let workspace_available = self.notion_startup.workspace_api().is_some();
        let context = DateViewContext::from_board(&self.board, workspace_available);
        let effects = self
            .date_view
            .ensure_query_effects(&context, &self.board.items);
        self.execute_date_view_effects(effects, cx);
    }

    pub(crate) fn schedule_date_view_queries(&self, cx: &mut Context<Self>) {
        if !self
            .date_view
            .begin_query_schedule(self.board.active_date_view().is_some())
        {
            return;
        }
        let surface = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = surface.update(cx, |surface, cx| {
                surface.date_view.finish_query_schedule();
                surface.ensure_date_view_queries(cx);
            });
        });
    }

    pub(in crate::ui::board_workspace) fn finish_date_view_completion(
        &mut self,
        completion: DateViewCompletion,
        cx: &mut Context<Self>,
    ) {
        let workspace_available = self.notion_startup.workspace_api().is_some();
        let context = DateViewContext::from_board(&self.board, workspace_available);
        let outcome = self
            .date_view
            .finish_completion(completion, &context, &self.board.items);
        let effects = match outcome {
            DateViewCompletionOutcome::Stale => return,
            DateViewCompletionOutcome::Effects(effects) => effects,
            DateViewCompletionOutcome::Failure(failure) => {
                if self.handle_notion_workspace_failure(failure.operation, failure.error, cx) {
                    return;
                }
                self.date_view
                    .recover_failure(failure.recovery, &context, &self.board.items)
            }
        };
        self.execute_date_view_effects(effects, cx);
    }
}
