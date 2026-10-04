use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::persistence::VerifiedPageTextBlockKind;
use super::super::rich_text::annotations::{
    annotated_text_slice, apply_annotated_text, concatenate_annotated_text,
};
use super::super::rich_text::{PageProjectedText, PageWriteTextProjection};
use super::super::support::{page_block_subtree_end, remove_merged_page_block};
use super::super::{CardPage, TextInputSnapshot};
use super::delete_empty::{EmptyPageBlockDeleteDirection, PreparedEmptyPageBlockDeletion};
use crate::model::{
    MergePageBlocksRequest, PageBlockPlacement, PageMutation, ReorderPageBlockSubtreesRequest,
};
use crate::ui::board_workspace::page::editor::PageMutationPlan;

struct PreparedPageBlockMerge {
    page: CardPage,
    source_index: usize,
    target_index: usize,
    source_editable: crate::ui::CardPageEditableBlock,
    target_editable: crate::ui::CardPageEditableBlock,
    edited_block_id: String,
    snapshot: TextInputSnapshot,
}

struct ActivePageBlockEdit<'a> {
    index: usize,
    block_id: &'a str,
    snapshot: &'a TextInputSnapshot,
}

impl PageEditSession<'_> {
    pub(crate) fn merge_page_block_backward(
        &mut self,
        block_id: &str,
        snapshot: TextInputSnapshot,
    ) {
        if snapshot.is_composing {
            return;
        }
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let Some(index) = page_block_index(&page, block_id) else {
            return;
        };
        let edit = ActivePageBlockEdit {
            index,
            block_id,
            snapshot: &snapshot,
        };
        if self.convert_empty_page_block(&mut page, &edit) {
            return;
        }
        if self.outdent_empty_page_block(&mut page, &edit) {
            return;
        }
        if let Some(deletion) = PreparedEmptyPageBlockDeletion::new(
            &page,
            index,
            block_id,
            &snapshot,
            EmptyPageBlockDeleteDirection::Backward,
        ) {
            deletion.apply(self, page);
            return;
        }
        let parent_block_id = page.blocks[index].parent_block_id.clone();
        let Some(previous_index) = (0..index)
            .rev()
            .find(|candidate| page.blocks[*candidate].parent_block_id == parent_block_id)
        else {
            return;
        };
        let Some(merge) =
            prepare_page_block_merge(page, index, previous_index, block_id.to_string(), snapshot)
        else {
            return;
        };
        self.merge_prepared_page_blocks(merge);
    }

    fn convert_empty_page_block(
        &mut self,
        page: &mut CardPage,
        edit: &ActivePageBlockEdit<'_>,
    ) -> bool {
        let Some(editable) = page.blocks[edit.index].editable_content() else {
            return true;
        };
        let Some(source_kind) = VerifiedPageTextBlockKind::parse(editable.kind) else {
            return true;
        };
        if !edit.snapshot.text.is_empty()
            || source_kind.card_kind() == VerifiedPageTextBlockKind::text().card_kind()
        {
            return false;
        }
        let target_kind = VerifiedPageTextBlockKind::text();
        self.editor.record_page_structural_edit(
            page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: edit.block_id.to_string(),
                offset: edit.snapshot.cursor,
            }),
        );
        page.blocks[edit.index]
            .editable_content_mut()
            .expect("active page block must remain editable")
            .set_kind(target_kind.card_kind());
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page.block_id.clone(),
                mutation: PageMutation::ConvertBlock(
                    target_kind.conversion_request(edit.block_id.to_string()),
                ),
            },
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(page.clone()));
        self.effects.push(PageEditEffect::Notify);
        true
    }

    fn outdent_empty_page_block(
        &mut self,
        page: &mut CardPage,
        edit: &ActivePageBlockEdit<'_>,
    ) -> bool {
        if !edit.snapshot.text.is_empty() || page.blocks[edit.index].depth == 0 {
            return false;
        }
        let source_parent_id = page.blocks[edit.index].parent_block_id.clone();
        let Some(parent) = page
            .blocks
            .iter()
            .find(|block| block.block_id == source_parent_id)
            .cloned()
        else {
            return true;
        };
        if parent.is_layout_container() {
            return false;
        }
        self.editor.record_page_structural_edit(
            page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: edit.block_id.to_string(),
                offset: edit.snapshot.cursor,
            }),
        );
        super::super::indent::outdent_page_block_in_page(page, edit.index);
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page.block_id.clone(),
                mutation: PageMutation::ReorderSubtrees(ReorderPageBlockSubtreesRequest {
                    target_parent_block_id: parent.parent_block_id,
                    block_ids: vec![edit.block_id.to_string()],
                    placement: PageBlockPlacement::After(source_parent_id),
                }),
            },
        ));
        self.effects
            .push(PageEditEffect::ReplaceLoadedPage(page.clone()));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: edit.block_id.to_string(),
            offset: 0,
        });
        self.effects.push(PageEditEffect::Notify);
        true
    }

    pub(crate) fn merge_page_block_forward(&mut self, block_id: &str, snapshot: TextInputSnapshot) {
        if snapshot.is_composing {
            return;
        }
        let Some(page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let Some(index) = page_block_index(&page, block_id) else {
            return;
        };
        if let Some(deletion) = PreparedEmptyPageBlockDeletion::new(
            &page,
            index,
            block_id,
            &snapshot,
            EmptyPageBlockDeleteDirection::Forward,
        ) {
            deletion.apply(self, page);
            return;
        }
        let parent_block_id = page.blocks[index].parent_block_id.clone();
        let subtree_end = page_block_subtree_end(&page.blocks, index);
        let Some(next_index) = (subtree_end..page.blocks.len())
            .find(|candidate| page.blocks[*candidate].parent_block_id == parent_block_id)
        else {
            return;
        };
        let Some(merge) =
            prepare_page_block_merge(page, next_index, index, block_id.to_string(), snapshot)
        else {
            return;
        };
        self.merge_prepared_page_blocks(merge);
    }

    fn merge_prepared_page_blocks(&mut self, merge: PreparedPageBlockMerge) {
        let PreparedPageBlockMerge {
            mut page,
            source_index,
            target_index,
            source_editable,
            target_editable,
            edited_block_id,
            snapshot,
        } = merge;
        self.editor.record_page_structural_edit(
            &page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: edited_block_id.clone(),
                offset: snapshot.cursor,
            }),
        );
        let page_block_id = page.block_id.clone();
        let source_block_id = page.blocks[source_index].block_id.clone();
        let target_block_id = page.blocks[target_index].block_id.clone();
        let target_length = target_editable.text.len();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::text(
                &page_block_id,
                &edited_block_id,
                snapshot.text,
            )),
        ));
        let projection = merge_page_blocks_in_page(
            &mut page,
            source_index,
            target_index,
            &source_editable,
            &target_editable,
        );
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutationWithProjection {
                page_id: page_block_id,
                mutation: PageMutation::MergeBlocks(MergePageBlocksRequest {
                    source_block_id: source_block_id.clone(),
                    target_block_id: target_block_id.clone(),
                }),
                projection,
            },
        ));
        self.effects
            .push(PageEditEffect::Editor(PageEditorEffect::RemoveBlockInput {
                block_id: source_block_id,
            }));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: target_block_id,
            offset: target_length,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}

