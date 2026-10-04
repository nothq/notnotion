use crate::model::{CardPage, PageMutation, PageTextLineBreak};

use super::super::super::super::rich_text::{PageWrite, PageWriteOperation};
use super::PageWriteReplayError;

mod content;
mod effect;
mod fixed;
mod order;
mod table_cell;

use effect::{EffectSide, PageEffect};
use fixed::{DeleteBlockPostcondition, FixedBlockPostcondition};
use order::{fixed_write_order, mutation_order_effects};
use table_cell::SimpleTableCellPostcondition;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum FailedPageWriteDisposition {
    Applied,
    NotApplied,
    Conflict,
    RetrySafe,
}

enum FailedPageWritePostcondition {
    RichText(PagePostcondition),
    Mutation(PageMutationPostcondition),
}

enum PagePostcondition {
    Touched(PageEffect),
    Fixed(FixedBlockPostcondition),
    Delete(DeleteBlockPostcondition),
    RetrySafe,
}

enum PageMutationPostcondition {
    CreateBlock(PagePostcondition),
    CreateCodeBlock(PagePostcondition),
    DuplicateAlias(PagePostcondition),
    ReplaceBlockWithCode(PagePostcondition),
    RestoreBlockFromCode(PagePostcondition),
    ReplaceBlockWithDivider(PagePostcondition),
    ConvertBlockToDivider(PagePostcondition),
    RestoreBlockFromDivider(PagePostcondition),
    ReplaceBlockText(PagePostcondition),
    ReplaceSimpleTableCell(SimpleTableCellPostcondition),
    ReplaceBlockTextAndConvert(PagePostcondition),
    ReplaceTextSelection(PagePostcondition),
    BreakTextSelection(PagePostcondition),
    PasteTextSelection(PagePostcondition),
    SplitBlock(PagePostcondition),
    MergeBlocks(PagePostcondition),
    ConvertBlock(PagePostcondition),
    SetToDoState(PagePostcondition),
    SetCodeLanguage(PagePostcondition),
    SetCodeWrap(PagePostcondition),
    SetPageIcon(PagePostcondition),
    SetBlockColor(PagePostcondition),
    SetQuoteSize(PagePostcondition),
    ResizeColumns(PagePostcondition),
    DeleteBlock(PagePostcondition),
    ReorderSubtrees(PagePostcondition),
    InsertMention(PagePostcondition),
    UpdateMention(PagePostcondition),
}

pub(in super::super) fn classify_failed_page_write(
    write: &PageWrite,
    before: &CardPage,
    after: &CardPage,
    authority: &CardPage,
) -> Result<FailedPageWriteDisposition, PageWriteReplayError> {
    if before.block_id != after.block_id || before.block_id != authority.block_id {
        return Err(PageWriteReplayError::PageIdentity {
            baseline_id: before.block_id.clone(),
            authority_id: authority.block_id.clone(),
        });
    }
    FailedPageWritePostcondition::new(write, before, after)
        .map(|postcondition| postcondition.classify(authority))
}

impl FailedPageWritePostcondition {
    fn new(
        write: &PageWrite,
        before: &CardPage,
        after: &CardPage,
    ) -> Result<Self, PageWriteReplayError> {
        match &write.operation {
            PageWriteOperation::RichText(_) => Ok(Self::RichText(PagePostcondition::Touched(
                PageEffect::between(before, after, Vec::new()),
            ))),
            PageWriteOperation::Mutation(mutation) => Ok(Self::Mutation(
                PageMutationPostcondition::new(write, mutation, before, after)?,
            )),
        }
    }

    fn classify(&self, authority: &CardPage) -> FailedPageWriteDisposition {
        match self {
            Self::RichText(postcondition) => postcondition.classify(authority),
            Self::Mutation(postcondition) => postcondition.classify(authority),
        }
    }
}

impl PagePostcondition {
    fn touched(mutation: &PageMutation, before: &CardPage, after: &CardPage) -> Self {
        Self::Touched(PageEffect::between(
            before,
            after,
            mutation_order_effects(mutation, before, after),
        ))
    }

    fn classify(&self, authority: &CardPage) -> FailedPageWriteDisposition {
        match self {
            Self::Fixed(postcondition) => postcondition.classify(authority),
            Self::Delete(postcondition) => postcondition.classify(authority),
            Self::Touched(effect) => classify_touched_effect(effect, authority),
            Self::RetrySafe => FailedPageWriteDisposition::RetrySafe,
        }
    }
}

fn classify_touched_effect(
    effect: &PageEffect,
    authority: &CardPage,
) -> FailedPageWriteDisposition {
    let before_matches = effect.matches(authority, EffectSide::Before);
    let after_matches = effect.matches(authority, EffectSide::After);
    match (before_matches, after_matches) {
        (_, true) => FailedPageWriteDisposition::Applied,
        (true, false) => FailedPageWriteDisposition::NotApplied,
        (false, false) => FailedPageWriteDisposition::Conflict,
    }
}

