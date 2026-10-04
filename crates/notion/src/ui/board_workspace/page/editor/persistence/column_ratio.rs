use super::snapshot::PageSnapshotIndex;
use crate::model::{CardPage, CardPageLayoutBlock, ResizePageColumnsRequest};

pub(super) fn column_resize_between(
    current: &CardPage,
    target: &CardPage,
    index: &PageSnapshotIndex<'_>,
) -> Result<Option<ResizePageColumnsRequest>, String> {
    if current.block_id != target.block_id {
        return Err("column ratio snapshot transition belongs to different pages".to_string());
    }
    let changed = changed_column_weights(current, index);
    if changed.is_empty() {
        return Ok(None);
    }
    if changed_column_layouts_have_equal_geometry(current, target, &changed)? {
        return Ok(None);
    }
    let [left, right] = changed.as_slice() else {
        return Err("a column ratio snapshot transition must change one adjacent pair".to_string());
    };
    if left.parent_block_id != right.parent_block_id {
        return Err("changed column weights do not belong to the same column list".to_string());
    }
    let request = ResizePageColumnsRequest::from_materialized_destination(
        current,
        target,
        left.parent_block_id.clone(),
        left.block_id.clone(),
        right.block_id.clone(),
    )?;
    Ok(Some(request))
}

fn changed_column_layouts_have_equal_geometry(
    current: &CardPage,
    target: &CardPage,
    changed: &[&crate::model::CardPageBlock],
) -> Result<bool, String> {
    let mut column_list_ids = Vec::new();
    for block in changed {
        if !column_list_ids.contains(&block.parent_block_id.as_str()) {
            column_list_ids.push(block.parent_block_id.as_str());
        }
    }
    for column_list_id in column_list_ids {
        if !current.column_layout_effective_geometry_eq(target, column_list_id)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn changed_column_weights<'a>(
    current: &'a CardPage,
    index: &PageSnapshotIndex<'_>,
) -> Vec<&'a crate::model::CardPageBlock> {
    current
        .blocks
        .iter()
        .filter_map(|current_block| {
            let CardPageLayoutBlock::Column {
                ratio: current_weight,
            } = current_block.layout_content()?
            else {
                return None;
            };
            let target_block = index.target_block(&current_block.block_id)?;
            let CardPageLayoutBlock::Column {
                ratio: target_weight,
            } = target_block.layout_content()?
            else {
                return None;
            };
            (current_weight != target_weight).then_some(current_block)
        })
        .collect()
}
