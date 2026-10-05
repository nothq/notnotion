use std::collections::VecDeque;

use crate::model::{PageMutation, ResizePageColumnsRequest};
use crate::ui::board_workspace::{PageWrite, PageWriteOperation};

use super::super::PageMutationQueueHead;

pub(super) fn coalesce_pending_column_resize(
    queue: &mut VecDeque<PageWrite>,
    next_write: &PageWrite,
    queue_head: PageMutationQueueHead,
) -> bool {
    let PageWriteOperation::Mutation(PageMutation::ResizeColumns(next)) = &next_write.operation
    else {
        return false;
    };
    let pending_start = match queue_head {
        PageMutationQueueHead::InFlight => 1,
        PageMutationQueueHead::Pending => 0,
    };
    for index in (pending_start..queue.len()).rev() {
        let PageWriteOperation::Mutation(PageMutation::ResizeColumns(pending)) =
            &queue[index].operation
        else {
            if !write_is_disjoint_from_resize(&queue[index], next) {
                return false;
            }
            continue;
        };
        if pending.same_column_list(next) {
            if !pending.same_divider(next) {
                return false;
            }
            let Ok(coalesced) = pending.coalesced_with(next) else {
                return false;
            };
            if let Some(coalesced) = coalesced {
                let PageWriteOperation::Mutation(PageMutation::ResizeColumns(pending)) =
                    &mut queue[index].operation
                else {
                    unreachable!("matched pending column resize must remain a resize");
                };
                *pending = coalesced;
                queue[index]
                    .text_projection
                    .clone_from(&next_write.text_projection);
            } else {
                queue.remove(index);
            }
            return true;
        }
    }
    false
}

fn write_is_disjoint_from_resize(write: &PageWrite, resize: &ResizePageColumnsRequest) -> bool {
    match &write.operation {
        PageWriteOperation::RichText(request) => request
            .targets()
            .iter()
            .all(|target| !resize_references(resize, target.block_id())),
        PageWriteOperation::Mutation(mutation) => mutation_is_disjoint(mutation, resize),
    }
}

fn mutation_is_disjoint(mutation: &PageMutation, resize: &ResizePageColumnsRequest) -> bool {
    match mutation {
        PageMutation::CreateBlock(request) => ids_are_disjoint(
            resize,
            [request.block_id.as_str(), request.parent_block_id.as_str()],
        ),
        PageMutation::CreateCodeBlock(request) => {
            ids_are_disjoint(resize, [request.block_id(), request.parent_block_id()])
        }
        PageMutation::ReplaceBlockText(request) => !resize_references(resize, &request.block_id),
        PageMutation::ReplaceSimpleTableCell(request) => ids_are_disjoint(
            resize,
            [
                request.target().table_block_id(),
                request.target().row_block_id(),
            ],
        ),
        PageMutation::ReplaceBlockTextAndConvert(request) => {
            !resize_references(resize, &request.block_id)
        }
        PageMutation::ConvertBlock(request) => !resize_references(resize, &request.block_id),
        PageMutation::InsertMention(request) => !resize_references(resize, request.block_id()),
        PageMutation::UpdateMention(request) => !resize_references(resize, request.block_id()),
        PageMutation::SetToDoState(request) => !resize_references(resize, &request.block_id),
        PageMutation::SetCodeLanguage(request) => !resize_references(resize, request.block_id()),
        PageMutation::SetCodeWrap(request) => !resize_references(resize, request.block_id()),
        PageMutation::SetPageIcon(request) => !resize_references(resize, request.block_id()),
        PageMutation::SetBlockColor(request) => request
            .block_ids()
            .iter()
            .all(|block_id| !resize_references(resize, block_id)),
        PageMutation::SetQuoteSize(request) => request
            .block_ids()
            .iter()
            .all(|block_id| !resize_references(resize, block_id)),
        PageMutation::ResizeColumns(other) => !resize.same_column_list(other),
        PageMutation::DuplicateAlias(_)
        | PageMutation::ReplaceBlockWithCode(_)
        | PageMutation::RestoreBlockFromCode(_)
        | PageMutation::ReplaceBlockWithDivider(_)
        | PageMutation::ConvertBlockToDivider(_)
        | PageMutation::RestoreBlockFromDivider(_)
        | PageMutation::ReplaceTextSelection(_)
        | PageMutation::BreakTextSelection(_)
        | PageMutation::PasteTextSelection(_)
        | PageMutation::SplitBlock(_)
        | PageMutation::MergeBlocks(_)
        | PageMutation::DeleteBlock(_)
        | PageMutation::ReorderSubtrees(_) => false,
    }
}

fn ids_are_disjoint<'a>(
    resize: &ResizePageColumnsRequest,
    ids: impl IntoIterator<Item = &'a str>,
) -> bool {
    ids.into_iter()
        .all(|block_id| !resize_references(resize, block_id))
}

fn resize_references(resize: &ResizePageColumnsRequest, block_id: &str) -> bool {
    block_id == resize.target().column_list_block_id()
        || resize
            .source_columns()
            .iter()
            .any(|column| column.column_block_id() == block_id)
}