impl PageMutationPostcondition {
    fn new(
        write: &PageWrite,
        mutation: &PageMutation,
        before: &CardPage,
        after: &CardPage,
    ) -> Result<Self, PageWriteReplayError> {
        let touched = || PagePostcondition::touched(mutation, before, after);
        Ok(match mutation {
            PageMutation::CreateBlock(request) => {
                Self::CreateBlock(fixed(write, &request.block_id, after)?)
            }
            PageMutation::CreateCodeBlock(request) => {
                Self::CreateCodeBlock(fixed(write, request.block_id(), after)?)
            }
            PageMutation::SplitBlock(request) => {
                Self::SplitBlock(fixed(write, &request.new_block_id, after)?)
            }
            PageMutation::BreakTextSelection(request) => {
                Self::BreakTextSelection(match request.line_break() {
                    PageTextLineBreak::Enter { new_block_id, .. } => {
                        fixed(write, new_block_id, after)?
                    }
                    PageTextLineBreak::ShiftEnter => touched(),
                })
            }
            PageMutation::PasteTextSelection(request) => {
                Self::PasteTextSelection(fixed(write, request.new_block_id(), after)?)
            }
            PageMutation::DeleteBlock(request) => Self::DeleteBlock(deleted(&request.block_id)),
            PageMutation::DuplicateAlias(_) => Self::DuplicateAlias(touched()),
            PageMutation::ReplaceBlockWithCode(_) => Self::ReplaceBlockWithCode(touched()),
            PageMutation::RestoreBlockFromCode(_) => Self::RestoreBlockFromCode(touched()),
            PageMutation::ReplaceBlockWithDivider(_) => Self::ReplaceBlockWithDivider(touched()),
            PageMutation::ConvertBlockToDivider(_) => Self::ConvertBlockToDivider(touched()),
            PageMutation::RestoreBlockFromDivider(_) => Self::RestoreBlockFromDivider(touched()),
            PageMutation::ReplaceBlockText(_) => Self::ReplaceBlockText(touched()),
            PageMutation::ReplaceSimpleTableCell(request) => Self::ReplaceSimpleTableCell(
                SimpleTableCellPostcondition::new(request, before, after)?,
            ),
            PageMutation::ReplaceBlockTextAndConvert(_) => {
                Self::ReplaceBlockTextAndConvert(touched())
            }
            PageMutation::ReplaceTextSelection(_) => Self::ReplaceTextSelection(touched()),
            PageMutation::MergeBlocks(_) => Self::MergeBlocks(touched()),
            PageMutation::ConvertBlock(_) => Self::ConvertBlock(touched()),
            PageMutation::SetToDoState(_) => Self::SetToDoState(touched()),
            PageMutation::SetCodeLanguage(_) => Self::SetCodeLanguage(touched()),
            PageMutation::SetCodeWrap(_) => Self::SetCodeWrap(touched()),
            PageMutation::SetPageIcon(request) if request.block_id() == before.block_id => {
                Self::SetPageIcon(PagePostcondition::RetrySafe)
            }
            PageMutation::SetPageIcon(_) => Self::SetPageIcon(touched()),
            PageMutation::SetBlockColor(_) => Self::SetBlockColor(touched()),
            PageMutation::SetQuoteSize(_) => Self::SetQuoteSize(touched()),
            PageMutation::ResizeColumns(_) => Self::ResizeColumns(touched()),
            PageMutation::ReorderSubtrees(_) => Self::ReorderSubtrees(touched()),
            PageMutation::InsertMention(_) => Self::InsertMention(touched()),
            PageMutation::UpdateMention(_) => Self::UpdateMention(touched()),
        })
    }

    fn classify(&self, authority: &CardPage) -> FailedPageWriteDisposition {
        match self {
            Self::CreateBlock(value)
            | Self::CreateCodeBlock(value)
            | Self::DuplicateAlias(value)
            | Self::ReplaceBlockWithCode(value)
            | Self::RestoreBlockFromCode(value)
            | Self::ReplaceBlockWithDivider(value)
            | Self::ConvertBlockToDivider(value)
            | Self::RestoreBlockFromDivider(value)
            | Self::ReplaceBlockText(value)
            | Self::ReplaceBlockTextAndConvert(value)
            | Self::ReplaceTextSelection(value)
            | Self::BreakTextSelection(value)
            | Self::PasteTextSelection(value)
            | Self::SplitBlock(value)
            | Self::MergeBlocks(value)
            | Self::ConvertBlock(value)
            | Self::SetToDoState(value)
            | Self::SetCodeLanguage(value)
            | Self::SetCodeWrap(value)
            | Self::SetPageIcon(value)
            | Self::SetBlockColor(value)
            | Self::SetQuoteSize(value)
            | Self::ResizeColumns(value)
            | Self::DeleteBlock(value)
            | Self::ReorderSubtrees(value)
            | Self::InsertMention(value)
            | Self::UpdateMention(value) => value.classify(authority),
            Self::ReplaceSimpleTableCell(value) => value.classify(authority),
        }
    }
}

fn deleted(block_id: &str) -> PagePostcondition {
    PagePostcondition::Delete(DeleteBlockPostcondition::new(block_id))
}

fn fixed(
    write: &PageWrite,
    block_id: &str,
    after: &CardPage,
) -> Result<PagePostcondition, PageWriteReplayError> {
    Ok(PagePostcondition::Fixed(FixedBlockPostcondition::new(
        block_id,
        after,
        fixed_write_order(&write.operation, after),
    )?))
}
