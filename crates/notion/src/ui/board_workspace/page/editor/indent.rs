use super::editing::{PageEditEffect, PageEditSession, PageEditWriteEffect};
use super::support::{indent_page_block_in_page, page_block_subtree_end};
use super::{CardPage, TextInputSnapshot};
use crate::model::{PageBlockPlacement, PageMutation, ReorderPageBlockSubtreesRequest};

enum PageBlockIndentPlan {
    Indent { parent_id: String },
    Outdent { parent_id: String, after_id: String },
}

impl PageEditSession<'_> {
    pub(crate) fn indent_page_block(
        &mut self,
        block_id: &str,
        outdent: bool,
        snapshot: TextInputSnapshot,
    ) {
        if snapshot.is_composing {
            return;
        }
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let Some(index) = page
            .blocks
            .iter()
            .position(|block| block.block_id == block_id)
        else {
            return;
        };
        let Some(plan) = page_block_indent_plan(&page, index, outdent) else {
            return;
        };
        let page_block_id = page.block_id.clone();
        if let PageBlockIndentPlan::Indent { parent_id } = &plan {
            self.editor
                .expand_page_toggle_destination(self.documents, &page_block_id, parent_id);
        }
        self.editor.record_page_structural_edit(
            &page,
            Some(crate::ui::PageEditFocus::Block {
                block_id: block_id.to_string(),
                offset: snapshot.cursor,
            }),
        );
        let request = plan.apply(&mut page, index, block_id);
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id: page_block_id,
                mutation: PageMutation::ReorderSubtrees(request),
            },
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::FocusBlock {
            block_id: block_id.to_string(),
            offset: snapshot.cursor,
        });
        self.effects.push(PageEditEffect::Notify);
    }
}

fn page_block_indent_plan(
    page: &super::CardPage,
    index: usize,
    outdent: bool,
) -> Option<PageBlockIndentPlan> {
    let source_parent_id = page.blocks[index].parent_block_id.clone();
    if outdent {
        let parent = page
            .blocks
            .iter()
            .find(|block| block.block_id == source_parent_id)?;
        if parent.is_layout_container() {
            return None;
        }
        return Some(PageBlockIndentPlan::Outdent {
            parent_id: parent.parent_block_id.clone(),
            after_id: source_parent_id,
        });
    }
    let parent_id = (0..index)
        .rev()
        .find(|candidate| page.blocks[*candidate].parent_block_id == source_parent_id)
        .filter(|candidate| page.blocks[*candidate].can_accept_children())
        .map(|candidate| page.blocks[candidate].block_id.clone())?;
    Some(PageBlockIndentPlan::Indent { parent_id })
}

impl PageBlockIndentPlan {
    fn apply(
        self,
        page: &mut super::CardPage,
        index: usize,
        block_id: &str,
    ) -> ReorderPageBlockSubtreesRequest {
        match self {
            PageBlockIndentPlan::Indent { parent_id } => {
                indent_page_block_in_page(page, index);
                ReorderPageBlockSubtreesRequest {
                    target_parent_block_id: parent_id,
                    block_ids: vec![block_id.to_string()],
                    placement: PageBlockPlacement::Append,
                }
            }
            PageBlockIndentPlan::Outdent {
                parent_id,
                after_id,
            } => {
                outdent_page_block_in_page(page, index);
                ReorderPageBlockSubtreesRequest {
                    target_parent_block_id: parent_id,
                    block_ids: vec![block_id.to_string()],
                    placement: PageBlockPlacement::After(after_id),
                }
            }
        }
    }
}

pub(super) fn outdent_page_block_in_page(page: &mut CardPage, index: usize) {
    let parent_block_id = page.blocks[index].parent_block_id.clone();
    let Some(parent_index) = page
        .blocks
        .iter()
        .position(|block| block.block_id == parent_block_id)
    else {
        return;
    };
    let new_parent_block_id = page.blocks[parent_index].parent_block_id.clone();
    let subtree_end = page_block_subtree_end(&page.blocks, index);
    let mut moved = page.blocks.drain(index..subtree_end).collect::<Vec<_>>();
    for block in &mut moved {
        block.depth = block.depth.saturating_sub(1);
    }
    moved[0].parent_block_id = new_parent_block_id;
    let parent_end = page_block_subtree_end(&page.blocks, parent_index);
    page.blocks.splice(parent_end..parent_end, moved);
}
