use std::sync::Arc;

use gpui::Context;
use gpui_components::{spawn_background_task_for_entity, spawn_timer_task_for_entity};

use super::super::actions::PageMentionAction;
use super::super::data::{
    PageMentionCompletion, MENTION_PAGE_RESULT_LIMIT, MENTION_SEARCH_DEBOUNCE,
};
use crate::model::{
    LoadRecentPagesRequest, LoadRecentPagesResult, NotionWorkspaceApi, SearchWorkspaceRequest,
};
use crate::ui::SurfaceState;

pub(super) struct PageMentionPeopleJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
}

pub(super) struct PageMentionRecentPagesJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) current_board_url: String,
    pub(super) generation: u64,
}

pub(super) struct PageMentionSearchDelay {
    pub(super) generation: u64,
    pub(super) query: String,
}

pub(super) struct PageMentionSearchJob {
    pub(super) workspace_api: Arc<dyn NotionWorkspaceApi>,
    pub(super) generation: u64,
    pub(super) query: String,
    pub(super) request: SearchWorkspaceRequest,
}

impl PageMentionPeopleJob {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        spawn_background_task_for_entity(
            self.workspace_api,
            cx,
            |workspace_api| workspace_api.load_comment_users(),
            |surface, result, cx| {
                surface.finish_page_mention_completion(PageMentionCompletion::People(result), cx);
            },
        );
    }
}

impl PageMentionRecentPagesJob {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        let request = LoadRecentPagesRequest {
            current_board_url: self.current_board_url,
            limit: MENTION_PAGE_RESULT_LIMIT as u8,
        };
        spawn_background_task_for_entity(
            (self.workspace_api, self.generation, request),
            cx,
            |(workspace_api, generation, request)| {
                (
                    generation,
                    workspace_api
                        .load_recent_pages(request)
                        .map(LoadRecentPagesResult::into_pages),
                )
            },
            |surface, (generation, result), cx| {
                surface.finish_page_mention_completion(
                    PageMentionCompletion::Pages {
                        generation,
                        query: String::new(),
                        result,
                    },
                    cx,
                );
            },
        );
    }
}

impl PageMentionSearchDelay {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        spawn_timer_task_for_entity(
            (self.generation, self.query),
            MENTION_SEARCH_DEBOUNCE,
            cx,
            |surface, (generation, query), cx| {
                surface.dispatch_page_mention_action(
                    PageMentionAction::SearchDelayElapsed { generation, query },
                    cx,
                );
            },
        );
    }
}

impl PageMentionSearchJob {
    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        spawn_background_task_for_entity(
            (
                self.workspace_api,
                self.generation,
                self.query,
                self.request,
            ),
            cx,
            |(workspace_api, generation, query, request)| {
                (
                    generation,
                    query,
                    workspace_api
                        .search_workspace(request)
                        .map(|result| result.results),
                )
            },
            |surface, (generation, query, result), cx| {
                surface.finish_page_mention_completion(
                    PageMentionCompletion::Pages {
                        generation,
                        query,
                        result,
                    },
                    cx,
                );
            },
        );
    }
}