fn merge_page_blocks_in_page(
    page: &mut CardPage,
    source_index: usize,
    target_index: usize,
    source_editable: &crate::ui::CardPageEditableBlock,
    target_editable: &crate::ui::CardPageEditableBlock,
) -> PageWriteTextProjection {
    let merged = concatenate_annotated_text([
        annotated_text_slice(target_editable, 0..target_editable.text.len()),
        annotated_text_slice(source_editable, 0..source_editable.text.len()),
    ]);
    apply_annotated_text(
        page.blocks[target_index]
            .editable_content_mut()
            .expect("merge target must remain editable"),
        merged,
    );
    let target_projection = PageProjectedText::block(&page.blocks[target_index])
        .expect("merged page block target must remain editable");
    let retired_ids = if source_editable.kind == crate::ui::CardPageBlockKind::ToggleList {
        let subtree_end = page_block_subtree_end(&page.blocks, source_index);
        page.blocks[source_index..subtree_end]
            .iter()
            .map(|block| block.block_id.clone())
            .collect()
    } else {
        vec![page.blocks[source_index].block_id.clone()]
    };
    remove_merged_page_block(&mut page.blocks, source_index);
    PageWriteTextProjection::update(target_projection).with_retired_blocks(retired_ids)
}

fn prepare_page_block_merge(
    page: CardPage,
    source_index: usize,
    target_index: usize,
    edited_block_id: String,
    snapshot: TextInputSnapshot,
) -> Option<PreparedPageBlockMerge> {
    let source_editable = page.blocks.get(source_index)?.editable_content()?.clone();
    let target_editable = page.blocks.get(target_index)?.editable_content()?.clone();
    // A block holding an inline token notnotion cannot round-trip is read-only, and
    // merging would rewrite its title as plain text.
    if source_editable.read_only.is_some() || target_editable.read_only.is_some() {
        return None;
    }
    VerifiedPageTextBlockKind::parse(source_editable.kind)?;
    VerifiedPageTextBlockKind::parse(target_editable.kind)?;
    let edited_text = if page.blocks[source_index].block_id == edited_block_id {
        &source_editable.text
    } else if page.blocks[target_index].block_id == edited_block_id {
        &target_editable.text
    } else {
        return None;
    };
    assert_eq!(
        *edited_text, snapshot.text,
        "merge snapshot must match the loaded edited block"
    );
    Some(PreparedPageBlockMerge {
        page,
        source_index,
        target_index,
        source_editable,
        target_editable,
        edited_block_id,
        snapshot,
    })
}

fn page_block_index(page: &CardPage, block_id: &str) -> Option<usize> {
    page.blocks
        .iter()
        .position(|block| block.block_id == block_id)
}
