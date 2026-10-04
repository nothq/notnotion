use super::{
    column_ratio::column_resize_between,
    created::PreparedPageBlockCreations,
    history::{prepare_deleted_page_blocks, PreparedPageBlockDeletion},
    snapshot::PageSnapshotIndex,
};
use crate::model::{
    CardPage, ColumnSafePageBlockMutationSequence, PageBlockPlacement, PageBlockStructuralMutation,
    ReorderPageBlockSubtreesRequest, ResizePageColumnsRequest,
};

pub(super) struct PreparedPageSnapshotTransition {
    pub(super) column_resize: Option<ResizePageColumnsRequest>,
    pub(super) creations: PreparedPageBlockCreations,
    pub(super) reorder_requests: Vec<ReorderPageBlockSubtreesRequest>,
    pub(super) deletions: Vec<PreparedPageBlockDeletion>,
}

impl PreparedPageSnapshotTransition {
    pub(super) fn new(
        current: &CardPage,
        target: &CardPage,
        index: &PageSnapshotIndex<'_>,
    ) -> Result<Self, String> {
        let creations = PreparedPageBlockCreations::new(index, target)?;
        let column_resize = column_resize_between(current, target, index)?;
        let pre_reorder_page = creations.projected_page(current)?;
        let reorder_index = PageSnapshotIndex::new(&pre_reorder_page, target);
        let reorder_requests = target_page_block_order_requests(target, &reorder_index);
        let deletions = prepare_deleted_page_blocks(current, index);
        let structural_mutations =
            reorder_requests
                .iter()
                .cloned()
                .map(PageBlockStructuralMutation::Reorder)
                .chain(deletions.iter().map(|deletion| {
                    PageBlockStructuralMutation::Delete(deletion.request().clone())
                }))
                .collect();
        ColumnSafePageBlockMutationSequence::new(&pre_reorder_page, structural_mutations)?;
        Ok(Self {
            column_resize,
            creations,
            reorder_requests,
            deletions,
        })
    }
}

fn target_page_block_order_requests(
    target: &CardPage,
    index: &PageSnapshotIndex<'_>,
) -> Vec<ReorderPageBlockSubtreesRequest> {
    let mut requests = Vec::new();
    let target_parent_ids = std::iter::once(target.block_id.as_str())
        .chain(target.blocks.iter().map(|block| block.block_id.as_str()));
    for parent_block_id in target_parent_ids {
        let target_order = index.target_children(parent_block_id);
        let retained_prefix = retained_target_prefix_len(index, parent_block_id, target_order);
        if retained_prefix == target_order.len() {
            continue;
        }
        requests.extend(target_order[retained_prefix..].iter().map(|block_id| {
            ReorderPageBlockSubtreesRequest {
                target_parent_block_id: parent_block_id.to_string(),
                block_ids: vec![(*block_id).to_string()],
                placement: PageBlockPlacement::Append,
            }
        }));
    }
    requests
}

fn retained_target_prefix_len(
    index: &PageSnapshotIndex<'_>,
    parent_id: &str,
    target_order: &[&str],
) -> usize {
    let mut retained = 0;
    for block_id in index.current_children(parent_id) {
        let remains_in_parent = index
            .target_block(block_id)
            .is_some_and(|block| block.parent_block_id == parent_id);
        if remains_in_parent && target_order.get(retained) == Some(block_id) {
            retained += 1;
        }
    }
    retained
}
