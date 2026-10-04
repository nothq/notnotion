use crate::model::{
    CardPage, CardPageBlock, CardPageBlockContent, CardPageBlockKind, CardPageStructuralBlock,
    ConvertPageBlockToDividerRequest, DuplicatePageAliasRequest, MergePageBlocksRequest,
    ReplacePageBlockWithCodeRequest, ReplacePageBlockWithDividerRequest,
    RestorePageBlockFromCodeRequest, RestorePageBlockFromDividerRequest, SplitPageBlockRequest,
};

use super::super::super::super::rich_text::annotations::{
    annotated_text_slice, apply_annotated_text, apply_plain_text_annotation_replacement,
    concatenate_annotated_text,
};
use super::hierarchy::{
    block_index, delete_subtree, editable_mut, projected_editable_kind, subtree_range,
};
use super::PageWriteReplayError;

pub(super) fn replace_text(
    page: &mut CardPage,
    block_id: &str,
    text: &str,
) -> Result<(), PageWriteReplayError> {
    if block_id == page.block_id {
        page.title = text.to_string();
        return Ok(());
    }
    let editable = editable_mut(page, block_id, "replace block text")?;
    let previous = editable.text.clone();
    apply_plain_text_annotation_replacement(editable, &previous, text);
    editable.text = text.to_string();
    Ok(())
}

pub(super) fn convert_block(
    page: &mut CardPage,
    block_id: &str,
    kind: &crate::model::NotionPageBlockKind,
) -> Result<(), PageWriteReplayError> {
    let kind = projected_editable_kind(kind, "convert block")?;
    editable_mut(page, block_id, "convert block")?.set_kind(kind);
    Ok(())
}

pub(super) fn duplicate_alias(
    page: &mut CardPage,
    request: &DuplicatePageAliasRequest,
) -> Result<(), PageWriteReplayError> {
    if page
        .blocks
        .iter()
        .any(|block| block.block_id == request.new_block_id())
    {
        return Err(PageWriteReplayError::conflict(
            "duplicate alias",
            format!("block {} already exists", request.new_block_id()),
        ));
    }
    let index = block_index(page, request.source_block_id(), "duplicate alias")?;
    let source = &page.blocks[index];
    if source.parent_block_id != page.block_id
        || subtree_range(&page.blocks, index).end != index + 1
    {
        return Err(PageWriteReplayError::conflict(
            "duplicate alias",
            "source is not a root-level leaf",
        ));
    }
    let mut duplicate = source.clone();
    duplicate.block_id = request.new_block_id().to_string();
    duplicate.last_edited = None;
    duplicate
        .alias_content_mut()
        .ok_or_else(|| PageWriteReplayError::conflict("duplicate alias", "source is not an alias"))?
        .copied_from_block_id = Some(source.block_id.clone());
    page.blocks.insert(index + 1, duplicate);
    Ok(())
}

pub(super) fn replace_with_divider(
    page: &mut CardPage,
    request: &ReplacePageBlockWithDividerRequest,
) -> Result<(), PageWriteReplayError> {
    let index = leaf_editable_index(page, request.source_block_id(), "replace with divider")?;
    ensure_fresh(page, request.divider_block_id(), "replace with divider")?;
    let source = page.blocks[index].clone();
    page.blocks[index] = CardPageBlock::structural(
        request.divider_block_id(),
        source.parent_block_id,
        source.depth,
        CardPageStructuralBlock::Divider,
    );
    Ok(())
}

pub(super) fn replace_with_code(
    page: &mut CardPage,
    request: &ReplacePageBlockWithCodeRequest,
) -> Result<(), PageWriteReplayError> {
    let index = leaf_editable_index(page, request.source_block_id(), "replace with Code")?;
    ensure_fresh(page, request.code_block_id(), "replace with Code")?;
    let source = &page.blocks[index];
    let valid_source = source.editable_content().is_some_and(|editable| {
        editable.kind == CardPageBlockKind::Text
            && editable.text == request.source_text().expected_persisted_text()
            && editable.annotations.is_empty()
    });
    if !valid_source || source.parent_block_id != page.block_id || source.depth != 0 {
        return Err(PageWriteReplayError::conflict(
            "replace with Code",
            "source is not the exact root-level verified Text token",
        ));
    }
    let parent_block_id = source.parent_block_id.clone();
    let depth = source.depth;
    page.blocks[index] = CardPageBlock::code(
        request.code_block_id(),
        parent_block_id,
        depth,
        request.settings().clone(),
    );
    Ok(())
}

