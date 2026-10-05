use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
};

use crate::ui::{CardPageLayoutBlock, LoadedCardPageData};

use super::preview::{
    page_block_drag_preview_rows, page_block_drag_root_ids, page_block_drag_selected_ids,
    page_block_subtree_ids,
};
use super::{PageBlockDragSource, ResolvedPageBlockDragPayload};

#[derive(Clone)]
pub(super) struct PageBlockDragSourceCountProof {
    allocation: Weak<LoadedCardPageData>,
    columns: Arc<[PageBlockDragColumnSourceCount]>,
}

#[derive(Clone)]
struct PageBlockDragColumnSourceCount {
    column_block_id: Arc<str>,
    direct_child_count: usize,
    moved_root_count: usize,
}

impl PageBlockDragSourceCountProof {
    fn new(data: &Arc<LoadedCardPageData>, root_block_ids: &[String]) -> Self {
        let mut moved_by_column = HashMap::<Arc<str>, usize>::new();
        for root_id in root_block_ids {
            let root = data
                .page_block(root_id)
                .expect("resolved page drag root must remain indexed");
            let Some(parent) = data.page_block(&root.parent_block_id) else {
                continue;
            };
            if matches!(
                parent.layout_content(),
                Some(CardPageLayoutBlock::Column { .. })
            ) {
                *moved_by_column
                    .entry(Arc::from(parent.block_id.as_str()))
                    .or_default() += 1;
            }
        }
        let mut child_counts = moved_by_column
            .keys()
            .cloned()
            .map(|column_id| (column_id, 0))
            .collect::<HashMap<_, usize>>();
        for block in &data.page.blocks {
            if let Some(count) = child_counts.get_mut(block.parent_block_id.as_str()) {
                *count += 1;
            }
        }
        let mut columns = moved_by_column
            .into_iter()
            .map(
                |(column_block_id, moved_root_count)| PageBlockDragColumnSourceCount {
                    direct_child_count: child_counts[&column_block_id],
                    column_block_id,
                    moved_root_count,
                },
            )
            .collect::<Vec<_>>();
        columns.sort_by(|left, right| left.column_block_id.cmp(&right.column_block_id));
        Self {
            allocation: Arc::downgrade(data),
            columns: columns.into(),
        }
    }

    pub(super) fn matches(&self, data: &Arc<LoadedCardPageData>) -> bool {
        std::ptr::eq(self.allocation.as_ptr(), Arc::as_ptr(data))
    }

    pub(super) fn allows_target(
        &self,
        data: &Arc<LoadedCardPageData>,
        target_parent_id: &str,
    ) -> bool {
        self.matches(data)
            && self.columns.iter().all(|source| {
                source.column_block_id.as_ref() == target_parent_id
                    || source.moved_root_count < source.direct_child_count
            })
    }
}

pub(super) fn resolve_page_block_drag_payload(
    source: &PageBlockDragSource,
) -> ResolvedPageBlockDragPayload {
    let data = &source.data;
    let mut selected_block_ids = page_block_drag_selected_ids(
        data,
        &source.selection.block_selection,
        source.selection.text_selection.as_ref(),
        &source.dragged_block_id,
    );
    let dragged_column = data
        .visible_rows
        .iter()
        .find(|row| row.block_index == source.dragged_index)
        .and_then(|row| row.column_block_index);
    let dragged_column_block_ids = data
        .visible_rows
        .iter()
        .filter(|row| row.column_block_index == dragged_column)
        .map(|row| data.page.blocks[row.block_index].block_id.as_str())
        .collect::<HashSet<_>>();
    selected_block_ids
        .retain(|selected_id| dragged_column_block_ids.contains(selected_id.as_str()));
    let root_block_ids = page_block_drag_root_ids(
        &data.page.blocks,
        &selected_block_ids,
        &source.dragged_block_id,
    );
    let subtree_block_ids = page_block_subtree_ids(&data.page.blocks, &root_block_ids)
        .expect("page block drag roots must resolve to page subtrees");
    let source_count_proof = PageBlockDragSourceCountProof::new(data, &root_block_ids);
    let subtree_membership = subtree_block_ids.iter().cloned().collect();
    let (preview_rows, preview_anchor_index) = page_block_drag_preview_rows(
        data,
        source.dragged_index,
        &root_block_ids,
        &subtree_block_ids,
        source.block_images.as_ref(),
    );
    ResolvedPageBlockDragPayload {
        root_block_ids,
        subtree_membership,
        source_count_proof,
        preview_rows,
        preview_anchor_index,
    }
}
