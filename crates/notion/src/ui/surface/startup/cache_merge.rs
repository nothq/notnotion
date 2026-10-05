use std::collections::{HashMap, HashSet};

use self::{
    fields::{merge_cached_page_block, merge_cached_page_root_fields},
    hierarchy::{
        cached_page_resurrection_ids, flatten_merged_page_blocks, merge_structurally_edited_blocks,
    },
    order::merge_cached_page_child_orders,
};
use crate::model::{CardPage, CardPageBlock};

mod fields;
mod hierarchy;
mod order;

struct CachedPageBlocks<'a> {
    baseline: HashMap<&'a str, &'a CardPageBlock>,
    edited: HashMap<&'a str, &'a CardPageBlock>,
    live: HashMap<&'a str, &'a CardPageBlock>,
}

impl<'a> CachedPageBlocks<'a> {
    fn new(baseline: &'a CardPage, edited: &'a CardPage, live: &'a CardPage) -> Self {
        Self {
            baseline: page_blocks_by_id(baseline),
            edited: page_blocks_by_id(edited),
            live: page_blocks_by_id(live),
        }
    }
}

pub(super) fn merge_cached_page_edits(
    baseline: &CardPage,
    edited: &CardPage,
    live: &CardPage,
) -> Option<CardPage> {
    if baseline.block_id != edited.block_id || baseline.block_id != live.block_id {
        return None;
    }
    if !has_cached_page_edits(baseline, edited) {
        return None;
    }

    let indexed = CachedPageBlocks::new(baseline, edited, live);
    let resurrected_ids = cached_page_resurrection_ids(edited, &indexed);
    let structure_changed = cached_page_structure_changed(baseline, edited, &resurrected_ids);
    let mut merged = live.clone();
    merge_cached_page_root_fields(&mut merged, baseline, edited);
    if !structure_changed {
        merge_cached_page_blocks(&mut merged, &indexed);
        return Some(valid_cached_page_merge_or_fallback(
            merged, baseline, edited, live, &indexed,
        ));
    }

    let blocks_by_id = merge_structurally_edited_blocks(edited, live, &indexed, &resurrected_ids);
    let children_by_parent = merge_cached_page_child_orders(baseline, edited, live, &blocks_by_id);
    let Some(blocks) = flatten_merged_page_blocks(&live.block_id, blocks_by_id, children_by_parent)
    else {
        return Some(cached_page_merge_fallback(baseline, edited, live, &indexed));
    };
    merged.blocks = blocks;
    Some(valid_cached_page_merge_or_fallback(
        merged, baseline, edited, live, &indexed,
    ))
}

fn page_blocks_by_id(page: &CardPage) -> HashMap<&str, &CardPageBlock> {
    page.blocks
        .iter()
        .map(|block| (block.block_id.as_str(), block))
        .collect()
}

fn has_cached_page_edits(baseline: &CardPage, edited: &CardPage) -> bool {
    baseline.title != edited.title
        || baseline.status != edited.status
        || baseline.blocks != edited.blocks
}

fn cached_page_structure_changed(
    baseline: &CardPage,
    edited: &CardPage,
    resurrected_ids: &HashSet<String>,
) -> bool {
    baseline.blocks.len() != edited.blocks.len()
        || baseline
            .blocks
            .iter()
            .zip(&edited.blocks)
            .any(|(baseline, edited)| {
                baseline.block_id != edited.block_id
                    || baseline.parent_block_id != edited.parent_block_id
                    || baseline.depth != edited.depth
            })
        || !resurrected_ids.is_empty()
}

fn merge_cached_page_blocks(target: &mut CardPage, indexed: &CachedPageBlocks<'_>) {
    for block in &mut target.blocks {
        let (Some(baseline), Some(edited)) = (
            indexed.baseline.get(block.block_id.as_str()),
            indexed.edited.get(block.block_id.as_str()),
        ) else {
            continue;
        };
        *block = merge_cached_page_block(baseline, edited, block, false);
    }
}

fn valid_cached_page_merge_or_fallback(
    merged: CardPage,
    baseline: &CardPage,
    edited: &CardPage,
    live: &CardPage,
    indexed: &CachedPageBlocks<'_>,
) -> CardPage {
    if merged.validate_block_hierarchy().is_ok() {
        return merged;
    }
    cached_page_merge_fallback(baseline, edited, live, indexed)
}

fn cached_page_merge_fallback(
    baseline: &CardPage,
    edited: &CardPage,
    live: &CardPage,
    indexed: &CachedPageBlocks<'_>,
) -> CardPage {
    let mut fallback = live.clone();
    merge_cached_page_root_fields(&mut fallback, baseline, edited);
    merge_cached_page_blocks(&mut fallback, indexed);
    fallback
}

impl crate::ui::surface::PageDocuments {
    pub(super) fn merge_cached_page_visual(
        &self,
        baseline: &CardPage,
        edited: CardPage,
        live: Option<&CardPage>,
    ) -> Option<CardPage> {
        live.and_then(|live| merge_cached_page_edits(baseline, &edited, live))
            .or_else(|| self.standalone.is_none().then_some(edited))
    }
}
