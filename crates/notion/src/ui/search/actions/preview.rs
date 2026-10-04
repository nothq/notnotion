use std::sync::Arc;

use gpui::Context;
use gpui_components::{spawn_background_task_for_entity, spawn_timer_task_for_entity};

use crate::model::NotionWorkspaceApi;
use crate::ui::search::action::{QuickFindCompletion, QuickFindPreviewJob, QuickFindTimer};
use crate::ui::SurfaceState;

pub(super) fn schedule(timer: QuickFindTimer, cx: &mut Context<SurfaceState>) {
    let delay = timer.delay();
    spawn_timer_task_for_entity(timer, delay, cx, |surface, timer, cx| {
        let effects = surface
            .notion_search
            .finish_quick_find_timer(timer, surface.notion_chrome.notion_search_open);
        surface.execute_quick_find_effects(effects, cx);
    });
}

pub(super) fn spawn_preview(
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    job: QuickFindPreviewJob,
    cx: &mut Context<SurfaceState>,
) {
    spawn_background_task_for_entity(
        job,
        cx,
        move |job| {
            let result = workspace_api.load_card_page_preview(&job.block_id);
            QuickFindCompletion::preview(job, result)
        },
        |surface, completion, cx| surface.finish_quick_find_completion(completion, cx),
    );
}
