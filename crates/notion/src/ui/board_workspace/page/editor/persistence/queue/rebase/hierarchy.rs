use std::collections::HashSet;
use std::ops::Range;

use crate::model::{
    CardPage, CardPageBlock, CardPageBlockKind, CardPageEditableBlock, CardPageStructuralBlock,
    CreatePageCodeBlockRequest, NotionPageBlockKind, PageBlockPlacement,
    ReorderPageBlockSubtreesRequest,
};

use super::{PageWriteReplayError, ReplayPageSide};

pub(super) fn block<'a>(
    page: &'a CardPage,
    block_id: &str,
    operation: &'static str,
) -> Result<&'a CardPageBlock, PageWriteReplayError> {
    block_on(page, block_id, operation, ReplayPageSide::Authority)
}

pub(super) fn block_on<'a>(
    page: &'a CardPage,
    block_id: &str,
    operation: &'static str,
    side: ReplayPageSide,
) -> Result<&'a CardPageBlock, PageWriteReplayError> {
    page.blocks
        .iter()
        .find(|block| block.block_id == block_id)
        .ok_or_else(|| PageWriteReplayError::MissingBlock {
            operation,
            block_id: block_id.to_string(),
            side,
        })
}

pub(super) fn block_index(
    page: &CardPage,
    block_id: &str,
    operation: &'static str,
) -> Result<usize, PageWriteReplayError> {
    page.blocks
        .iter()
        .position(|block| block.block_id == block_id)
        .ok_or_else(|| PageWriteReplayError::MissingBlock {
            operation,
            block_id: block_id.to_string(),
            side: ReplayPageSide::Authority,
        })
}

pub(super) fn editable_mut<'a>(
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

pub(super) fn subtree_range(blocks: &[CardPageBlock], index: usize) -> Range<usize> {
    let depth = blocks[index].depth;
    let end = blocks[index + 1..]
        .iter()
        .position(|block| block.depth <= depth)
        .map_or(blocks.len(), |offset| index + 1 + offset);
    index..end
}

#[derive(Clone, Copy)]
pub(super) struct NewBlock<'a> {
    pub(super) block_id: &'a str,
    pub(super) parent_id: &'a str,
    pub(super) kind: &'a NotionPageBlockKind,
    pub(super) text: &'a str,
}

pub(super) fn create_block(
    page: &mut CardPage,
    new_block: NewBlock<'_>,
    placement: &PageBlockPlacement,
) -> Result<(), PageWriteReplayError> {
    let NewBlock {
        block_id,
        parent_id,
        ..
    } = new_block;
    ensure_fresh_id(page, block_id, "create block")?;
    let depth = child_depth(page, parent_id, "create block")?;
    let index = insertion_index(page, parent_id, placement, "create block")?;
    let block = projected_block(new_block, depth, "create block")?;
    page.blocks.insert(index, block);
    Ok(())
}

pub(super) fn create_code_block(
    page: &mut CardPage,
    request: &CreatePageCodeBlockRequest,
) -> Result<(), PageWriteReplayError> {
    ensure_fresh_id(page, request.block_id(), "create Code block")?;
    if request.parent_block_id() != page.block_id {
        return Err(PageWriteReplayError::conflict(
            "create Code block",
            "Code append parent is not the page root",
        ));
    }
    let depth = child_depth(page, request.parent_block_id(), "create Code block")?;
    let index = insertion_index(
        page,
        request.parent_block_id(),
        request.placement(),
        "create Code block",
    )?;
    page.blocks.insert(
        index,
        CardPageBlock::code(
            request.block_id(),
            request.parent_block_id(),
            depth,
            request.settings().clone(),
        ),
    );
    Ok(())
}

pub(super) fn delete_subtree(
    page: &mut CardPage,
    block_id: &str,
    operation: &'static str,
) -> Result<(), PageWriteReplayError> {
    let index = block_index(page, block_id, operation)?;
    let range = subtree_range(&page.blocks, index);
    page.blocks.drain(range);
    Ok(())
}

