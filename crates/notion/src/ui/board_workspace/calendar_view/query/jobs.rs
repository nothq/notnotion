use std::sync::Arc;

use gpui::Context;
use gpui_components::{spawn_background_task_for_entity, spawn_timer_task_for_entity};

use super::types::{
    CalendarDateAssignmentJob, CalendarMonthLoad, CalendarPageCreation, CalendarRangeResizeJob,
    DateUndatedCountJob, DateUndatedCountRetry, DateUndatedQueryJob, DateViewCompletion,
};
use crate::model::NotionWorkspaceApi;
use crate::ui::SurfaceState;

impl CalendarMonthLoad {
    pub(in crate::ui::board_workspace) fn spawn(
        self,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<SurfaceState>,
    ) {
        let Self { request, token } = self;
        spawn_background_task_for_entity(
            request,
            cx,
            move |request| workspace_api.load_calendar_items(request),
            move |surface, result, cx| {
                surface.finish_date_view_completion(
                    DateViewCompletion::CalendarMonth { token, result },
                    cx,
                );
            },
        );
    }
}

impl CalendarPageCreation {
    pub(in crate::ui::board_workspace) fn spawn(
        self,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<SurfaceState>,
    ) {
        let request = self.request.clone();
        spawn_background_task_for_entity(
            request,
            cx,
            move |request| workspace_api.create_calendar_page(request),
            move |surface, result, cx| {
                surface.finish_date_view_completion(
                    DateViewCompletion::CalendarPageCreation {
                        creation: self,
                        result,
                    },
                    cx,
                );
            },
        );
    }
}

impl CalendarRangeResizeJob {
    pub(in crate::ui::board_workspace) fn spawn(
        self,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<SurfaceState>,
    ) {
        let request = self.request.clone();
        spawn_background_task_for_entity(
            request,
            cx,
            move |request| workspace_api.set_calendar_page_date_range(request),
            move |surface, result, cx| {
                surface.finish_date_view_completion(
                    DateViewCompletion::CalendarRangeResize { job: self, result },
                    cx,
                );
            },
        );
    }
}

impl CalendarDateAssignmentJob {
    pub(in crate::ui::board_workspace) fn spawn(
        self,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<SurfaceState>,
    ) {
        let request = self.request.clone();
        spawn_background_task_for_entity(
            request,
            cx,
            move |request| workspace_api.set_calendar_page_date(request),
            move |surface, result, cx| {
                surface.finish_date_view_completion(
                    DateViewCompletion::CalendarDateAssignment { job: self, result },
                    cx,
                );
            },
        );
    }
}

impl DateUndatedQueryJob {
    pub(in crate::ui::board_workspace) fn spawn(
        self,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<SurfaceState>,
    ) {
        let Self { request, token } = self;
        spawn_background_task_for_entity(
            request,
            cx,
            move |request| workspace_api.load_calendar_items(request),
            move |surface, result, cx| {
                surface.finish_date_view_completion(
                    DateViewCompletion::UndatedItems { token, result },
                    cx,
                );
            },
        );
    }
}

impl DateUndatedCountJob {
    pub(in crate::ui::board_workspace) fn spawn(
        self,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        cx: &mut Context<SurfaceState>,
    ) {
        spawn_background_task_for_entity(
            (),
            cx,
            move |()| workspace_api.load_date_undated_count(),
            move |surface, result, cx| {
                surface.finish_date_view_completion(
                    DateViewCompletion::UndatedCount { job: self, result },
                    cx,
                );
            },
        );
    }
}

impl DateUndatedCountRetry {
    pub(in crate::ui::board_workspace) fn spawn(self, cx: &mut Context<SurfaceState>) {
        let delay = std::time::Duration::from_millis(500 * u64::from(self.retry_attempt));
        spawn_timer_task_for_entity(self, delay, cx, |surface, retry, cx| {
            surface.finish_date_view_completion(DateViewCompletion::UndatedCountRetry(retry), cx);
        });
    }
}