pub(super) fn restore_from_code(
    page: &mut CardPage,
    request: &RestorePageBlockFromCodeRequest,
) -> Result<(), PageWriteReplayError> {
    let index = leaf_editable_index(page, request.code_block_id(), "restore from Code")?;
    ensure_fresh(page, request.source_block_id(), "restore from Code")?;
    let code = &page.blocks[index];
    let valid_code = code.editable_content().is_some_and(|editable| {
        editable.kind == CardPageBlockKind::Code
            && editable.text.is_empty()
            && editable.annotations.is_empty()
    });
    if !valid_code || code.parent_block_id != page.block_id || code.depth != 0 {
        return Err(PageWriteReplayError::conflict(
            "restore from Code",
            "replacement is not an empty root-level leaf Code block",
        ));
    }
    let parent_block_id = code.parent_block_id.clone();
    let depth = code.depth;
    page.blocks[index] = CardPageBlock::editable(
        request.source_block_id(),
        parent_block_id,
        depth,
        CardPageBlockKind::Text,
        request.source_text().as_str(),
    );
    Ok(())
}

pub(super) fn convert_to_divider(
    page: &mut CardPage,
    request: &ConvertPageBlockToDividerRequest,
) -> Result<(), PageWriteReplayError> {
    let index = leaf_editable_index(page, request.source_block_id(), "convert to divider")?;
    ensure_fresh(page, request.continuation_block_id(), "convert to divider")?;
    let source = page.blocks[index].clone();
    page.blocks[index].content = CardPageBlockContent::Structural(CardPageStructuralBlock::Divider);
    let continuation = CardPageBlock::editable(
        request.continuation_block_id(),
        source.parent_block_id,
        source.depth,
        CardPageBlockKind::Text,
        "",
    );
    page.blocks.insert(index + 1, continuation);
    Ok(())
}

pub(super) fn restore_from_divider(
    page: &mut CardPage,
    request: &RestorePageBlockFromDividerRequest,
) -> Result<(), PageWriteReplayError> {
    let divider_index = block_index(page, request.divider_block_id(), "restore divider")?;
    let continuation_index = block_index(page, request.continuation_block_id(), "restore divider")?;
    let divider = &page.blocks[divider_index];
    let continuation = &page.blocks[continuation_index];
    let valid = divider.structural_content() == Some(&CardPageStructuralBlock::Divider)
        && subtree_range(&page.blocks, divider_index).end == divider_index + 1
        && continuation_index == divider_index + 1
        && continuation.parent_block_id == divider.parent_block_id
        && continuation.editable_content().is_some_and(|editable| {
            editable.kind == CardPageBlockKind::Text && editable.text.is_empty()
        })
        && subtree_range(&page.blocks, continuation_index).end == continuation_index + 1;
    if !valid {
        return Err(PageWriteReplayError::conflict(
            "restore divider",
            "divider and continuation no longer form a restorable pair",
        ));
    }
    page.blocks[divider_index].content =
        CardPageBlockContent::Editable(crate::model::CardPageEditableBlock::new(
            CardPageBlockKind::Text,
            request.text(),
            Vec::new(),
        ));
    page.blocks.remove(continuation_index);
    Ok(())
}

pub(super) fn split_block(
    page: &mut CardPage,
    request: &SplitPageBlockRequest,
) -> Result<(), PageWriteReplayError> {
    let index = block_index(page, &request.block_id, "split block")?;
    ensure_fresh(page, &request.new_block_id, "split block")?;
    let source = page.blocks[index].clone();
    let editable = source
        .editable_content()
        .ok_or_else(|| PageWriteReplayError::conflict("split block", "source is not editable"))?;
    let offset = super::text_offset::utf8_offset_from_utf16(
        &request.block_id,
        &editable.text,
        request.split_offset_utf16,
    )?;
    let left = annotated_text_slice(editable, 0..offset);
    let right = annotated_text_slice(editable, offset..editable.text.len());
    let mut destination = editable.clone();
    destination.set_kind(projected_editable_kind(
        &request.new_block_kind,
        "split block",
    )?);
    apply_annotated_text(&mut destination, right);
    apply_annotated_text(
        page.blocks[index]
            .editable_content_mut()
            .expect("validated split source must remain editable"),
        left,
    );
    reparent_split_children(page, index, &request.new_block_id);
    page.blocks.insert(
        index + 1,
        CardPageBlock::from_editable(
            request.new_block_id.clone(),
            source.parent_block_id,
            source.depth,
            destination,
        ),
    );
    Ok(())
}

