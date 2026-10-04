use std::sync::Arc;

use gpui::Context;
use gpui_components::spawn_background_task_for_entity;

use crate::model::NotionWorkspaceApi;
use crate::ui::search::action::{
    QuickFindCompletion, QuickFindEffect, QuickFindRecentsJob, QuickFindRecentsSource,
    QuickFindRequest,
};
use crate::ui::SurfaceState;

impl SurfaceState {
    pub(crate) fn load_notion_search_recents(&mut self, cx: &mut Context<Self>) {
        self.execute_quick_find_effects(
            vec![QuickFindEffect::Request(QuickFindRequest::RefreshRecents)],
            cx,
        );
    }
}

pub(super) fn source(
    workspace_api: &Arc<dyn NotionWorkspaceApi>,
    current_page_id: Option<String>,
    current_board_url: String,
) -> QuickFindRecentsSource {
    QuickFindRecentsSource {
        cache_scope: workspace_api.quick_find_cache_scope(),
        current_page_id,
        current_board_url,
        persisted_results: workspace_api
            .cached_recent_pages()
            .map(|cached| cached.results),
        local_search: workspace_api.cached_quick_find_local_search(),
    }
}

pub(super) fn spawn_recents(
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    job: QuickFindRecentsJob,
    cx: &mut Context<SurfaceState>,
) {
    spawn_background_task_for_entity(
        job,
        cx,
        move |job| {
            let result = workspace_api.load_recent_pages(job.request);
            QuickFindCompletion::recents(job.request_token, result)
        },
        |surface, completion, cx| surface.finish_quick_find_completion(completion, cx),
    );
}
