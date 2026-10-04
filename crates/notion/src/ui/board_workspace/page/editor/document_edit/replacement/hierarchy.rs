use std::collections::{HashMap, HashSet};

use crate::ui::{CardPage, CardPageBlock};

#[derive(Clone)]
pub(in super::super) struct ReparentedRoot {
    pub(in super::super) block_id: String,
    pub(in super::super) target_parent_block_id: String,
}

pub(in super::super) struct SelectionRemovalPlan {
    pub(in super::super) removed_ids: Vec<String>,
    pub(in super::super) removed: HashSet<String>,
    pub(in super::super) reparented: Vec<ReparentedRoot>,
}

impl SelectionRemovalPlan {
    pub(in super::super) fn new(
        page: &CardPage,
        selected_indices: &[usize],
        collapsed_hidden_owner_indices: &[Option<usize>],
    ) -> Result<Self, String> {
        if selected_indices.len() < 2 {
            return Err("a cross-block text selection must contain two blocks".to_string());
        }
        if selected_indices
            .windows(2)
            .any(|indices| indices[0] >= indices[1])
        {
            return Err("selected page blocks must be in document order".to_string());
        }
        if selected_indices
            .last()
            .is_some_and(|index| *index >= page.blocks.len())
            || collapsed_hidden_owner_indices.len() != page.blocks.len()
            || collapsed_hidden_owner_indices
                .iter()
                .flatten()
                .any(|index| *index >= page.blocks.len())
        {
            return Err(
                "page selection projection does not match its canonical blocks".to_string(),
            );
        }
        let mut selected_removal_roots = vec![false; page.blocks.len()];
        for index in &selected_indices[1..] {
            selected_removal_roots[*index] = true;
        }
        let removed_ids = page
            .blocks
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                selected_removal_roots[*index]
                    || collapsed_hidden_owner_indices[*index]
                        .is_some_and(|owner_index| selected_removal_roots[owner_index])
            })
            .map(|(_, block)| block.block_id.clone())
            .collect::<Vec<_>>();
        let removed = removed_ids.iter().cloned().collect::<HashSet<_>>();
        let blocks_by_id = page
            .blocks
            .iter()
            .map(|block| (block.block_id.as_str(), block))
            .collect::<HashMap<_, _>>();
        let reparented = reparented_surviving_roots(page, &blocks_by_id, &removed)?;
        Ok(Self {
            removed_ids,
            removed,
            reparented,
        })
    }

    pub(in super::super) fn apply_to(&self, page: &mut CardPage) -> Result<(), String> {
        for moved in &self.reparented {
            let block = page
                .blocks
                .iter_mut()
                .find(|block| block.block_id == moved.block_id)
                .ok_or_else(|| format!("retained block {} is missing", moved.block_id))?;
            block
                .parent_block_id
                .clone_from(&moved.target_parent_block_id);
        }
        page.blocks
            .retain(|block| !self.removed.contains(&block.block_id));
        recompute_page_block_depths(page)
    }
}

fn reparented_surviving_roots(
    page: &CardPage,
    blocks_by_id: &HashMap<&str, &CardPageBlock>,
    removed: &HashSet<String>,
) -> Result<Vec<ReparentedRoot>, String> {
    page.blocks
        .iter()
        .filter(|block| {
            !removed.contains(&block.block_id) && removed.contains(&block.parent_block_id)
        })
        .map(|block| {
            Ok(ReparentedRoot {
                block_id: block.block_id.clone(),
                target_parent_block_id: nearest_surviving_parent(
                    &page.block_id,
                    &block.parent_block_id,
                    blocks_by_id,
                    removed,
                )?,
            })
        })
        .collect()
}

fn nearest_surviving_parent(
    page_id: &str,
    initial_parent_id: &str,
    blocks_by_id: &HashMap<&str, &CardPageBlock>,
    removed: &HashSet<String>,
) -> Result<String, String> {
    let mut parent_id = initial_parent_id;
    let mut visited = HashSet::new();
    while removed.contains(parent_id) {
        if !visited.insert(parent_id) {
            return Err(format!(
                "page block hierarchy contains a cycle at {parent_id}"
            ));
        }
        parent_id = blocks_by_id
            .get(parent_id)
            .ok_or_else(|| format!("removed parent block {parent_id} is missing"))?
            .parent_block_id
            .as_str();
    }
    if parent_id != page_id && !blocks_by_id.contains_key(parent_id) {
        return Err(format!("surviving parent block {parent_id} is missing"));
    }
    Ok(parent_id.to_string())
}

pub(in super::super) fn recompute_page_block_depths(page: &mut CardPage) -> Result<(), String> {
    let mut depths = HashMap::new();
    for block in &mut page.blocks {
        block.depth = if block.parent_block_id == page.block_id {
            0
        } else {
            depths.get(&block.parent_block_id).copied().ok_or_else(|| {
                format!(
                    "parent block {} must precede retained block {}",
                    block.parent_block_id, block.block_id
                )
            })? + 1
        };
        depths.insert(block.block_id.clone(), block.depth);
    }
    Ok(())
}
