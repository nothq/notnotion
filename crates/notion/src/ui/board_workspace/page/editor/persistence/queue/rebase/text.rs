use crate::model::{
    CardPage, CardPageEditableBlock, EditPageBlockTextRequest, PageTextAnnotation,
    PageTextAnnotationKind, PageTextEditTarget,
};

use super::super::super::super::rich_text::annotations::{
    apply_annotation_actions_to_range, apply_typing_annotations,
};
use super::text_offset::{rebase_text_offset, TextOffsetBias};
use super::{PageWriteProjector, PageWriteReplayError, ReplayPageSide};

struct AnnotationActions<'a> {
    removals: &'a [PageTextAnnotationKind],
    additions: &'a [PageTextAnnotation],
}

pub(super) fn apply_rich_text_request(
    page: &mut CardPage,
    request: &EditPageBlockTextRequest,
) -> Result<(), PageWriteReplayError> {
    if request.page_block_id != page.block_id {
        return Err(PageWriteReplayError::RequestPageIdentity {
            expected_id: page.block_id.clone(),
            request_id: request.page_block_id.clone(),
        });
    }
    let actions = AnnotationActions {
        removals: request.annotation_removals(),
        additions: request.annotation_additions(),
    };
    for target in request.targets() {
        match target {
            PageTextEditTarget::Typing {
                block_id,
                offset_utf8,
                text,
            } => apply_typing(page, block_id, *offset_utf8, text, &actions)?,
            PageTextEditTarget::Selection {
                block_id,
                start_utf8,
                end_utf8,
            } => apply_selection(page, block_id, *start_utf8..*end_utf8, &actions)?,
        }
    }
    Ok(())
}

impl PageWriteProjector {
    pub(super) fn rebase_rich_text_request(
        &mut self,
        request: &mut EditPageBlockTextRequest,
    ) -> Result<(), PageWriteReplayError> {
        if request.page_block_id != self.local.block_id {
            return Err(PageWriteReplayError::RequestPageIdentity {
                expected_id: self.local.block_id.clone(),
                request_id: request.page_block_id.clone(),
            });
        }
        let actions = AnnotationActions {
            removals: request.annotation_removals(),
            additions: request.annotation_additions(),
        };
        let mut targets = Vec::with_capacity(request.targets().len());
        for target in request.targets() {
            targets.push(self.rebase_rich_text_target(target, &actions)?);
        }
        *request = EditPageBlockTextRequest::new(
            request.page_block_id.clone(),
            targets,
            actions.removals.to_vec(),
            actions.additions.to_vec(),
        )
        .map_err(|detail| PageWriteReplayError::invalid_request("rich text", detail))?;
        Ok(())
    }

    fn rebase_rich_text_target(
        &mut self,
        target: &PageTextEditTarget,
        actions: &AnnotationActions<'_>,
    ) -> Result<PageTextEditTarget, PageWriteReplayError> {
        match target {
            PageTextEditTarget::Typing {
                block_id,
                offset_utf8,
                text,
            } => self.rebase_typing(block_id, *offset_utf8, text, actions),
            PageTextEditTarget::Selection {
                block_id,
                start_utf8,
                end_utf8,
            } => self.rebase_annotation_selection(block_id, *start_utf8, *end_utf8, actions),
        }
    }

    fn rebase_typing(
        &mut self,
        block_id: &str,
        offset: usize,
        inserted: &str,
        actions: &AnnotationActions<'_>,
    ) -> Result<PageTextEditTarget, PageWriteReplayError> {
        let local_text = target_text(
            &self.local,
            block_id,
            "rich-text typing",
            ReplayPageSide::QueueBaseline,
        )?;
        let authority_text = target_text(
            &self.authority,
            block_id,
            "rich-text typing",
            ReplayPageSide::Authority,
        )?;
        let authority_offset = rebase_text_offset(
            block_id,
            local_text,
            authority_text,
            offset,
            TextOffsetBias::After,
        )?;
        apply_typing(&mut self.local, block_id, offset, inserted, actions)?;
        apply_typing(
            &mut self.authority,
            block_id,
            authority_offset,
            inserted,
            actions,
        )?;
        Ok(PageTextEditTarget::Typing {
            block_id: block_id.to_string(),
            offset_utf8: authority_offset,
            text: inserted.to_string(),
        })
    }

