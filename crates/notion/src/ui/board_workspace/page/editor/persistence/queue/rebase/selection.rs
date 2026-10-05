use crate::model::{
    validate_column_safe_page_block_mutations, BreakPageTextSelectionRequest, CardPage,
    CardPageBlock, CardPageEditableBlock, PageBlockPlacement, PageBlockStructuralMutation,
    PageTextLineBreak, PageTextSelectionEndpoint, PastePageTextSelectionRequest,
    ReplacePageTextSelectionRequest,
};

use super::super::super::super::rich_text::annotations::{
    annotated_text_slice, apply_annotated_text, concatenate_annotated_text, AnnotatedText,
};
use super::hierarchy::{
    block_index, create_block, delete_subtree, projected_editable_kind, reorder_subtrees,
    subtree_range, NewBlock,
};
use super::PageWriteReplayError;

pub(super) fn replace_text_selection(
    page: &mut CardPage,
    request: &ReplacePageTextSelectionRequest,
) -> Result<(), PageWriteReplayError> {
    validate_selection_structure(page, request.structural_mutations())?;
    let endpoints = selection_content(page, request.start(), request.end())?;
    let survivor = concatenate_annotated_text([
        endpoints.prefix,
        AnnotatedText::plain(request.replacement()),
        endpoints.suffix,
    ]);
    apply_annotated_text(
        page.blocks[endpoints.first_index]
            .editable_content_mut()
            .expect("validated selection survivor must remain editable"),
        survivor,
    );
    apply_validated_selection_structure(page, request.structural_mutations())
}

pub(super) fn break_text_selection(
    page: &mut CardPage,
    request: &BreakPageTextSelectionRequest,
) -> Result<(), PageWriteReplayError> {
    validate_selection_structure(page, request.structural_mutations())?;
    let endpoints = selection_content(page, request.start(), request.end())?;
    let survivor_id = request.start().block_id.clone();
    let survivor_parent = page.blocks[endpoints.first_index].parent_block_id.clone();
    let survivor_depth = page.blocks[endpoints.first_index].depth;
    apply_validated_selection_structure(page, request.structural_mutations())?;
    let survivor_index = block_index(page, &survivor_id, "break text selection")?;
    match request.line_break() {
        PageTextLineBreak::ShiftEnter => apply_shift_enter(page, survivor_index, endpoints),
        PageTextLineBreak::Enter {
            new_block_id,
            new_block_kind,
        } => apply_enter(
            page,
            survivor_index,
            endpoints,
            EnterNewBlock {
                parent_id: &survivor_parent,
                depth: survivor_depth,
                block_id: new_block_id,
                kind: new_block_kind,
            },
        ),
    }
}

pub(super) fn paste_text_selection(
    page: &mut CardPage,
    request: &PastePageTextSelectionRequest,
) -> Result<(), PageWriteReplayError> {
    validate_endpoint(page, request.start(), "paste selection start")?;
    validate_endpoint(page, request.end(), "paste selection end")?;
    let end_index = block_index(page, &request.end().block_id, "paste text selection")?;
    let end = page.blocks[end_index].clone();
    create_block(
        page,
        NewBlock {
            block_id: request.new_block_id(),
            parent_id: &end.parent_block_id,
            kind: &crate::model::NotionPageBlockKind::Text,
            text: request.text(),
        },
        &PageBlockPlacement::After(end.block_id),
    )
}

struct SelectionContent {
    first_index: usize,
    prefix: AnnotatedText,
    suffix: AnnotatedText,
}

