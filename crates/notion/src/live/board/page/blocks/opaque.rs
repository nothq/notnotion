use crate::model::CardPageBlock;

use super::super::super::{record_is_deleted, record_value_state, RecordValueState};
use super::super::{loaded_record_value, PageBlockParseContext};

pub(super) fn parsed_opaque_unavailable_block(
    context: &PageBlockParseContext<'_>,
    parent_block_id: &str,
    block_id: &str,
    depth: usize,
) -> Result<Option<CardPageBlock>, String> {
    if context.allow_partial_preview {
        return Ok(match record_value_state(context.blocks.get(block_id)) {
            RecordValueState::Present(block) if !record_is_deleted(block) => None,
            RecordValueState::Incomplete
            | RecordValueState::Unavailable
            | RecordValueState::Present(_) => Some(CardPageBlock::opaque_unavailable(
                block_id,
                parent_block_id,
                depth,
            )),
        });
    }
    if context
        .blocks
        .get(block_id)
        .and_then(loaded_record_value)
        .is_some()
    {
        return Ok(None);
    }
    let Some(proof) = context.opaque_unavailable_blocks.get(block_id) else {
        return Ok(None);
    };
    if !proof.validates_content_edge(block_id, parent_block_id, context.space_id) {
        return Err(format!(
            "opaque Notion block {block_id} proof does not match parsed parent {parent_block_id} in space {}",
            context.space_id
        ));
    }
    Ok(Some(CardPageBlock::opaque_unavailable(
        block_id,
        parent_block_id,
        depth,
    )))
}