    fn rebase_annotation_selection(
        &mut self,
        block_id: &str,
        start: usize,
        end: usize,
        actions: &AnnotationActions<'_>,
    ) -> Result<PageTextEditTarget, PageWriteReplayError> {
        let local_text = target_text(
            &self.local,
            block_id,
            "rich-text selection",
            ReplayPageSide::QueueBaseline,
        )?;
        let authority_text = target_text(
            &self.authority,
            block_id,
            "rich-text selection",
            ReplayPageSide::Authority,
        )?;
        let authority_start = rebase_text_offset(
            block_id,
            local_text,
            authority_text,
            start,
            TextOffsetBias::After,
        )?;
        let authority_end = rebase_text_offset(
            block_id,
            local_text,
            authority_text,
            end,
            TextOffsetBias::Before,
        )?;
        let authority_range =
            authority_start.min(authority_end)..authority_start.max(authority_end);
        apply_selection(&mut self.local, block_id, start..end, actions)?;
        apply_selection(
            &mut self.authority,
            block_id,
            authority_range.clone(),
            actions,
        )?;
        Ok(PageTextEditTarget::Selection {
            block_id: block_id.to_string(),
            start_utf8: authority_range.start,
            end_utf8: authority_range.end,
        })
    }
}

pub(super) fn target_text<'a>(
    page: &'a CardPage,
    block_id: &str,
    operation: &'static str,
    side: ReplayPageSide,
) -> Result<&'a str, PageWriteReplayError> {
    if block_id == page.block_id {
        return Ok(&page.title);
    }
    let block = page
        .blocks
        .iter()
        .find(|block| block.block_id == block_id)
        .ok_or_else(|| PageWriteReplayError::MissingBlock {
            operation,
            block_id: block_id.to_string(),
            side,
        })?;
    block
        .editable_content()
        .map(|editable| editable.text.as_str())
        .ok_or_else(|| PageWriteReplayError::InvalidTextTarget {
            operation,
            block_id: block_id.to_string(),
            side,
        })
}

fn apply_typing(
    page: &mut CardPage,
    block_id: &str,
    offset: usize,
    inserted: &str,
    actions: &AnnotationActions<'_>,
) -> Result<(), PageWriteReplayError> {
    if block_id == page.block_id {
        insert_text(block_id, &mut page.title, offset, inserted)?;
        return Ok(());
    }
    let editable = target_editable_mut(page, block_id, "rich-text typing")?;
    validate_utf8_offset(block_id, &editable.text, offset)?;
    apply_typing_annotations(
        editable,
        offset,
        inserted.len(),
        actions.removals,
        actions.additions,
    );
    editable.text.insert_str(offset, inserted);
    Ok(())
}

fn apply_selection(
    page: &mut CardPage,
    block_id: &str,
    range: std::ops::Range<usize>,
    actions: &AnnotationActions<'_>,
) -> Result<(), PageWriteReplayError> {
    if block_id == page.block_id {
        validate_utf8_range(block_id, &page.title, &range)?;
        return Ok(());
    }
    let editable = target_editable_mut(page, block_id, "rich-text selection")?;
    validate_utf8_range(block_id, &editable.text, &range)?;
    apply_annotation_actions_to_range(editable, range, actions.removals, actions.additions);
    Ok(())
}

fn target_editable_mut<'a>(
    page: &'a mut CardPage,
    block_id: &str,
    operation: &'static str,
) -> Result<&'a mut CardPageEditableBlock, PageWriteReplayError> {
    let block = page
        .blocks
        .iter_mut()
        .find(|block| block.block_id == block_id)
        .ok_or_else(|| PageWriteReplayError::MissingBlock {
            operation,
            block_id: block_id.to_string(),
            side: ReplayPageSide::Authority,
        })?;
    block
        .editable_content_mut()
        .ok_or_else(|| PageWriteReplayError::InvalidTextTarget {
            operation,
            block_id: block_id.to_string(),
            side: ReplayPageSide::Authority,
        })
}

fn insert_text(
    block_id: &str,
    target: &mut String,
    offset: usize,
    text: &str,
) -> Result<(), PageWriteReplayError> {
    validate_utf8_offset(block_id, target, offset)?;
    target.insert_str(offset, text);
    Ok(())
}

fn validate_utf8_offset(
    block_id: &str,
    text: &str,
    offset: usize,
) -> Result<(), PageWriteReplayError> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return Err(PageWriteReplayError::InvalidTextOffset {
            block_id: block_id.to_string(),
            offset,
            text_len: text.len(),
            encoding: "UTF-8",
        });
    }
    Ok(())
}

fn validate_utf8_range(
    block_id: &str,
    text: &str,
    range: &std::ops::Range<usize>,
) -> Result<(), PageWriteReplayError> {
    validate_utf8_offset(block_id, text, range.start)?;
    validate_utf8_offset(block_id, text, range.end)?;
    if range.start >= range.end {
        return Err(PageWriteReplayError::invalid_request(
            "rich-text selection",
            "selection became empty or reversed",
        ));
    }
    Ok(())
}
