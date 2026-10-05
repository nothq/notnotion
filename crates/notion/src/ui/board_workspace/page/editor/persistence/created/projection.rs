use std::collections::{HashMap, HashSet};

use super::{PreparedPageBlockCreation, PreparedPageBlockCreations};
use crate::model::{CardPage, CardPageBlock};

pub(super) enum PreparedPageBlockCreationPlacement {
    Append(String),
    AfterLeaf(String),
}

impl PreparedPageBlockCreations {
    pub(in crate::ui::board_workspace::page::editor::persistence) fn projected_page(
        &self,
        current: &CardPage,
    ) -> Result<CardPage, String> {
        PreparedCreationProjection::new(current, &self.blocks).build()
    }
}

#[derive(Clone, Copy)]
enum ProjectedBlock {
    Current(usize),
    Creation(usize),
}

struct PreparedCreationProjection<'a> {
    current: &'a CardPage,
    creations: &'a [PreparedPageBlockCreation],
    current_children: HashMap<&'a str, Vec<usize>>,
    appended: HashMap<&'a str, Vec<usize>>,
    after_leaf: HashMap<&'a str, Vec<usize>>,
}

impl<'a> PreparedCreationProjection<'a> {
    fn new(current: &'a CardPage, creations: &'a [PreparedPageBlockCreation]) -> Self {
        let mut current_children = HashMap::<&str, Vec<usize>>::new();
        for (index, block) in current.blocks.iter().enumerate() {
            current_children
                .entry(&block.parent_block_id)
                .or_default()
                .push(index);
        }
        let mut appended = HashMap::<&str, Vec<usize>>::new();
        let mut after_leaf = HashMap::<&str, Vec<usize>>::new();
        for (index, creation) in creations.iter().enumerate() {
            match &creation.placement {
                PreparedPageBlockCreationPlacement::Append(parent_id) => {
                    appended.entry(parent_id).or_default().push(index);
                }
                PreparedPageBlockCreationPlacement::AfterLeaf(block_id) => {
                    after_leaf.entry(block_id).or_default().push(index);
                }
            }
        }
        Self {
            current,
            creations,
            current_children,
            appended,
            after_leaf,
        }
    }

    fn build(self) -> Result<CardPage, String> {
        let expected_len = self.current.blocks.len() + self.creations.len();
        let mut blocks = Vec::with_capacity(expected_len);
        let mut seen = HashSet::with_capacity(expected_len);
        let mut pending = Vec::new();
        self.push_children(&self.current.block_id, &mut pending);
        while let Some(node) = pending.pop() {
            let block = self.block(node);
            if !seen.insert(block.block_id.as_str()) {
                return Err(format!(
                    "prepared creation projection repeats block {}",
                    block.block_id
                ));
            }
            blocks.push(block.clone());
            self.push_after_leaf(&block.block_id, &mut pending);
            self.push_children(&block.block_id, &mut pending);
        }
        if blocks.len() != expected_len {
            return Err(
                "prepared creation projection does not cover the page hierarchy".to_string(),
            );
        }
        Ok(CardPage {
            block_id: self.current.block_id.clone(),
            title: self.current.title.clone(),
            status: self.current.status.clone(),
            properties: self.current.properties.clone(),
            blocks,
            discussions: self.current.discussions.clone(),
            comments_writable: self.current.comments_writable,
            format: self.current.format,
        })
    }

    fn block(&self, node: ProjectedBlock) -> &CardPageBlock {
        match node {
            ProjectedBlock::Current(index) => &self.current.blocks[index],
            ProjectedBlock::Creation(index) => &self.creations[index].projected,
        }
    }

    fn push_children(&self, parent_id: &str, pending: &mut Vec<ProjectedBlock>) {
        if let Some(indices) = self.appended.get(parent_id) {
            pending.extend(indices.iter().rev().copied().map(ProjectedBlock::Creation));
        }
        if let Some(indices) = self.current_children.get(parent_id) {
            pending.extend(indices.iter().rev().copied().map(ProjectedBlock::Current));
        }
    }

    fn push_after_leaf(&self, block_id: &str, pending: &mut Vec<ProjectedBlock>) {
        if let Some(indices) = self.after_leaf.get(block_id) {
            pending.extend(indices.iter().copied().map(ProjectedBlock::Creation));
        }
    }
}
