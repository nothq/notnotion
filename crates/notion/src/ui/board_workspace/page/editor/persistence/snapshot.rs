use std::collections::HashMap;

use super::history::{page_code_history_mutation, page_divider_history_mutation};
use super::snapshot_plan::PreparedPageSnapshotTransition;
use super::PageMutationPlan;
use crate::model::{CardPage, CardPageBlock, PageMutation};

pub(super) struct PageSnapshotIndex<'a> {
    current: &'a CardPage,
    current_by_id: HashMap<&'a str, &'a CardPageBlock>,
    target_by_id: HashMap<&'a str, &'a CardPageBlock>,
    current_children_by_parent: HashMap<&'a str, Vec<&'a str>>,
    target_children_by_parent: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> PageSnapshotIndex<'a> {
    pub(super) fn new(current: &'a CardPage, target: &'a CardPage) -> Self {
        Self {
            current,
            current_by_id: current
                .blocks
                .iter()
                .map(|block| (block.block_id.as_str(), block))
                .collect(),
            target_by_id: target
                .blocks
                .iter()
                .map(|block| (block.block_id.as_str(), block))
                .collect(),
            current_children_by_parent: ordered_children_by_parent(current),
            target_children_by_parent: ordered_children_by_parent(target),
        }
    }

    pub(super) fn current_block(&self, block_id: &str) -> Option<&'a CardPageBlock> {
        self.current_by_id.get(block_id).copied()
    }

    pub(super) fn current_page(&self) -> &'a CardPage {
        self.current
    }

    pub(super) fn target_block(&self, block_id: &str) -> Option<&'a CardPageBlock> {
        self.target_by_id.get(block_id).copied()
    }

    pub(super) fn contains_current(&self, block_id: &str) -> bool {
        self.current_by_id.contains_key(block_id)
    }

    pub(super) fn contains_target(&self, block_id: &str) -> bool {
        self.target_by_id.contains_key(block_id)
    }

    pub(super) fn current_children(&self, parent_id: &str) -> &[&'a str] {
        self.current_children_by_parent
            .get(parent_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn target_children(&self, parent_id: &str) -> &[&'a str] {
        self.target_children_by_parent
            .get(parent_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

fn ordered_children_by_parent(page: &CardPage) -> HashMap<&str, Vec<&str>> {
    let mut children = HashMap::<&str, Vec<&str>>::new();
    for block in &page.blocks {
        children
            .entry(block.parent_block_id.as_str())
            .or_default()
            .push(block.block_id.as_str());
    }
    children
}

impl PageMutationPlan {
    pub(in crate::ui::board_workspace::page::editor) fn persist_page_snapshot_transition(
        &mut self,
        current: &CardPage,
        target: &CardPage,
    ) -> Result<(), String> {
        if let Some(transition) = page_code_history_mutation(current, target) {
            self.enqueue_page_mutation(&target.block_id, transition.into_page_mutation());
            self.persist_page_title_transition(current, target);
            return Ok(());
        }
        if let Some(transition) = page_divider_history_mutation(current, target) {
            self.enqueue_page_mutation(&target.block_id, transition.into_page_mutation());
            self.persist_page_title_transition(current, target);
            return Ok(());
        }

        let index = PageSnapshotIndex::new(current, target);
        let PreparedPageSnapshotTransition {
            column_resize,
            creations,
            reorder_requests,
            deletions,
        } = PreparedPageSnapshotTransition::new(current, target, &index)?;
        if let Some(request) = column_resize {
            self.enqueue_page_mutation(&target.block_id, PageMutation::ResizeColumns(request));
        }
        self.persist_prepared_page_block_creations(&target.block_id, creations);
        self.persist_target_page_block_order(target, reorder_requests);
        self.persist_updated_page_blocks(&index, target);
        self.persist_deleted_page_blocks(&target.block_id, deletions);
        self.persist_page_title_transition(current, target);
        Ok(())
    }

    fn persist_page_title_transition(&mut self, current: &CardPage, target: &CardPage) {
        if current.title == target.title {
            return;
        }
        self.persist_page_block_text(&target.block_id, &target.block_id, target.title.clone());
    }
}
