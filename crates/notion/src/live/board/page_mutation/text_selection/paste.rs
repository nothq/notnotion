use super::super::{
    blocks::{paste_text_block_operations, MutationIdentity, PasteTextBlockInput},
    text::text_offset_utf8_to_utf16,
    BuiltPageMutation, CrdtClock,
};
use super::planner::ordered_text_blocks;
use crate::live::board::page_state::PageMutationState;
use crate::model::{PageMutationEffect, PastePageTextSelectionRequest};

pub(in crate::live::board::page_mutation) fn build_multiline_text_selection_paste(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &PastePageTextSelectionRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let ordered = ordered_text_blocks(state)?;
    let start_index = ordered
        .iter()
        .position(|block| block.id == request.start().block_id)
        .ok_or_else(|| {
            format!(
                "paste selection start block {} is not editable",
                request.start().block_id
            )
        })?;
    let end_index = ordered
        .iter()
        .position(|block| block.id == request.end().block_id)
        .ok_or_else(|| {
            format!(
                "paste selection end block {} is not editable",
                request.end().block_id
            )
        })?;
    if start_index > end_index {
        return Err("multiline paste endpoints are not in document order".to_string());
    }
    let start = ordered[start_index];
    let end = ordered[end_index];
    text_offset_utf8_to_utf16(start.title()?, request.start().offset_utf8)?;
    text_offset_utf8_to_utf16(end.title()?, request.end().offset_utf8)?;

    Ok(BuiltPageMutation {
        operations: paste_text_block_operations(
            state,
            identity,
            PasteTextBlockInput {
                block_id: request.new_block_id(),
                after_block_id: &end.id,
                text: request.text(),
            },
            clock,
        )?,
        effect: PageMutationEffect::BlockCreated {
            block_id: request.new_block_id().to_string(),
        },
        user_action: "paste",
    })
}
