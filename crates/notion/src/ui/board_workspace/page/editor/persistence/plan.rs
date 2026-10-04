use std::collections::VecDeque;

use super::super::rich_text::PageWriteTextProjection;
use super::PageBlockTextConversion;
use crate::model::{CardPage, CardPageBlock, EditPageBlockTextRequest, PageMutation};

/// An ordered, prepared set of page writes. Planning has no UI or network effects.
#[derive(Default)]
pub(crate) struct PageMutationPlan {
    writes: VecDeque<PlannedPageWrite>,
}

pub(super) enum PlannedPageWrite {
    Mutation {
        page_id: String,
        mutation: PageMutation,
    },
    ProjectedMutation {
        page_id: String,
        mutation: PageMutation,
        projection: PageWriteTextProjection,
    },
    RichText {
        request: EditPageBlockTextRequest,
        projection: PageWriteTextProjection,
    },
    UnprojectedRichText(EditPageBlockTextRequest),
}

impl PageMutationPlan {
    pub(crate) fn mutation(page_id: impl Into<String>, mutation: PageMutation) -> Self {
        Self {
            writes: VecDeque::from([PlannedPageWrite::Mutation {
                page_id: page_id.into(),
                mutation,
            }]),
        }
    }

    pub(crate) fn projected_mutation(
        page_id: impl Into<String>,
        mutation: PageMutation,
        projection: PageWriteTextProjection,
    ) -> Self {
        Self {
            writes: VecDeque::from([PlannedPageWrite::ProjectedMutation {
                page_id: page_id.into(),
                mutation,
                projection,
            }]),
        }
    }

    pub(crate) fn rich_text(request: EditPageBlockTextRequest) -> Self {
        Self {
            writes: VecDeque::from([PlannedPageWrite::UnprojectedRichText(request)]),
        }
    }

    pub(in crate::ui::board_workspace::page) fn snapshot_transition(
        current: &CardPage,
        target: &CardPage,
    ) -> Result<Self, String> {
        let mut plan = Self::default();
        plan.persist_page_snapshot_transition(current, target)?;
        Ok(plan)
    }

    pub(in crate::ui::board_workspace::page) fn appended_block(
        page_id: &str,
        block: &CardPageBlock,
    ) -> Self {
        let mut plan = Self::default();
        plan.persist_appended_page_block(page_id, block);
        plan
    }

    pub(in crate::ui::board_workspace::page) fn created_metadata<'a>(
        page_id: &str,
        blocks: impl IntoIterator<Item = &'a CardPageBlock>,
    ) -> Self {
        let mut plan = Self::default();
        plan.persist_created_page_block_metadata(page_id, blocks);
        plan
    }

    pub(in crate::ui::board_workspace::page) fn annotations(
        page_id: &str,
        block: &CardPageBlock,
    ) -> Self {
        let mut plan = Self::default();
        plan.persist_page_block_annotations(page_id, block);
        plan
    }

    pub(in crate::ui::board_workspace::page) fn text(
        page_id: &str,
        block_id: &str,
        text: String,
    ) -> Self {
        let mut plan = Self::default();
        plan.persist_page_block_text(page_id, block_id, text);
        plan
    }

    pub(in crate::ui::board_workspace::page::editor) fn text_and_convert(
        page_id: &str,
        block_id: &str,
        change: PageBlockTextConversion,
    ) -> Self {
        let mut plan = Self::default();
        plan.persist_page_block_text_and_convert(page_id, block_id, change);
        plan
    }

    pub(super) fn enqueue_page_mutation(&mut self, page_id: &str, mutation: PageMutation) {
        self.writes.push_back(PlannedPageWrite::Mutation {
            page_id: page_id.to_string(),
            mutation,
        });
    }

    pub(super) fn enqueue_page_mutation_with_projection(
        &mut self,
        page_id: &str,
        mutation: PageMutation,
        projection: PageWriteTextProjection,
    ) {
        self.writes.push_back(PlannedPageWrite::ProjectedMutation {
            page_id: page_id.to_string(),
            mutation,
            projection,
        });
    }

    pub(super) fn enqueue_page_rich_text_edit_with_projection(
        &mut self,
        request: EditPageBlockTextRequest,
        projection: PageWriteTextProjection,
    ) {
        self.writes.push_back(PlannedPageWrite::RichText {
            request,
            projection,
        });
    }

    pub(super) fn split_first(mut self) -> Option<(PlannedPageWrite, Self)> {
        let first = self.writes.pop_front()?;
        Some((first, self))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
}