fn selection_content(
    page: &CardPage,
    start: &PageTextSelectionEndpoint,
    end: &PageTextSelectionEndpoint,
) -> Result<SelectionContent, PageWriteReplayError> {
    let first_index = block_index(page, &start.block_id, "text selection")?;
    let last_index = block_index(page, &end.block_id, "text selection")?;
    if first_index >= last_index {
        return Err(PageWriteReplayError::conflict(
            "text selection",
            "endpoints are not in document order",
        ));
    }
    let first = page.blocks[first_index]
        .editable_content()
        .ok_or_else(|| PageWriteReplayError::conflict("text selection", "start is not editable"))?;
    let last = page.blocks[last_index]
        .editable_content()
        .ok_or_else(|| PageWriteReplayError::conflict("text selection", "end is not editable"))?;
    validate_text_offset(&start.block_id, &first.text, start.offset_utf8)?;
    validate_text_offset(&end.block_id, &last.text, end.offset_utf8)?;
    Ok(SelectionContent {
        first_index,
        prefix: annotated_text_slice(first, 0..start.offset_utf8),
        suffix: annotated_text_slice(last, end.offset_utf8..last.text.len()),
    })
}

fn validate_endpoint(
    page: &CardPage,
    endpoint: &PageTextSelectionEndpoint,
    operation: &'static str,
) -> Result<(), PageWriteReplayError> {
    let index = block_index(page, &endpoint.block_id, operation)?;
    let editable = page.blocks[index]
        .editable_content()
        .ok_or_else(|| PageWriteReplayError::conflict(operation, "endpoint is not editable"))?;
    validate_text_offset(&endpoint.block_id, &editable.text, endpoint.offset_utf8)
}

fn validate_text_offset(
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

fn validate_selection_structure(
    page: &CardPage,
    mutations: &[PageBlockStructuralMutation],
) -> Result<(), PageWriteReplayError> {
    validate_column_safe_page_block_mutations(page, mutations)
        .map_err(|detail| PageWriteReplayError::conflict("text selection", detail))
}

fn apply_validated_selection_structure(
    page: &mut CardPage,
    mutations: &[PageBlockStructuralMutation],
) -> Result<(), PageWriteReplayError> {
    for mutation in mutations {
        match mutation {
            PageBlockStructuralMutation::Reorder(request) => reorder_subtrees(page, request)?,
            PageBlockStructuralMutation::Delete(request) => {
                delete_subtree(page, &request.block_id, "text selection delete")?
            }
        }
    }
    Ok(())
}

fn apply_shift_enter(
    page: &mut CardPage,
    survivor_index: usize,
    endpoints: SelectionContent,
) -> Result<(), PageWriteReplayError> {
    let survivor = concatenate_annotated_text([
        endpoints.prefix,
        AnnotatedText::plain("\n"),
        endpoints.suffix,
    ]);
    apply_annotated_text(
        page.blocks[survivor_index]
            .editable_content_mut()
            .ok_or_else(|| {
                PageWriteReplayError::conflict("break text selection", "survivor is not editable")
            })?,
        survivor,
    );
    Ok(())
}

struct EnterNewBlock<'a> {
    parent_id: &'a str,
    depth: usize,
    block_id: &'a str,
    kind: &'a crate::model::NotionPageBlockKind,
}

fn apply_enter(
    page: &mut CardPage,
    survivor_index: usize,
    endpoints: SelectionContent,
    new_block: EnterNewBlock<'_>,
) -> Result<(), PageWriteReplayError> {
    let EnterNewBlock {
        parent_id,
        depth,
        block_id: new_block_id,
        kind: new_kind,
    } = new_block;
    apply_annotated_text(
        page.blocks[survivor_index]
            .editable_content_mut()
            .ok_or_else(|| {
                PageWriteReplayError::conflict("break text selection", "survivor is not editable")
            })?,
        endpoints.prefix,
    );
    if page
        .blocks
        .iter()
        .any(|block| block.block_id == new_block_id)
    {
        return Err(PageWriteReplayError::conflict(
            "break text selection",
            format!("block {new_block_id} already exists"),
        ));
    }
    let kind = projected_editable_kind(new_kind, "break text selection")?;
    let destination =
        CardPageEditableBlock::new(kind, endpoints.suffix.text, endpoints.suffix.annotations);
    let insertion = subtree_range(&page.blocks, survivor_index).end;
    page.blocks.insert(
        insertion,
        CardPageBlock::from_editable(new_block_id, parent_id, depth, destination),
    );
    Ok(())
}
