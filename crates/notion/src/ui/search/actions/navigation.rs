use std::sync::Arc;

use gpui::Context;
use gpui_components::spawn_background_task_for_entity;

use crate::model::NotionWorkspaceApi;
use crate::ui::search::action::{QuickFindCompletion, QuickFindQueryJob, QuickFindVisitJob};
use crate::ui::SurfaceState;

pub(super) fn spawn_query(
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    job: QuickFindQueryJob,
    cx: &mut Context<SurfaceState>,
) {
    spawn_background_task_for_entity(
        job,
        cx,
        move |job| {
            let result = workspace_api.search_workspace(job.request.clone());
            QuickFindCompletion::query(job, result)
        },
        |surface, completion, cx| surface.finish_quick_find_completion(completion, cx),
    );
}

pub(super) fn spawn_visit(
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    job: QuickFindVisitJob,
    cx: &mut Context<SurfaceState>,
) {
    spawn_background_task_for_entity(
        job,
        cx,
        move |job| workspace_api.record_recent_page_visit(job.recent_page),
        |surface, completion, cx| {
            surface.finish_quick_find_completion(QuickFindCompletion::visit(completion), cx);
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::ui::search::action::quick_find_should_prefetch;

    #[test]
    fn keyboard_pagination_arms_within_three_results_of_the_tail() {
        assert!(!quick_find_should_prefetch(0, 0));
        assert!(!quick_find_should_prefetch(15, 20));
        assert!(quick_find_should_prefetch(16, 20));
        assert!(quick_find_should_prefetch(19, 20));
    }
}
