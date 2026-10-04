use crate::model::{CardPage, InsertPageMentionRequest, UpdatePageMentionRequest};

use super::super::super::super::rich_text::annotations::{
    apply_mention_insert_to_editable, apply_mention_update_to_editable,
};
use super::text_offset::{rebase_text_offset, TextOffsetBias};
use super::{PageWriteProjector, PageWriteReplayError};

const INSERT_LABEL: &str = "insert mention";
const UPDATE_LABEL: &str = "update mention";

impl PageWriteProjector {
    /// Move a queued mention insertion onto the authority text by rebasing the
    /// replaced range through the pending local edits.
    pub(super) fn rebase_insert_mention_request(
        &self,
        request: &mut InsertPageMentionRequest,
    ) -> Result<(), PageWriteReplayError> {
        let local_text = editable_text(&self.local, request.block_id(), INSERT_LABEL)?;
        let authority_text = editable_text(&self.authority, request.block_id(), INSERT_LABEL)?;
        let start = rebase_text_offset(
            request.block_id(),
            local_text,
            authority_text,
            request.start_utf8(),
            TextOffsetBias::Before,
        )?;
        let end = rebase_text_offset(
            request.block_id(),
            local_text,
            authority_text,
            request.end_utf8(),
            TextOffsetBias::After,
        )?;
        request
            .set_range(start, end)
            .map_err(|detail| PageWriteReplayError::invalid_request(INSERT_LABEL, detail))
    }

    pub(super) fn rebase_update_mention_request(
        &self,
        request: &mut UpdatePageMentionRequest,
    ) -> Result<(), PageWriteReplayError> {
        let local_text = editable_text(&self.local, request.block_id(), UPDATE_LABEL)?;
        let authority_text = editable_text(&self.authority, request.block_id(), UPDATE_LABEL)?;
        let offset = rebase_text_offset(
            request.block_id(),
            local_text,
            authority_text,
            request.offset_utf8(),
            TextOffsetBias::Before,
        )?;
        request.set_offset(offset);
        Ok(())
    }
}

pub(super) fn insert_mention(
    page: &mut CardPage,
    request: &InsertPageMentionRequest,
) -> Result<(), PageWriteReplayError> {
    let editable = editable_mut(page, request.block_id(), INSERT_LABEL)?;
    let end = request.end_utf8();
    if end > editable.text.len()
        || !editable.text.is_char_boundary(request.start_utf8())
        || !editable.text.is_char_boundary(end)
    {
        return Err(PageWriteReplayError::invalid_request(
            INSERT_LABEL,
            format!(
                "range {}..{end} does not fit block {}",
                request.start_utf8(),
                request.block_id()
            ),
        ));
    }
    apply_mention_insert_to_editable(editable, request);
    Ok(())
}

pub(super) fn update_mention(
    page: &mut CardPage,
    request: &UpdatePageMentionRequest,
) -> Result<(), PageWriteReplayError> {
    let editable = editable_mut(page, request.block_id(), UPDATE_LABEL)?;
    apply_mention_update_to_editable(editable, request)
        .map_err(|detail| PageWriteReplayError::invalid_request(UPDATE_LABEL, detail))
}

fn editable_text<'a>(
    page: &'a CardPage,
    block_id: &str,
    label: &'static str,
) -> Result<&'a str, PageWriteReplayError> {
    page.blocks
        .iter()
        .find(|block| block.block_id == block_id)
        .and_then(|block| block.editable_content())
        .map(|editable| editable.text.as_str())
        .ok_or_else(|| {
            PageWriteReplayError::invalid_request(
                label,
                format!("block {block_id} is not an editable block of the page"),
            )
        })
}

fn editable_mut<'a>(
    page: &'a mut CardPage,
    block_id: &str,
    label: &'static str,
) -> Result<&'a mut crate::model::CardPageEditableBlock, PageWriteReplayError> {
    page.blocks
        .iter_mut()
        .find(|block| block.block_id == block_id)
        .and_then(|block| block.editable_content_mut())
        .ok_or_else(|| {
            PageWriteReplayError::invalid_request(
                label,
                format!("block {block_id} is not an editable block of the page"),
            )
        })
}
