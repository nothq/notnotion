use std::{collections::HashMap, ops::Range};

use gpui::App;

use super::super::editing::{
    PageEditEffect, PageEditSession, PageEditWriteEffect, PageEditorEffect,
};
use super::super::persistence::page_block_creation;
use super::super::support::page_block_subtree_end;
use super::super::{generated_notion_record_id, CardPage, CardPageBlock, PageMutationPlan};
use crate::model::{
    CreatePageBlockRequest, DuplicatePageAliasRequest, PageBlockPlacement, PageMutation,
};

struct DuplicatedPageBlockSubtree {
    blocks: Vec<CardPageBlock>,
    root_id: String,
    root_editable: bool,
}

impl PageEditSession<'_> {
    pub(in crate::ui::board_workspace::page::editor) fn duplicate_page_block(
        &mut self,
        block_id: &str,
        cx: &App,
    ) {
        let Some(mut page) = self.documents.page_containing_block(block_id) else {
            return;
        };
        let root_indices = self
            .editor
            .page_block_context_menu_root_indices(&page, block_id);
        if root_indices.is_empty() {
            return;
        }
        if let Some(source_index) = page_alias_duplicate_source(&page, &root_indices) {
            self.duplicate_page_alias(page, source_index, cx);
            return;
        }
        let ranges = page_block_duplicate_ranges(&page, &root_indices);
        if !page_block_duplicate_ranges_are_supported(&page, &ranges) {
            self.effects.push(PageEditEffect::Error(
                "Database and layout blocks cannot be duplicated through the page editor yet."
                    .to_string(),
            ));
            self.effects.push(PageEditEffect::Notify);
            return;
        }
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        let page_id = page.block_id.clone();
        let mut first_duplicate_root = None;
        for range in ranges.into_iter().rev() {
            let source_root_id = page.blocks[range.start].block_id.clone();
            let duplicate = duplicate_page_block_subtree(&page.blocks[range.clone()]);
            first_duplicate_root = Some((duplicate.root_id.clone(), duplicate.root_editable));
            self.persist_page_block_duplicate(&page_id, &source_root_id, &duplicate);
            page.blocks.splice(range.end..range.end, duplicate.blocks);
        }
        self.finish_page_block_duplication(page, first_duplicate_root);
    }

    fn duplicate_page_alias(&mut self, mut page: CardPage, source_index: usize, cx: &App) {
        let source = page.blocks[source_index].clone();
        let new_block_id = generated_notion_record_id();
        let mut duplicate = source.clone();
        duplicate.block_id.clone_from(&new_block_id);
        duplicate.last_edited = None;
        duplicate
            .alias_content_mut()
            .expect("validated alias duplicate must remain an alias")
            .copied_from_block_id = Some(source.block_id.clone());
        let focus = self.editor.current_page_edit_focus(cx);
        self.editor.record_page_structural_edit(&page, focus);
        page.blocks.insert(source_index + 1, duplicate);
        let request = DuplicatePageAliasRequest::new(source.block_id, new_block_id)
            .expect("generated alias duplicate IDs must be distinct and non-empty");
        let page_id = page.block_id.clone();
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::EnqueueMutation {
                page_id,
                mutation: PageMutation::DuplicateAlias(request),
            },
        ));
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenuAndSelection,
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        self.effects.push(PageEditEffect::Notify);
    }

    fn persist_page_block_duplicate(
        &mut self,
        page_id: &str,
        source_root_id: &str,
        duplicate: &DuplicatedPageBlockSubtree,
    ) {
        for (duplicate_index, block) in duplicate.blocks.iter().enumerate() {
            let (kind, text) = page_block_creation(block)
                .expect("validated duplicate block must have a creation shape");
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::EnqueueMutation {
                    page_id: page_id.to_string(),
                    mutation: PageMutation::CreateBlock(CreatePageBlockRequest {
                        block_id: block.block_id.clone(),
                        parent_block_id: block.parent_block_id.clone(),
                        kind,
                        text: text.to_string(),
                        placement: if duplicate_index == 0 {
                            PageBlockPlacement::After(source_root_id.to_string())
                        } else {
                            PageBlockPlacement::Append
                        },
                    }),
                },
            ));
            self.effects.push(PageEditEffect::Write(
                PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::annotations(
                    page_id, block,
                )),
            ));
        }
        self.effects.push(PageEditEffect::Write(
            PageEditWriteEffect::ApplyMutationPlan(PageMutationPlan::created_metadata(
                page_id,
                duplicate.blocks.iter(),
            )),
        ));
    }

    fn finish_page_block_duplication(
        &mut self,
        page: CardPage,
        first_duplicate_root: Option<(String, bool)>,
    ) {
        self.effects.push(PageEditEffect::Editor(
            PageEditorEffect::ClearBlockContextMenuAndSelection,
        ));
        self.effects.push(PageEditEffect::ReplaceLoadedPage(page));
        if let Some((duplicate_root_id, true)) = first_duplicate_root {
            self.effects.push(PageEditEffect::FocusBlock {
                block_id: duplicate_root_id,
                offset: 0,
            });
        }
        self.effects.push(PageEditEffect::Notify);
    }
}

fn page_block_duplicate_ranges(page: &CardPage, root_indices: &[usize]) -> Vec<Range<usize>> {
    root_indices
        .iter()
        .map(|index| *index..page_block_subtree_end(&page.blocks, *index))
        .collect()
}

fn page_block_duplicate_ranges_are_supported(page: &CardPage, ranges: &[Range<usize>]) -> bool {
    ranges.iter().all(|range| {
        page.blocks[range.clone()]
            .iter()
            .all(|block| page_block_creation(block).is_some())
    })
}

pub(super) fn page_alias_duplicate_source(
    page: &CardPage,
    root_indices: &[usize],
) -> Option<usize> {
    let [index] = root_indices else {
        return None;
    };
    let source = page.blocks.get(*index)?;
    (source.alias_content().is_some()
        && source.parent_block_id == page.block_id
        && page_block_subtree_end(&page.blocks, *index) == *index + 1)
        .then_some(*index)
}

fn duplicate_page_block_subtree(blocks: &[CardPageBlock]) -> DuplicatedPageBlockSubtree {
    let id_map = blocks
        .iter()
        .map(|block| (block.block_id.clone(), generated_notion_record_id()))
        .collect::<HashMap<_, _>>();
    let blocks = blocks
        .iter()
        .cloned()
        .map(|mut block| {
            block.block_id = id_map[&block.block_id].clone();
            block.last_edited = None;
            if let Some(parent_block_id) = id_map.get(&block.parent_block_id) {
                block.parent_block_id = parent_block_id.clone();
            }
            block
        })
        .collect::<Vec<_>>();
    DuplicatedPageBlockSubtree {
        root_id: blocks[0].block_id.clone(),
        root_editable: blocks[0].is_editable(),
        blocks,
    }
}
