use super::PageMutationPlan;
use std::collections::HashMap;

use super::super::{rich_text::PageWriteTextProjection, support::page_block_subtree_end};
use super::snapshot::PageSnapshotIndex;
use crate::model::{
    CardPage, CardPageBlock, ConvertPageBlockToDividerRequest, DeletePageBlockRequest,
    PageCodeBlockSourceText, PageMutation, ReplacePageBlockWithCodeRequest,
    RestorePageBlockFromCodeRequest, RestorePageBlockFromDividerRequest,
};
use crate::ui::{CardPageBlockColor, CardPageBlockKind, CardPageStructuralBlock};

pub(super) enum PageCodeHistoryMutation {
    Replace(ReplacePageBlockWithCodeRequest),
    Restore(RestorePageBlockFromCodeRequest),
}

pub(super) struct PreparedPageBlockDeletion {
    request: DeletePageBlockRequest,
    projection: PageWriteTextProjection,
}

pub(super) fn prepare_deleted_page_blocks(
    current: &CardPage,
    index: &PageSnapshotIndex<'_>,
) -> Vec<PreparedPageBlockDeletion> {
    let mut deletions = Vec::<(DeletePageBlockRequest, Vec<String>)>::new();
    let mut deletion_by_absent_block = HashMap::<&str, usize>::new();
    for block in &current.blocks {
        if index.contains_target(&block.block_id) {
            continue;
        }
        let deletion_index = if block.parent_block_id == current.block_id
            || index.contains_target(&block.parent_block_id)
        {
            let deletion_index = deletions.len();
            deletions.push((
                DeletePageBlockRequest {
                    block_id: block.block_id.clone(),
                },
                Vec::new(),
            ));
            deletion_index
        } else {
            let Some(&deletion_index) =
                deletion_by_absent_block.get(block.parent_block_id.as_str())
            else {
                continue;
            };
            deletion_index
        };
        deletion_by_absent_block.insert(block.block_id.as_str(), deletion_index);
        deletions[deletion_index].1.push(block.block_id.clone());
    }
    deletions
        .into_iter()
        .map(|(request, retired_blocks)| PreparedPageBlockDeletion {
            request,
            projection: PageWriteTextProjection::default().with_retired_blocks(retired_blocks),
        })
        .collect()
}

impl PreparedPageBlockDeletion {
    pub(super) fn request(&self) -> &DeletePageBlockRequest {
        &self.request
    }
}

impl PageCodeHistoryMutation {
    pub(super) fn into_page_mutation(self) -> PageMutation {
        match self {
            Self::Replace(request) => PageMutation::ReplaceBlockWithCode(request),
            Self::Restore(request) => PageMutation::RestoreBlockFromCode(request),
        }
    }
}

pub(super) fn page_code_history_mutation(
    current: &CardPage,
    target: &CardPage,
) -> Option<PageCodeHistoryMutation> {
    if let Some(parsed) = parse_page_code_replacement(current, target) {
        return ReplacePageBlockWithCodeRequest::new(
            parsed.source.block_id.clone(),
            parsed.code.block_id.clone(),
            parsed.source_text,
            parsed.settings,
        )
        .ok()
        .map(PageCodeHistoryMutation::Replace);
    }
    let parsed = parse_page_code_replacement(target, current)?;
    RestorePageBlockFromCodeRequest::new(
        parsed.source.block_id.clone(),
        parsed.code.block_id.clone(),
        parsed.source_text,
    )
    .ok()
    .map(PageCodeHistoryMutation::Restore)
}

struct ParsedPageCodeReplacement<'a> {
    source: &'a CardPageBlock,
    code: &'a CardPageBlock,
    source_text: PageCodeBlockSourceText,
    settings: crate::model::CardPageCodeSettings,
}

fn parse_page_code_replacement<'a>(
    source_page: &'a CardPage,
    code_page: &'a CardPage,
) -> Option<ParsedPageCodeReplacement<'a>> {
    if !page_metadata_matches(source_page, code_page)
        || source_page.blocks.len() != code_page.blocks.len()
    {
        return None;
    }
    let differing = source_page
        .blocks
        .iter()
        .zip(&code_page.blocks)
        .enumerate()
        .filter(|(_, (source, code))| source != code)
        .collect::<Vec<_>>();
    let [(index, (source, code))] = differing.as_slice() else {
        return None;
    };
    let editable = source.editable_content()?;
    let code_editable = code.editable_content()?;
    let source_text = PageCodeBlockSourceText::parse(editable.text.clone()).ok()?;
    let valid = editable.kind == CardPageBlockKind::Text
        && editable.annotations.is_empty()
        && source.parent_block_id == source_page.block_id
        && source.depth == 0
        && page_block_subtree_end(&source_page.blocks, *index) == *index + 1
        && source.block_id != code.block_id
        && !source_page
            .blocks
            .iter()
            .any(|block| block.block_id == code.block_id)
        && code.parent_block_id == source.parent_block_id
        && code.depth == source.depth
        && code.color == CardPageBlockColor::default()
        && code.icon.is_none()
        && code_editable.kind == CardPageBlockKind::Code
        && code_editable.text.is_empty()
        && code_editable.annotations.is_empty();
    valid.then(|| ParsedPageCodeReplacement {
        source,
        code,
        source_text,
        settings: code_editable
            .code_settings()
            .expect("verified Code history block must retain settings"),
    })
}

