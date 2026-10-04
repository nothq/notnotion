use std::collections::VecDeque;

use crate::ui::board_workspace::PageLinkIconController;
use crate::ui::surface::{NotionSearchState, PageDocuments, PageMutationCoordinator};
use crate::ui::{Arc, NotionWorkspaceApi};

use super::action::{
    PageMutationAction, PageMutationBackgroundJob, PageMutationEffect, PageMutationInternalAction,
    PageMutationProjectionReplacement,
};

mod enqueue;
mod recovery;
mod success;

pub(super) struct PageMutationEnvironment {
    pub(super) workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    pub(super) cached_workspace_visible: bool,
}

pub(super) struct PageMutationSession<'a> {
    pub(super) coordinator: &'a mut PageMutationCoordinator,
    pub(super) documents: &'a PageDocuments,
    pub(super) search: &'a mut NotionSearchState,
    pub(super) page_link_icons: &'a PageLinkIconController,
    pub(super) environment: PageMutationEnvironment,
}

impl<'a> PageMutationSession<'a> {
    pub(super) fn new(
        coordinator: &'a mut PageMutationCoordinator,
        documents: &'a PageDocuments,
        search: &'a mut NotionSearchState,
        page_link_icons: &'a PageLinkIconController,
        environment: PageMutationEnvironment,
    ) -> Self {
        Self {
            coordinator,
            documents,
            search,
            page_link_icons,
            environment,
        }
    }

    pub(super) fn reduce(&mut self, action: PageMutationAction) -> Vec<PageMutationEffect> {
        match action {
            PageMutationAction::ApplyPlan(plan) => self.apply_plan(plan),
            PageMutationAction::CaptureVisibleProjection => {
                self.capture_visible_projection();
                Vec::new()
            }
            PageMutationAction::RestoreVisibleProjection => self.restore_visible_projection(),
            PageMutationAction::StartPending => self.start_pending(),
            PageMutationAction::BeginQueuedReplay {
                page_id,
                baseline,
                workspace_api,
            } => self.begin_queued_replay(page_id, baseline, workspace_api),
            PageMutationAction::BeginSearchMutation {
                lane_page_id,
                block_id,
            } => {
                self.begin_search_mutation(&lane_page_id, &block_id);
                Vec::new()
            }
            PageMutationAction::PersistSearchMutation {
                lane_page_id,
                workspace_api,
            } => {
                self.persist_search_mutation(&lane_page_id, workspace_api.as_ref());
                Vec::new()
            }
            PageMutationAction::FinishSearchMutation { lane_page_id } => {
                self.finish_search_mutation(&lane_page_id)
            }
            PageMutationAction::Internal(action) => self.reduce_internal(action),
        }
    }

    fn reduce_internal(&mut self, action: PageMutationInternalAction) -> Vec<PageMutationEffect> {
        match action {
            PageMutationInternalAction::StartPage(page_id) => self.start_page(page_id),
            PageMutationInternalAction::StartPages(page_ids) => self.start_pages(page_ids),
            PageMutationInternalAction::WriteCompleted(completion) => {
                self.finish_write(*completion)
            }
            PageMutationInternalAction::ContinueWriteFailure { page_id, token } => {
                self.continue_write_failure(page_id, token)
            }
            PageMutationInternalAction::ContinueSuccessfulReplayRecovery(failure) => {
                self.continue_successful_replay_failure(*failure)
            }
            PageMutationInternalAction::RecoveryDelayElapsed { page_id, token } => {
                self.finish_recovery_delay(page_id, token)
            }
            PageMutationInternalAction::RecoveryLoaded(completion) => {
                self.finish_recovery_load(*completion)
            }
            PageMutationInternalAction::ContinueRecoveryFailure { page_id, token }
            | PageMutationInternalAction::DeferRecovery { page_id, token } => {
                self.defer_recovery(page_id, token)
            }
        }
    }

    fn capture_visible_projection(&mut self) {
        if let Some(page) = self.documents.visible_page_mutation_snapshot() {
            self.coordinator.capture_optimistic_page(page);
        }
    }

    fn restore_visible_projection(&mut self) -> Vec<PageMutationEffect> {
        let Some(mut page) = self.documents.visible_page_mutation_snapshot() else {
            return Vec::new();
        };
        let page_id = page.block_id.clone();
        let authority = self.coordinator.reconcile_external_load(&mut page, None);
        vec![PageMutationEffect::ReplaceLoaded(Box::new(
            PageMutationProjectionReplacement {
                page_id,
                page,
                authority,
                clear_history: false,
            },
        ))]
    }

    fn start_pending(&mut self) -> Vec<PageMutationEffect> {
        self.start_pages(self.coordinator.pending_page_ids().into())
    }

    fn start_pages(&mut self, mut page_ids: VecDeque<String>) -> Vec<PageMutationEffect> {
        let Some(page_id) = page_ids.pop_front() else {
            return Vec::new();
        };
        let mut effects = self.start_page(page_id);
        if !page_ids.is_empty() {
            effects.push(PageMutationEffect::Continue(Box::new(
                PageMutationAction::Internal(PageMutationInternalAction::StartPages(page_ids)),
            )));
        }
        effects
    }

    fn start_page(&mut self, page_id: String) -> Vec<PageMutationEffect> {
        let Some(dispatch) = self
            .coordinator
            .begin_next(&page_id, self.environment.workspace_api.clone())
        else {
            return Vec::new();
        };
        self.persist_search_mutation(&dispatch.page_id, dispatch.workspace_api.as_ref());
        vec![PageMutationEffect::RunBackground(
            PageMutationBackgroundJob::Write(Box::new(dispatch)),
        )]
    }

    pub(super) fn begin_search_mutation(&mut self, lane_page_id: &str, block_id: &str) {
        if !self.search.begin_page_mutation(lane_page_id, block_id) {
            return;
        }
        if let Some(workspace_api) = self.environment.workspace_api.as_deref() {
            workspace_api.invalidate_quick_find_page_mutation_in_memory(block_id);
        }
    }

    fn persist_search_mutation(
        &mut self,
        lane_page_id: &str,
        workspace_api: &dyn NotionWorkspaceApi,
    ) {
        self.search
            .persist_page_mutation(lane_page_id, workspace_api);
    }

    pub(super) fn finish_search_mutation(&mut self, lane_page_id: &str) -> Vec<PageMutationEffect> {
        let page_ids = self.search.finish_page_mutation_lane(lane_page_id);
        if page_ids.is_empty() {
            return Vec::new();
        }
        if let Some(workspace_api) = self.environment.workspace_api.as_deref() {
            for page_id in page_ids {
                workspace_api.invalidate_quick_find_page_mutation(&page_id);
            }
        }
        vec![PageMutationEffect::RefreshSearch]
    }
}