pub(super) fn merge_blocks(
    page: &mut CardPage,
    request: &MergePageBlocksRequest,
) -> Result<(), PageWriteReplayError> {
    let source_index = block_index(page, &request.source_block_id, "merge blocks")?;
    let target_index = block_index(page, &request.target_block_id, "merge blocks")?;
    validate_merge(page, source_index, target_index)?;
    let source = page.blocks[source_index]
        .editable_content()
        .expect("validated merge source must be editable")
        .clone();
    let target = page.blocks[target_index]
        .editable_content()
        .expect("validated merge target must be editable")
        .clone();
    let merged = concatenate_annotated_text([
        annotated_text_slice(&target, 0..target.text.len()),
        annotated_text_slice(&source, 0..source.text.len()),
    ]);
    apply_annotated_text(
        page.blocks[target_index]
            .editable_content_mut()
            .expect("validated merge target must remain editable"),
        merged,
    );
    remove_merged_source(page, source_index, source.kind);
    Ok(())
}

fn leaf_editable_index(
    page: &CardPage,
    block_id: &str,
    operation: &'static str,
) -> Result<usize, PageWriteReplayError> {
    let index = block_index(page, block_id, operation)?;
    if page.blocks[index].editable_content().is_none()
        || subtree_range(&page.blocks, index).end != index + 1
    {
        return Err(PageWriteReplayError::conflict(
            operation,
            format!("block {block_id} is not an editable leaf"),
        ));
    }
    Ok(index)
}

fn ensure_fresh(
    page: &CardPage,
    block_id: &str,
    operation: &'static str,
) -> Result<(), PageWriteReplayError> {
    if block_id == page.block_id || page.blocks.iter().any(|block| block.block_id == block_id) {
        return Err(PageWriteReplayError::conflict(
            operation,
            format!("block id {block_id} already exists"),
        ));
    }
    Ok(())
}

fn reparent_split_children(page: &mut CardPage, source_index: usize, new_parent_id: &str) {
    let source_id = page.blocks[source_index].block_id.clone();
    let end = subtree_range(&page.blocks, source_index).end;
    for child in &mut page.blocks[source_index + 1..end] {
        if child.parent_block_id == source_id {
            child.parent_block_id = new_parent_id.to_string();
        }
    }
}

fn validate_merge(
    page: &CardPage,
    source_index: usize,
    target_index: usize,
) -> Result<(), PageWriteReplayError> {
    let source = &page.blocks[source_index];
    let target = &page.blocks[target_index];
    let previous_sibling = (0..source_index)
        .rev()
        .find(|index| page.blocks[*index].parent_block_id == source.parent_block_id);
    if source.editable_content().is_none()
        || target.editable_content().is_none()
        || source.parent_block_id != target.parent_block_id
        || previous_sibling != Some(target_index)
    {
        return Err(PageWriteReplayError::conflict(
            "merge blocks",
            "source and target are no longer adjacent editable siblings",
        ));
    }
    Ok(())
}

fn remove_merged_source(page: &mut CardPage, source_index: usize, source_kind: CardPageBlockKind) {
    let range = subtree_range(&page.blocks, source_index);
    if source_kind == CardPageBlockKind::ToggleList {
        page.blocks.drain(range);
        return;
    }
    let source = page.blocks[source_index].clone();
    for descendant in &mut page.blocks[source_index + 1..range.end] {
        descendant.depth -= 1;
        if descendant.parent_block_id == source.block_id {
            descendant
                .parent_block_id
                .clone_from(&source.parent_block_id);
        }
    }
    page.blocks.remove(source_index);
}

pub(super) fn set_page_icon(
    page: &mut CardPage,
    block_id: &str,
    icon: Option<crate::model::PageShellIcon>,
) -> Result<(), PageWriteReplayError> {
    if block_id == page.block_id {
        return Ok(());
    }
    let index = block_index(page, block_id, "set page icon")?;
    page.blocks[index].icon = icon;
    Ok(())
}

pub(super) fn delete_block(
    page: &mut CardPage,
    block_id: &str,
) -> Result<(), PageWriteReplayError> {
    delete_subtree(page, block_id, "delete block")
}
