use super::super::page_state::{NotionBlockRecord, PageMutationState};
use super::{
    blocks::{
        build_duplicate_alias, convert_block_operations, create_block_operations,
        delete_block_operations, editable_block, lift_children_before_source, metadata_operation,
        move_children_operations, reorder_operations, require_mergeable_block,
        require_splittable_block, validate_block_conversion, CreateBlockInput, MutationIdentity,
    },
    single::{
        build_convert, build_convert_block_to_divider, build_create, build_create_code,
        build_replace_block_with_code, build_replace_block_with_divider, build_replace_text,
        build_restore_block_from_code, build_restore_block_from_divider,
    },
    text::{
        build_insert_mention, build_update_mention, merge_operations, replacement_operations,
        split_operations,
    },
    text_selection, validated_new_block_id,
    wire::{
        BlockColorArgs, BlockPropertyPrimitiveArgs, BlockPropertyPrimitiveOperation,
        BlockPropertyValueArgs, CodeLanguagePropertyArgs, CodeWrapArgs, QuoteSizeArgs,
        RecordPointer, SaveOperation, ToDoCheckedPropertyArgs,
    },
    BuiltPageMutation, CrdtClock,
};
use crate::model::{
    MergePageBlockChildrenEffect, MergePageBlocksEffect, MergePageBlocksRequest,
    NotionPageBlockKind, PageBlockPlacement, PageMutation, PageMutationEffect,
    ReorderPageBlockSubtreesRequest, ReplacePageBlockTextAndConvertRequest,
    SetPageBlockColorRequest, SetPageCodeLanguageRequest, SetPageCodeWrapRequest,
    SetPageIconRequest, SetPageQuoteSizeRequest, SetPageToDoStateRequest, SplitPageBlockEffect,
    SplitPageBlockRequest,
};

mod columns;
mod properties;
mod structure;
mod table;

use columns::build_resize_columns;
use properties::{
    build_set_block_color, build_set_code_language, build_set_code_wrap, build_set_page_icon,
    build_set_quote_size, build_set_to_do_state,
};
use structure::{build_merge, build_reorder, build_replace_text_and_convert, build_split};
use table::build_replace_simple_table_cell;

const REPLACEMENT_BATCH: &str = "text-selection replacement batches";

pub(super) fn build_mutation(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    mutation: &PageMutation,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    match mutation {
        PageMutation::ReplaceTextSelection(request) => {
            text_selection::build_text_selection_replacement(state, identity, request, clock)
        }
        PageMutation::BreakTextSelection(request) => {
            if let crate::model::PageTextLineBreak::Enter { new_block_id, .. } =
                request.line_break()
            {
                validated_new_block_id(state, new_block_id)?;
            }
            text_selection::build_text_selection_line_break(state, identity, request, clock)
        }
        PageMutation::PasteTextSelection(request) => {
            validated_new_block_id(state, request.new_block_id())?;
            text_selection::build_multiline_text_selection_paste(state, identity, request, clock)
        }
        PageMutation::ResizeColumns(request) => build_resize_columns(state, identity, request),
        _ => build_single_mutation(state, identity, mutation, clock),
    }
}

pub(super) fn build_single_mutation(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    mutation: &PageMutation,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    match mutation {
        PageMutation::CreateBlock(request) => build_create(state, identity, request, clock),
        PageMutation::CreateCodeBlock(request) => build_create_code(state, identity, request),
        PageMutation::DuplicateAlias(request) => build_duplicate_alias(state, identity, request),
        PageMutation::ReplaceBlockWithCode(request) => {
            build_replace_block_with_code(state, identity, request)
        }
        PageMutation::RestoreBlockFromCode(request) => {
            build_restore_block_from_code(state, identity, request, clock)
        }
        PageMutation::ReplaceBlockWithDivider(request) => {
            build_replace_block_with_divider(state, identity, request, clock)
        }
        PageMutation::ConvertBlockToDivider(request) => {
            build_convert_block_to_divider(state, identity, request, clock)
        }
        PageMutation::RestoreBlockFromDivider(request) => {
            build_restore_block_from_divider(state, identity, request, clock)
        }
        PageMutation::ReplaceBlockText(request) => {
            build_replace_text(state, identity, request, clock)
        }
        PageMutation::ReplaceSimpleTableCell(request) => {
            build_replace_simple_table_cell(state, identity, request)
        }
        PageMutation::ReplaceBlockTextAndConvert(request) => {
            build_replace_text_and_convert(state, identity, request, clock)
        }
        PageMutation::ReplaceTextSelection(_) => nested_batch_error(REPLACEMENT_BATCH),
        PageMutation::BreakTextSelection(_) => {
            nested_batch_error("text-selection line-break batches")
        }
        PageMutation::PasteTextSelection(_) => nested_batch_error("text-selection paste batches"),
        PageMutation::ResizeColumns(_) => nested_batch_error("column resize mutations"),
        PageMutation::SplitBlock(request) => build_split(state, identity, request, clock),
        PageMutation::MergeBlocks(request) => build_merge(state, identity, request, clock),
        PageMutation::ConvertBlock(request) => build_convert(state, identity, request),
        PageMutation::SetToDoState(request) => build_set_to_do_state(state, identity, request),
        PageMutation::SetCodeLanguage(request) => build_set_code_language(state, identity, request),
        PageMutation::SetCodeWrap(request) => build_set_code_wrap(state, identity, request),
        PageMutation::SetPageIcon(request) => build_set_page_icon(state, identity, request),
        PageMutation::SetBlockColor(request) => build_set_block_color(state, identity, request),
        PageMutation::SetQuoteSize(request) => build_set_quote_size(state, identity, request),
        PageMutation::DeleteBlock(request) => build_delete(state, identity, &request.block_id),
        PageMutation::ReorderSubtrees(request) => build_reorder(state, identity, request),
        PageMutation::InsertMention(request) => {
            build_insert_mention(state, identity, request, clock)
        }
        PageMutation::UpdateMention(request) => {
            build_update_mention(state, identity, request, clock)
        }
    }
}

fn build_delete(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    block_id: &str,
) -> Result<BuiltPageMutation, String> {
    Ok(BuiltPageMutation {
        operations: delete_block_operations(state, identity, block_id)?,
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "blockActions.removeBlock",
    })
}

fn nested_batch_error(kind: &str) -> Result<BuiltPageMutation, String> {
    Err(format!("{kind} cannot be nested"))
}
