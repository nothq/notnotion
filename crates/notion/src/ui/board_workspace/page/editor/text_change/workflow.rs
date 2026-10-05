use super::super::editing::PageEditSession;
use super::{
    apply_page_block_text_change, page_block_text_change_plan, CardPage, PageBlockPersistence,
    PageBlockTextChangePlan, PageBlockTextChangeState, PagePendingCrossBlockComposition,
    PageRichTextInsertion, TextInputSnapshot, VerifiedPageBlockMarkdownShortcut,
};

pub(super) struct PageBlockTextChangePreparation {
    pub(super) page: CardPage,
    pub(super) plan: PageBlockTextChangePlan,
    pub(super) pending_insertion: Option<PageRichTextInsertion>,
    pub(super) cross_block_composition: Option<PagePendingCrossBlockComposition>,
    pub(super) was_composing: bool,
    pub(super) composition_committed: bool,
}

pub(super) struct PageBlockTextChangeCommit<'change, 'state> {
    pub(super) page: CardPage,
    pub(super) pending_insertion: Option<PageRichTextInsertion>,
    pub(super) cross_block_composition: Option<PagePendingCrossBlockComposition>,
    pub(super) change: &'change PageBlockTextChangeState<'state>,
}

impl PageEditSession<'_> {
    pub(super) fn prepare_page_block_text_change(
        &mut self,
        block_id: &str,
        snapshot: &TextInputSnapshot,
    ) -> Option<PageBlockTextChangePreparation> {
        // A text input can emit one final change after its row has been removed by a toggle
        // collapse or page replacement. The current projection determines whether it is live.
        let data = self
            .documents
            .page_data_containing_editable_block(block_id)?;
        if !data.editable_block_is_visible(block_id) || !data.block_has_text_input(block_id) {
            return None;
        }
        let (was_composing, composition_committed) = self
            .editor
            .update_page_block_composition(block_id, snapshot.is_composing);
        let page = self.documents.page_containing_block(block_id)?;
        let cross_block_composition = self
            .editor
            .take_completed_cross_block_composition(block_id, composition_committed);
        let plan = page_block_text_change_plan(&page, block_id, snapshot)?;
        let pending_insertion = self.editor.take_page_rich_text_insertion(
            &page,
            block_id,
            snapshot,
            composition_committed,
        );
        Some(PageBlockTextChangePreparation {
            page,
            plan,
            pending_insertion,
            cross_block_composition,
            was_composing,
            composition_committed,
        })
    }

    pub(super) fn apply_terminal_page_markdown_shortcut(
        &mut self,
        page: CardPage,
        block_id: &str,
        plan: &PageBlockTextChangePlan,
    ) -> Option<CardPage> {
        match plan.markdown_shortcut.as_ref() {
            Some(VerifiedPageBlockMarkdownShortcut::Divider(shortcut)) => {
                self.apply_page_markdown_divider_shortcut(page, block_id, *shortcut);
                None
            }
            Some(VerifiedPageBlockMarkdownShortcut::Code(shortcut)) => {
                self.apply_page_markdown_code_shortcut(page, block_id, shortcut.clone());
                None
            }
            Some(VerifiedPageBlockMarkdownShortcut::Editable(_)) | None => Some(page),
        }
    }

    pub(super) fn commit_page_block_text_change(
        &mut self,
        commit: PageBlockTextChangeCommit<'_, '_>,
    ) {
        let PageBlockTextChangeCommit {
            mut page,
            pending_insertion,
            cross_block_composition,
            change,
        } = commit;
        let block_id = change.block_id;
        let snapshot = change.snapshot;
        let plan = change.plan;
        let page_block_id = page.block_id.clone();
        let local_page_changed =
            plan.text_changed || plan.markdown_shortcut.is_some() || pending_insertion.is_some();
        let Some(text) = apply_page_block_text_change(
            &mut page,
            block_id,
            snapshot,
            plan,
            pending_insertion.as_ref(),
        ) else {
            return;
        };
        self.replace_changed_page(page, local_page_changed);
        self.persist_page_block_change(PageBlockPersistence {
            page_block_id: &page_block_id,
            block_id,
            text,
            conversion: plan.markdown_shortcut.as_ref().and_then(|shortcut| {
                let VerifiedPageBlockMarkdownShortcut::Editable(conversion) = shortcut else {
                    return None;
                };
                Some(*conversion)
            }),
            insertion: pending_insertion,
            composition_active: snapshot.is_composing,
            cross_block_composition,
            should_persist: local_page_changed || change.composition_committed,
        });
        self.finish_page_block_text_change(change);
    }
}
