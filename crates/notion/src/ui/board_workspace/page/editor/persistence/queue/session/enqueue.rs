use crate::model::PageMutation;
use crate::ui::board_workspace::{PageWrite, PageWriteOperation};

use super::super::super::plan::{PageMutationPlan, PlannedPageWrite};
use super::super::action::PageMutationEffect;
use super::PageMutationSession;

impl PageMutationSession<'_> {
    pub(super) fn apply_plan(&mut self, plan: PageMutationPlan) -> Vec<PageMutationEffect> {
        let Some((planned, remaining)) = plan.split_first() else {
            return Vec::new();
        };
        let (page_id, write) = self.prepare_write(planned);
        let mut effects = self.enqueue_write(page_id, write);
        if !remaining.is_empty() {
            effects.push(PageMutationEffect::Continue(Box::new(
                super::super::action::PageMutationAction::ApplyPlan(remaining),
            )));
        }
        effects
    }

    fn prepare_write(&self, planned: PlannedPageWrite) -> (String, PageWrite) {
        match planned {
            PlannedPageWrite::Mutation { page_id, mutation } => {
                let projection = self
                    .documents
                    .page_mutation_text_projection(&page_id, &mutation);
                (page_id, PageWrite::mutation(mutation, projection))
            }
            PlannedPageWrite::ProjectedMutation {
                page_id,
                mutation,
                projection,
            } => (page_id, PageWrite::mutation(mutation, projection)),
            PlannedPageWrite::RichText {
                request,
                projection,
            } => {
                let page_id = request.page_block_id.clone();
                (page_id, PageWrite::rich_text(request, projection))
            }
            PlannedPageWrite::UnprojectedRichText(request) => {
                let projection = self.documents.page_rich_text_projection(&request);
                let page_id = request.page_block_id.clone();
                (page_id, PageWrite::rich_text(request, projection))
            }
        }
    }

    fn enqueue_write(&mut self, page_id: String, write: PageWrite) -> Vec<PageMutationEffect> {
        let nested_page_link_id = page_link_icon_target(&write);
        if nested_page_link_id
            .as_ref()
            .is_some_and(|block_id| self.page_link_icons.commit_in_flight(&page_id, block_id))
        {
            return Vec::new();
        }
        if self.environment.workspace_api.is_none() && !self.environment.cached_workspace_visible {
            return Vec::new();
        }
        let first_write_baseline = self
            .coordinator
            .needs_first_write_baseline(&page_id)
            .then(|| self.documents.page_authority_with_id(&page_id))
            .flatten();
        self.coordinator.enqueue_write(
            &page_id,
            self.environment.workspace_api.clone(),
            first_write_baseline,
            write,
        );
        if !self.coordinator.has_work(&page_id) {
            return self.finish_cancelled_lane(&page_id);
        }
        self.begin_search_mutation(&page_id, &page_id);
        if let Some(block_id) = nested_page_link_id {
            self.begin_search_mutation(&page_id, &block_id);
        }
        if let Some(page) = self.documents.page_with_id(&page_id) {
            self.coordinator.capture_optimistic_page(page);
        }
        self.start_page(page_id)
    }

    fn finish_cancelled_lane(&mut self, page_id: &str) -> Vec<PageMutationEffect> {
        if self.search.mutation_lane_is_dirty(page_id) {
            self.finish_search_mutation(page_id)
        } else {
            Vec::new()
        }
    }
}

fn page_link_icon_target(write: &PageWrite) -> Option<String> {
    match &write.operation {
        PageWriteOperation::Mutation(PageMutation::SetPageIcon(request)) => {
            Some(request.block_id().to_string())
        }
        PageWriteOperation::Mutation(_) | PageWriteOperation::RichText(_) => None,
    }
}
