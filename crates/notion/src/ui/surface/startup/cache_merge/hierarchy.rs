use std::collections::{HashMap, HashSet};

use super::{fields::merge_cached_page_block, CachedPageBlocks};
use crate::model::{CardPage, CardPageBlock};

pub(super) fn merge_structurally_edited_blocks(
    edited: &CardPage,
    live: &CardPage,
    indexed: &CachedPageBlocks<'_>,
    resurrected_ids: &HashSet<String>,
) -> HashMap<String, CardPageBlock> {
    let mut blocks_by_id = HashMap::with_capacity(edited.blocks.len() + live.blocks.len());
    let mut included_ids = HashSet::with_capacity(edited.blocks.len() + live.blocks.len());
    for edited_block in &edited.blocks {
        let block = match (
            indexed.baseline.get(edited_block.block_id.as_str()),
            indexed.live.get(edited_block.block_id.as_str()),
        ) {
            (Some(baseline), Some(live)) => merge_cached_page_block(
                baseline,
                edited_block,
                live,
                baseline.parent_block_id != edited_block.parent_block_id
                    || baseline.depth != edited_block.depth,
            ),
            (None, _) => edited_block.clone(),
            (Some(_), None) if resurrected_ids.contains(&edited_block.block_id) => {
                edited_block.clone()
            }
            (Some(_), None) => continue,
        };
        included_ids.insert(edited_block.block_id.clone());
        blocks_by_id.insert(edited_block.block_id.clone(), block);
    }

    for live_block in &live.blocks {
        if included_ids.contains(&live_block.block_id)
            || indexed.baseline.contains_key(live_block.block_id.as_str())
        {
            continue;
        }
        if live_block.parent_block_id != live.block_id
            && !included_ids.contains(&live_block.parent_block_id)
        {
            continue;
        }
        included_ids.insert(live_block.block_id.clone());
        blocks_by_id.insert(live_block.block_id.clone(), live_block.clone());
    }
    blocks_by_id
}

pub(super) fn cached_page_resurrection_ids(
    edited: &CardPage,
    indexed: &CachedPageBlocks<'_>,
) -> HashSet<String> {
    let mut roots = HashSet::new();
    for block in &edited.blocks {
        let locally_changed = indexed
            .baseline
            .get(block.block_id.as_str())
            .is_none_or(|baseline| *baseline != block);
        if indexed.live.contains_key(block.block_id.as_str()) || !locally_changed {
            continue;
        }
        let mut root = block;
        while root.parent_block_id != edited.block_id
            && !indexed.live.contains_key(root.parent_block_id.as_str())
        {
            let Some(parent) = indexed.edited.get(root.parent_block_id.as_str()) else {
                break;
            };
            root = parent;
        }
        roots.insert(root.block_id.clone());
    }
    if roots.is_empty() {
        return roots;
    }

    let mut resurrected = roots.clone();
    for block in &edited.blocks {
        let mut current = block;
        while current.parent_block_id != edited.block_id {
            if roots.contains(&current.parent_block_id) {
                resurrected.insert(block.block_id.clone());
                break;
            }
            let Some(parent) = indexed.edited.get(current.parent_block_id.as_str()) else {
                break;
            };
            current = parent;
        }
    }
    resurrected
}

pub(super) fn flatten_merged_page_blocks(
    page_id: &str,
    mut blocks_by_id: HashMap<String, CardPageBlock>,
    mut children_by_parent: HashMap<String, Vec<String>>,
) -> Option<Vec<CardPageBlock>> {
    let expected_len = blocks_by_id.len();
    let mut stack = children_by_parent
        .remove(page_id)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .map(|block_id| (block_id, page_id.to_string(), 0usize))
        .collect::<Vec<_>>();
    let mut blocks = Vec::with_capacity(expected_len);
    while let Some((block_id, parent_block_id, depth)) = stack.pop() {
        let mut block = blocks_by_id.remove(&block_id)?;
        block.parent_block_id = parent_block_id;
        block.depth = depth;
        if let Some(children) = children_by_parent.remove(&block_id) {
            let child_depth = depth.checked_add(1)?;
            stack.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child_id| (child_id, block_id.clone(), child_depth)),
            );
        }
        blocks.push(block);
    }
    (blocks.len() == expected_len && blocks_by_id.is_empty()).then_some(blocks)
}
