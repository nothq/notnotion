use super::super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::super::mention::PageMentionAction;
use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::support::page_block_subtree_end;
use super::super::{
    generated_notion_record_id, CardPage, CardPageBlock, CardPageBlockKind, Context, SurfaceState,
    TextInputSnapshot,
};
use crate::model::{CreatePageBlockRequest, PageBlockPlacement, PageMutation};
use crate::ui::surface::PageEditorState;
use crate::ui::PageEditFocus;

use self::prepared::{AppliedPageBlockSplit, PreparedPageBlockSplit};

mod prepared;

struct PreparedEmptyPageBlockConversion {
    page: CardPage,
    index: usize,
    block_id: String,
    cursor: usize,
}

struct LoadedPageBlockSplitSource {
    page: CardPage,
    index: usize,
    block: CardPageBlock,
    editable: crate::ui::CardPageEditableBlock,
}

impl SurfaceState {
    pub(crate) fn submit_page_block(
        &mut self,
        block_id: &str,
        snapshot: TextInputSnapshot,
        cx: &mut Context<Self>,
    ) {
        let action = PageMentionAction::reconcile(&self.page_documents, block_id, &snapshot);
        self.dispatch_page_mention_action(action, cx);
        if self
            .dispatch_page_mention_action(PageMentionAction::commit(block_id), cx)
            .consumed
        {
            return;
        }
        let transition = {
            let mut edit = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            edit.update_native_page_slash_menu(block_id, &snapshot);
            let consumed = edit.commit_native_page_slash_selection(block_id);
            edit.finish(consumed)
        };
        if self.apply_page_edit_transition(transition, cx) {
            return;
        }
        let transition = {
            let mut session = PageEditSession::new(&mut self.page_editor, &self.page_documents);
            session.split_page_block(block_id, snapshot);
            session.finish(())
        };
        self.apply_page_edit_transition(transition, cx);
    }
}

impl PageEditSession<'_> {
    pub(crate) fn split_page_block(&mut self, block_id: &str, snapshot: TextInputSnapshot) {
        if snapshot.is_composing {
            return;
        }
        let Some(source) = loaded_page_block_split_source(self.documents, block_id) else {
            return;
        };
        if source.editable.kind == CardPageBlockKind::PageLink {
            self.submit_page_link_on_enter(source, block_id, snapshot.cursor);
            return;
        }
        let Some(kind) = VerifiedPageTextBlockKind::parse(source.editable.kind) else {
            return;
        };
        self.split_verified_page_block(source, kind, snapshot);
    }
}

impl PageEditorState {
    fn page_toggle_accepts_first_child(
        &self,
        page: &CardPage,
        block_id: &str,
        editable: &crate::ui::CardPageEditableBlock,
        snapshot: &TextInputSnapshot,
    ) -> bool {
        editable.kind == CardPageBlockKind::ToggleList
            && snapshot.selection.start == snapshot.text.len()
            && snapshot.selection.end == snapshot.text.len()
            && self
                .page_toggle_disclosure
                .disclosure(&page.block_id, block_id)
                .is_expanded()
    }
}