fn page_metadata_matches(left: &CardPage, right: &CardPage) -> bool {
    left.block_id == right.block_id
        && left.title == right.title
        && left.status == right.status
        && left.properties == right.properties
}

pub(super) enum PageDividerHistoryMutation {
    Convert(ConvertPageBlockToDividerRequest),
    Restore(RestorePageBlockFromDividerRequest),
}

impl PageDividerHistoryMutation {
    pub(super) fn into_page_mutation(self) -> PageMutation {
        match self {
            Self::Convert(request) => PageMutation::ConvertBlockToDivider(request),
            Self::Restore(request) => PageMutation::RestoreBlockFromDivider(request),
        }
    }
}

pub(super) fn page_divider_history_mutation(
    current: &CardPage,
    target: &CardPage,
) -> Option<PageDividerHistoryMutation> {
    if let Some(parsed) = parse_page_divider_snapshot(current, target) {
        return ConvertPageBlockToDividerRequest::new(
            parsed.literal.block_id.clone(),
            parsed.continuation.block_id.clone(),
        )
        .ok()
        .map(PageDividerHistoryMutation::Convert);
    }
    let parsed = parse_page_divider_snapshot(target, current)?;
    let text = parsed.literal.editable_content()?.text.clone();
    RestorePageBlockFromDividerRequest::new(
        parsed.literal.block_id.clone(),
        parsed.continuation.block_id.clone(),
        text,
    )
    .ok()
    .map(PageDividerHistoryMutation::Restore)
}

struct ParsedPageDividerSnapshot<'a> {
    literal: &'a CardPageBlock,
    continuation: &'a CardPageBlock,
}

fn parse_page_divider_snapshot<'a>(
    literal_page: &'a CardPage,
    converted_page: &'a CardPage,
) -> Option<ParsedPageDividerSnapshot<'a>> {
    if literal_page.block_id != converted_page.block_id
        || converted_page.blocks.len() != literal_page.blocks.len() + 1
    {
        return None;
    }
    for (index, literal) in literal_page.blocks.iter().enumerate() {
        let Some(editable) = literal.editable_content() else {
            continue;
        };
        if editable.kind != CardPageBlockKind::Text || editable.text != "---" {
            continue;
        }
        if let Some(parsed) = parse_page_divider_at(literal_page, converted_page, index, literal) {
            return Some(parsed);
        }
    }
    None
}

fn parse_page_divider_at<'a>(
    literal_page: &'a CardPage,
    converted_page: &'a CardPage,
    index: usize,
    literal: &'a CardPageBlock,
) -> Option<ParsedPageDividerSnapshot<'a>> {
    let divider = &converted_page.blocks[index];
    if divider.block_id != literal.block_id
        || divider.parent_block_id != literal.parent_block_id
        || divider.depth != literal.depth
        || divider.structural_content() != Some(&CardPageStructuralBlock::Divider)
    {
        return None;
    }
    let mut expected_divider = literal.clone();
    expected_divider.content = divider.content.clone();
    if &expected_divider != divider {
        return None;
    }
    let continuation = &converted_page.blocks[index + 1];
    let continuation_editable = continuation.editable_content()?;
    if continuation.block_id == literal.block_id
        || literal_page
            .blocks
            .iter()
            .any(|block| block.block_id == continuation.block_id)
        || continuation.parent_block_id != literal.parent_block_id
        || continuation.depth != literal.depth
        || continuation.color != CardPageBlockColor::default()
        || continuation.icon.is_some()
        || continuation_editable.kind != CardPageBlockKind::Text
        || !continuation_editable.text.is_empty()
        || !continuation_editable.annotations.is_empty()
        || literal_page.blocks[..index] != converted_page.blocks[..index]
        || literal_page.blocks[index + 1..] != converted_page.blocks[index + 2..]
    {
        return None;
    }
    Some(ParsedPageDividerSnapshot {
        literal,
        continuation,
    })
}

impl PageMutationPlan {
    pub(super) fn persist_deleted_page_blocks(
        &mut self,
        page_id: &str,
        deletions: Vec<PreparedPageBlockDeletion>,
    ) {
        for deletion in deletions {
            self.enqueue_page_mutation_with_projection(
                page_id,
                PageMutation::DeleteBlock(deletion.request),
                deletion.projection,
            );
        }
    }
}
