use crate::model::{
    CardPageEditableBlock, CardPageTextAnnotationSpan, InsertPageMentionRequest, PageMention,
    PageTextAnnotation, UpdatePageMentionRequest, PAGE_MENTION_TOKEN, PAGE_MENTION_TOKEN_STR,
};

use super::transform::replace_annotated_text_range;

/// Apply a mention insertion to the local editable block: the trigger text is
/// replaced by the token (plus Notion's trailing space) and the token gets its
/// mention span.
pub(crate) fn apply_mention_insert_to_editable(
    editable: &mut CardPageEditableBlock,
    request: &InsertPageMentionRequest,
) {
    let start = request.start_utf8();
    replace_annotated_text_range(
        editable,
        start..request.end_utf8(),
        &request.inserted_text(),
    );
    editable.annotations.push(
        CardPageTextAnnotationSpan::new(
            &editable.text,
            start,
            start + PAGE_MENTION_TOKEN_STR.len(),
            PageTextAnnotation::Mention(request.mention().clone()),
        )
        .expect("inserted mention token must fit the edited text"),
    );
    editable
        .annotations
        .sort_by_key(|span| (span.start_utf8, span.end_utf8));
}

/// Replace the mention of the token that starts at the request offset.
pub(crate) fn apply_mention_update_to_editable(
    editable: &mut CardPageEditableBlock,
    request: &UpdatePageMentionRequest,
) -> Result<(), String> {
    let offset = request.offset_utf8();
    if editable
        .text
        .get(offset..)
        .and_then(|rest| rest.chars().next())
        != Some(PAGE_MENTION_TOKEN)
    {
        return Err(format!("block text offset {offset} is not a mention token"));
    }
    let span = editable
        .annotations
        .iter_mut()
        .find(|span| span.start_utf8 == offset && span.annotation.mention().is_some())
        .ok_or_else(|| format!("mention token at offset {offset} has no mention span"))?;
    span.annotation = PageTextAnnotation::Mention(request.mention().clone());
    Ok(())
}

/// The mention whose token starts at `offset`, if any.
pub(crate) fn mention_at_offset(
    editable: &CardPageEditableBlock,
    offset: usize,
) -> Option<&PageMention> {
    editable
        .annotations
        .iter()
        .find(|span| span.start_utf8 == offset)
        .and_then(|span| span.annotation.mention())
}

/// Every mention span in document order as `(token start offset, mention)`.
pub(crate) fn mention_spans(
    editable: &CardPageEditableBlock,
) -> impl Iterator<Item = (usize, &PageMention)> {
    editable.annotations.iter().filter_map(|span| {
        span.annotation
            .mention()
            .map(|mention| (span.start_utf8, mention))
    })
}
