use super::{merge_required_provenance, PageTraversalState};
use crate::live::board::page_chunk::opaque::{
    insert_proven_opaque_block, ProvenOpaqueUnavailableBlocks,
};
use crate::live::board::page_chunk::HydratedPageBlockScope;
use crate::live::NotionLiveError;

pub(super) fn consume_proven_opaque_block(
    pending_block: &super::pending::PendingPageBlock,
    has_record_value: bool,
    traversal: &mut PageTraversalState,
) -> Result<bool, String> {
    if has_record_value {
        traversal
            .opaque_unavailable_blocks
            .remove(&pending_block.block_id);
        traversal
            .reachable_opaque_block_ids
            .remove(&pending_block.block_id);
        return Ok(false);
    }
    let Some(proof) = traversal
        .opaque_unavailable_blocks
        .get(&pending_block.block_id)
    else {
        return Ok(false);
    };
    if pending_block.is_reference_target() {
        traversal.loaded.insert(pending_block.block_id.clone());
        traversal
            .unavailable_reference_block_ids
            .insert(pending_block.block_id.clone());
        return Ok(true);
    }
    let parent_block_id = pending_block.content_parent_id().ok_or_else(|| {
        format!(
            "opaque Notion block {} was reached without content-edge provenance",
            pending_block.block_id
        )
    })?;
    if !proof.validates_content_edge(
        &pending_block.block_id,
        parent_block_id,
        &pending_block.space_id,
    ) {
        return Ok(false);
    }
    merge_required_provenance(pending_block, traversal)?;
    traversal.loaded.insert(pending_block.block_id.clone());
    traversal
        .reachable_opaque_block_ids
        .insert(pending_block.block_id.clone());
    Ok(true)
}

pub(super) fn merge_opaque_proofs(
    traversal: &mut PageTraversalState,
    proofs: ProvenOpaqueUnavailableBlocks,
) -> Result<(), String> {
    for proof in proofs.into_values() {
        let block_id = proof.block_id().to_string();
        if traversal.reachable_opaque_block_ids.contains(&block_id) {
            insert_proven_opaque_block(&mut traversal.opaque_unavailable_blocks, proof)?;
        } else {
            traversal.opaque_unavailable_blocks.insert(block_id, proof);
        }
    }
    Ok(())
}

pub(super) fn restart_page_traversal(
    traversal: &mut PageTraversalState,
) -> Result<(), NotionLiveError> {
    let root = traversal
        .required_by_id
        .values()
        .find(|pending| pending.is_root())
        .cloned()
        .ok_or_else(|| {
            NotionLiveError::Fatal(
                "Notion page traversal lost its root provenance during parent refresh".to_string(),
            )
        })?;
    traversal.loaded.clear();
    traversal.expanded.clear();
    traversal.required_by_id.clear();
    traversal.unavailable_reference_block_ids.clear();
    traversal.reachable_opaque_block_ids.clear();
    traversal.collection_ids_by_space.clear();
    traversal.linked_view_ids_by_space.clear();
    traversal.user_ids.clear();
    traversal.pending.clear();
    traversal.pending.push(root);
    Ok(())
}

pub(super) fn page_block_scope(
    page_id: &str,
    space_id: String,
    traversal: PageTraversalState,
) -> HydratedPageBlockScope {
    let mut opaque_unavailable_blocks = traversal.opaque_unavailable_blocks;
    opaque_unavailable_blocks
        .retain(|block_id, _| traversal.reachable_opaque_block_ids.contains(block_id));
    let mut unavailable_reference_block_ids = traversal.unavailable_reference_block_ids;
    unavailable_reference_block_ids.extend(opaque_unavailable_blocks.keys().cloned());
    HydratedPageBlockScope {
        root_block_id: page_id.to_string(),
        space_id,
        block_ids: traversal.expanded,
        unavailable_reference_block_ids,
        opaque_unavailable_blocks,
        collection_ids_by_space: traversal.collection_ids_by_space,
        linked_view_ids_by_space: traversal.linked_view_ids_by_space,
        user_ids: traversal.user_ids,
    }
}