pub(super) fn reorder_subtrees(
    page: &mut CardPage,
    request: &ReorderPageBlockSubtreesRequest,
) -> Result<(), PageWriteReplayError> {
    validate_reorder(page, request)?;
    let roots = request
        .block_ids
        .iter()
        .map(|block_id| take_subtree(page, block_id))
        .collect::<Result<Vec<_>, _>>()?;
    let insertion = insertion_index(
        page,
        &request.target_parent_block_id,
        &request.placement,
        "reorder subtrees",
    )?;
    insert_moved_roots(page, roots, insertion, &request.target_parent_block_id)
}

pub(super) fn projected_editable_kind(
    kind: &NotionPageBlockKind,
    operation: &'static str,
) -> Result<CardPageBlockKind, PageWriteReplayError> {
    let kind = match kind {
        NotionPageBlockKind::Text => CardPageBlockKind::Text,
        NotionPageBlockKind::Header => CardPageBlockKind::SubHeader,
        NotionPageBlockKind::SubHeader => CardPageBlockKind::SubSubHeader,
        NotionPageBlockKind::SubSubHeader => CardPageBlockKind::Heading3,
        NotionPageBlockKind::Header4 => CardPageBlockKind::Heading4,
        NotionPageBlockKind::BulletedList => CardPageBlockKind::BulletedList,
        NotionPageBlockKind::NumberedList => CardPageBlockKind::NumberedList,
        NotionPageBlockKind::ToDo => CardPageBlockKind::ToDoList,
        NotionPageBlockKind::Toggle => CardPageBlockKind::ToggleList,
        NotionPageBlockKind::Page => CardPageBlockKind::PageLink,
        NotionPageBlockKind::Callout => CardPageBlockKind::Callout,
        NotionPageBlockKind::Quote => CardPageBlockKind::Quote,
        NotionPageBlockKind::Code => CardPageBlockKind::Code,
        NotionPageBlockKind::Divider
        | NotionPageBlockKind::Image
        | NotionPageBlockKind::Table
        | NotionPageBlockKind::LinkToPage
        | NotionPageBlockKind::Alias
        | NotionPageBlockKind::Other(_) => {
            return Err(PageWriteReplayError::UnsupportedBlockKind {
                operation,
                kind: kind.clone(),
            });
        }
    };
    Ok(kind)
}

fn ensure_fresh_id(
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

fn child_depth(
    page: &CardPage,
    parent_id: &str,
    operation: &'static str,
) -> Result<usize, PageWriteReplayError> {
    if parent_id == page.block_id {
        return Ok(0);
    }
    let parent = block(page, parent_id, operation)?;
    if !parent.accepts_content_children() {
        return Err(PageWriteReplayError::conflict(
            operation,
            format!("block {parent_id} cannot accept children"),
        ));
    }
    parent
        .depth
        .checked_add(1)
        .ok_or_else(|| PageWriteReplayError::conflict(operation, "page depth overflowed"))
}

fn insertion_index(
    page: &CardPage,
    parent_id: &str,
    placement: &PageBlockPlacement,
    operation: &'static str,
) -> Result<usize, PageWriteReplayError> {
    child_depth(page, parent_id, operation)?;
    match placement {
        PageBlockPlacement::Append => Ok(parent_subtree_end(page, parent_id)),
        PageBlockPlacement::Before(anchor) => {
            direct_child_index(page, parent_id, anchor, operation)
        }
        PageBlockPlacement::After(anchor) => {
            let index = direct_child_index(page, parent_id, anchor, operation)?;
            Ok(subtree_range(&page.blocks, index).end)
        }
    }
}

fn direct_child_index(
    page: &CardPage,
    parent_id: &str,
    block_id: &str,
    operation: &'static str,
) -> Result<usize, PageWriteReplayError> {
    let index = block_index(page, block_id, operation)?;
    if page.blocks[index].parent_block_id != parent_id {
        return Err(PageWriteReplayError::conflict(
            operation,
            format!("anchor {block_id} is not a child of {parent_id}"),
        ));
    }
    Ok(index)
}

fn parent_subtree_end(page: &CardPage, parent_id: &str) -> usize {
    page.blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.parent_block_id == parent_id)
        .map(|(index, _)| subtree_range(&page.blocks, index).end)
        .max()
        .or_else(|| {
            page.blocks
                .iter()
                .position(|block| block.block_id == parent_id)
                .map(|index| index + 1)
        })
        .unwrap_or(page.blocks.len())
}