impl PageEditSession<'_> {
    fn split_verified_page_block(
        &mut self,
        source: LoadedPageBlockSplitSource,
        kind: VerifiedPageTextBlockKind,
        snapshot: TextInputSnapshot,
    ) {
        let LoadedPageBlockSplitSource {
            page,
            index,
            block,
            editable,
        } = source;
        let block_id = block.block_id.clone();
        let accepts_first_child = self
            .editor
            .page_toggle_accepts_first_child(&page, &block_id, &editable, &snapshot);
        if accepts_first_child {
            let focus = Some(PageEditFocus::block(block_id, snapshot.cursor));
            self.insert_first_page_toggle_child_in_page(page, index, block, focus);
            return;
        }
        if empty_page_block_should_outdent(&page, &block, kind, &snapshot) {
            self.indent_page_block(&block_id, true, snapshot);
            return;
        }
        if empty_page_block_converts_to_text(kind, &snapshot) {
            self.convert_empty_page_block_on_enter(PreparedEmptyPageBlockConversion {
                page,
                index,
                block_id,
                cursor: snapshot.cursor,
            });
            return;
        }
        let source = LoadedPageBlockSplitSource {
            page,
            index,
            block,
            editable,
        };
        let focus = PageEditFocus::block(&source.block.block_id, snapshot.cursor);
        self.editor
            .record_page_structural_edit(&source.page, Some(focus));
        self.split_prepared_page_block(source.prepare(kind, snapshot));
    }

    fn submit_page_link_on_enter(
        &mut self,
        source: LoadedPageBlockSplitSource,
        block_id: &str,
        cursor: usize,
    ) {
        let LoadedPageBlockSplitSource {
            mut page,
            index,
            block,
            ..
        } = source;
        self.editor.record_page_structural_edit(
            &page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: block_id.to_string(),
                offset: cursor,
            }),
        );
        self.insert_text_block_after_page_link(&mut page, index, block);
    }

    fn convert_empty_page_block_on_enter(&mut self, conversion: PreparedEmptyPageBlockConversion) {
        let PreparedEmptyPageBlockConversion {
            mut page,
            index,
            block_id,
            cursor,
        } = conversion;
        let target_kind = VerifiedPageTextBlockKind::text();
        self.editor.record_page_structural_edit(
            &page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: block_id.clone(),
                offset: cursor,
            }),
        );
        page.blocks[index]
            .editable_content_mut()
            .expect("empty page block must remain editable")
            .set_kind(target_kind.card_kind());
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page.block_id.clone(),
                mutation: PageMutation::ConvertBlock(
                    target_kind.conversion_request(block_id.clone()),
                ),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }

    fn split_prepared_page_block(&mut self, split: PreparedPageBlockSplit) {
        let AppliedPageBlockSplit {
            page,
            page_id,
            text_plan,
            mutation,
            projection,
            new_block_id,
        } = split.apply();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(text_plan),
        ));
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutationWithProjection {
                page_id,
                mutation,
                projection,
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: new_block_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }

    fn insert_text_block_after_page_link(
        &mut self,
        page: &mut CardPage,
        index: usize,
        page_link: CardPageBlock,
    ) {
        let page_block_id = page.block_id.clone();
        let new_block_id = generated_notion_record_id();
        let insertion_index = page_block_subtree_end(&page.blocks, index);
        let new_block = CardPageBlock::editable(
            new_block_id.clone(),
            page_link.parent_block_id,
            page_link.depth,
            CardPageBlockKind::Text,
            String::new(),
        );
        page.blocks.insert(insertion_index, new_block.clone());
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page_block_id,
                mutation: PageMutation::CreateBlock(CreatePageBlockRequest {
                    block_id: new_block_id.clone(),
                    parent_block_id: new_block.parent_block_id,
                    kind: crate::model::NotionPageBlockKind::Text,
                    text: String::new(),
                    placement: PageBlockPlacement::After(page_link.block_id),
                }),
            },
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(page.clone()));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: new_block_id,
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}

fn page_block_index(page: &CardPage, block_id: &str) -> Option<usize> {
    page.blocks
        .iter()
        .position(|block| block.block_id == block_id)
}

fn loaded_page_block_split_source(
    documents: &crate::ui::surface::PageDocuments,
    block_id: &str,
) -> Option<LoadedPageBlockSplitSource> {
    let page = documents.page_containing_block(block_id)?;
    let index = page_block_index(&page, block_id)?;
    let block = page.blocks[index].clone();
    let editable = block.editable_content()?.clone();
    Some(LoadedPageBlockSplitSource {
        page,
        index,
        block,
        editable,
    })
}

fn page_block_parent_is_layout(page: &CardPage, block: &CardPageBlock) -> bool {
    page.blocks
        .iter()
        .find(|candidate| candidate.block_id == block.parent_block_id)
        .is_some_and(CardPageBlock::is_layout_container)
}

fn empty_page_block_should_outdent(
    page: &CardPage,
    block: &CardPageBlock,
    kind: VerifiedPageTextBlockKind,
    snapshot: &TextInputSnapshot,
) -> bool {
    snapshot.text.is_empty()
        && block.depth > 0
        && !page_block_parent_is_layout(page, block)
        && kind.card_kind() != VerifiedPageTextBlockKind::text().card_kind()
        && kind.split_kind().card_kind() == kind.card_kind()
}

fn empty_page_block_converts_to_text(
    kind: VerifiedPageTextBlockKind,
    snapshot: &TextInputSnapshot,
) -> bool {
    snapshot.text.is_empty() && kind.card_kind() != VerifiedPageTextBlockKind::text().card_kind()
}
