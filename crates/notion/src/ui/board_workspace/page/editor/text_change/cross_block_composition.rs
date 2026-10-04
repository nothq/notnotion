use super::super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::super::PagePendingCrossBlockComposition;
use crate::ui::surface::PageEditorState;

impl PageEditorState {
    pub(super) fn take_completed_cross_block_composition(
        &mut self,
        block_id: &str,
        composition_committed: bool,
    ) -> Option<PagePendingCrossBlockComposition> {
        if !composition_committed
            || !self
                .page_pending_cross_block_composition
                .as_ref()
                .is_some_and(|composition| composition.matches(block_id))
        {
            return None;
        }
        self.page_pending_cross_block_composition.take()
    }
}

impl PageEditSession<'_> {
    pub(super) fn persist_completed_cross_block_composition(
        &mut self,
        composition: PagePendingCrossBlockComposition,
        replacement: String,
    ) {
        let (page_id, survivor_id, removed_ids, mutation) =
            composition.into_persistence(replacement);
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueCompletedCrossBlockComposition {
                page_id,
                survivor_id,
                removed_ids,
                mutation,
            },
        ));
    }
}