fn projected_block(
    new_block: NewBlock<'_>,
    depth: usize,
    operation: &'static str,
) -> Result<CardPageBlock, PageWriteReplayError> {
    let NewBlock {
        block_id,
        parent_id,
        kind,
        text,
    } = new_block;
    if matches!(kind, NotionPageBlockKind::Divider) {
        if !text.is_empty() {
            return Err(PageWriteReplayError::conflict(
                operation,
                "divider cannot contain text",
            ));
        }
        return Ok(CardPageBlock::structural(
            block_id,
            parent_id,
            depth,
            CardPageStructuralBlock::Divider,
        ));
    }
    let editable_kind = projected_editable_kind(kind, operation)?;
    Ok(CardPageBlock::from_editable(
        block_id,
        parent_id,
        depth,
        CardPageEditableBlock::new(editable_kind, text, Vec::new()),
    ))
}

fn validate_reorder(
    page: &CardPage,
    request: &ReorderPageBlockSubtreesRequest,
) -> Result<(), PageWriteReplayError> {
    request
        .validate_column_sources(page)
        .map_err(|detail| PageWriteReplayError::conflict("reorder subtrees", detail))?;
    if request.block_ids.is_empty() {
        return Err(PageWriteReplayError::conflict(
            "reorder subtrees",
            "no roots were provided",
        ));
    }
    let moved = request.block_ids.iter().collect::<HashSet<_>>();
    if moved.len() != request.block_ids.len() {
        return Err(PageWriteReplayError::conflict(
            "reorder subtrees",
            "duplicate roots were provided",
        ));
    }
    for block_id in &request.block_ids {
        let index = block_index(page, block_id, "reorder subtrees")?;
        let range = subtree_range(&page.blocks, index);
        if page.blocks[range.clone()]
            .iter()
            .any(|block| moved.contains(&block.block_id) && block.block_id != *block_id)
        {
            return Err(PageWriteReplayError::conflict(
                "reorder subtrees",
                "moved roots overlap",
            ));
        }
    }
    Ok(())
}

fn take_subtree(
    page: &mut CardPage,
    block_id: &str,
) -> Result<Vec<CardPageBlock>, PageWriteReplayError> {
    let index = block_index(page, block_id, "reorder subtrees")?;
    let range = subtree_range(&page.blocks, index);
    Ok(page.blocks.drain(range).collect())
}

fn insert_moved_roots(
    page: &mut CardPage,
    roots: Vec<Vec<CardPageBlock>>,
    insertion: usize,
    parent_id: &str,
) -> Result<(), PageWriteReplayError> {
    let target_depth = child_depth(page, parent_id, "reorder subtrees")?;
    let mut flattened = Vec::new();
    for mut subtree in roots {
        let source_depth = subtree[0].depth;
        let delta = target_depth as isize - source_depth as isize;
        subtree[0].parent_block_id = parent_id.to_string();
        for block in &mut subtree {
            block.depth = block.depth.checked_add_signed(delta).ok_or_else(|| {
                PageWriteReplayError::conflict("reorder subtrees", "page depth overflowed")
            })?;
        }
        flattened.extend(subtree);
    }
    page.blocks.splice(insertion..insertion, flattened);
    Ok(())
}
