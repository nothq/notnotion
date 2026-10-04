use std::collections::{HashMap, HashSet};

use crate::model::{CardPage, CardPageBlock};

pub(super) fn merge_cached_page_child_orders(
    baseline: &CardPage,
    edited: &CardPage,
    live: &CardPage,
    blocks_by_id: &HashMap<String, CardPageBlock>,
) -> HashMap<String, Vec<String>> {
    let baseline_orders = cached_page_child_orders(baseline);
    let edited_orders = cached_page_child_orders(edited);
    let live_orders = cached_page_child_orders(live);
    let mut parent_ids = Vec::new();
    let mut seen_parents = HashSet::new();
    for source in edited.blocks.iter().chain(&live.blocks) {
        let Some(block) = blocks_by_id.get(&source.block_id) else {
            continue;
        };
        if seen_parents.insert(block.parent_block_id.clone()) {
            parent_ids.push(block.parent_block_id.clone());
        }
    }

    parent_ids
        .into_iter()
        .map(|parent_id| {
            let baseline_order = baseline_orders
                .get(parent_id.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            let edited_order = edited_orders
                .get(parent_id.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            let live_order = live_orders
                .get(parent_id.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            let (primary, secondary) = if baseline_order != edited_order {
                (edited_order, live_order)
            } else {
                (live_order, edited_order)
            };
            let order = merge_cached_sibling_order(&parent_id, primary, secondary, blocks_by_id);
            (parent_id, order)
        })
        .collect()
}

fn cached_page_child_orders(page: &CardPage) -> HashMap<&str, Vec<String>> {
    let mut orders = HashMap::new();
    for block in &page.blocks {
        orders
            .entry(block.parent_block_id.as_str())
            .or_insert_with(Vec::new)
            .push(block.block_id.clone());
    }
    orders
}

fn merge_cached_sibling_order(
    parent_id: &str,
    primary: &[String],
    secondary: &[String],
    blocks_by_id: &HashMap<String, CardPageBlock>,
) -> Vec<String> {
    let belongs_to_parent = |block_id: &str| {
        blocks_by_id
            .get(block_id)
            .is_some_and(|block| block.parent_block_id == parent_id)
    };
    let primary = primary
        .iter()
        .filter(|block_id| belongs_to_parent(block_id))
        .cloned()
        .collect::<Vec<_>>();
    let primary_positions = primary
        .iter()
        .enumerate()
        .map(|(index, block_id)| (block_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut buckets = vec![Vec::new(); primary.len() + 1];
    let mut pending = Vec::new();
    let mut previous_primary_position = None;
    for block_id in secondary {
        if !belongs_to_parent(block_id) {
            continue;
        }
        if let Some(position) = primary_positions.get(block_id.as_str()).copied() {
            buckets[position].append(&mut pending);
            previous_primary_position = Some(position);
        } else {
            pending.push(block_id.clone());
        }
    }
    let trailing_bucket = previous_primary_position.map_or(primary.len(), |position| position + 1);
    buckets[trailing_bucket].append(&mut pending);

    let mut merged = Vec::with_capacity(primary.len() + secondary.len());
    for (position, block_id) in primary.into_iter().enumerate() {
        merged.append(&mut buckets[position]);
        merged.push(block_id);
    }
    merged.append(
        buckets
            .last_mut()
            .expect("cached sibling order always has a trailing bucket"),
    );
    merged
}
