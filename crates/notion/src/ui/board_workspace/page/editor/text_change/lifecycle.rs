use super::super::editing::{PageEditEffect, PageEditSession, PageEditorEffect};
use super::{
    apply_changed_text_annotations, CardPage, PageBlockTextChangePlan, PageRichTextInsertion,
    TextInputSnapshot,
};
use crate::ui::surface::PageEditorState;

pub(super) struct PageBlockTextChangeState<'a> {
    pub(super) block_id: &'a str,
    pub(super) snapshot: &'a TextInputSnapshot,
    pub(super) plan: &'a PageBlockTextChangePlan,
    pub(super) composition_committed: bool,
}

impl PageEditorState {
    pub(super) fn record_page_block_text_change_history(
        &mut self,
        page: &CardPage,
        change: &PageBlockTextChangeState<'_>,
        pending_insertion: Option<&PageRichTextInsertion>,
        was_composing: bool,
    ) {
        if change.plan.text_changed && !was_composing {
            self.record_page_text_edit(page, change.block_id, change.plan.previous_cursor);
        }
        if change.plan.markdown_shortcut.is_none() {
            return;
        }
        let mut literal_page = page.clone();
        let editable = literal_page
            .blocks
            .iter_mut()
            .find(|block| block.block_id == change.block_id)
            .and_then(|block| block.editable_content_mut())
            .expect("markdown shortcut target must be editable");
        apply_changed_text_annotations(editable, &change.snapshot.text, pending_insertion);
        editable.text.clone_from(&change.snapshot.text);
        self.record_page_structural_edit(
            &literal_page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: change.block_id.to_string(),
                offset: change.snapshot.cursor,
            }),
        );
    }
}

impl PageEditSession<'_> {
    pub(super) fn finish_page_block_text_change(&mut self, state: &PageBlockTextChangeState<'_>) {
        match &state.plan.markdown_shortcut {
            Some(super::VerifiedPageBlockMarkdownShortcut::Editable(_)) => {
                self.effects.push(PageEditEffect::Editor(
                    PageEditorEffect::FinishEditableMarkdown {
                        block_id: state.block_id.to_string(),
                    },
                ));
                self.effects.push(PageEditEffect::FocusBlock {
                    block_id: state.block_id.to_string(),
                    offset: 0,
                });
                self.effects.push(PageEditEffect::Notify);
                return;
            }
            Some(super::VerifiedPageBlockMarkdownShortcut::Divider(_)) => return,
            Some(super::VerifiedPageBlockMarkdownShortcut::Code(_)) => return,
            None => {}
        }
        self.effects.push(PageEditEffect::ReconcileTextMenus {
            block_id: state.block_id.to_string(),
            snapshot: state.snapshot.clone(),
            change_requires_notify: state.plan.text_changed || state.composition_committed,
        });
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn finish_page_markdown_divider(
        &mut self,
        block_id: &str,
    ) {
        self.input
            .resource_state()
            .block_inputs
            .borrow_mut()
            .remove(block_id);
        self.page_block_compositions.remove(block_id);
        self.clear_page_composer_handoff(block_id);
        self.page_slash_menu = None;
        self.mention.clear_menu();
        self.page_block_selection.block_ids.clear();
    }

    pub(in crate::ui::board_workspace::page::editor) fn finish_editable_page_markdown(
        &mut self,
        _block_id: &str,
    ) {
        self.page_slash_menu = None;
        self.mention.clear_menu();
    }
}
