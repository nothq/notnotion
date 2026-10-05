use std::collections::VecDeque;

use crate::model::PageMutation;
use crate::ui::board_workspace::{PageWrite, PageWriteOperation};

use super::super::PageMutationQueueHead;

pub(super) fn remove_pending_simple_table_cell_writes(
    queue: &mut VecDeque<PageWrite>,
    next: &PageWrite,
    queue_head: PageMutationQueueHead,
) -> bool {
    let PageWriteOperation::Mutation(PageMutation::ReplaceSimpleTableCell(next)) = &next.operation
    else {
        return false;
    };
    let mut index = match queue_head {
        PageMutationQueueHead::InFlight => 1,
        PageMutationQueueHead::Pending => 0,
    };
    let mut removed = false;
    while index < queue.len() {
        let same_target = matches!(
            &queue[index].operation,
            PageWriteOperation::Mutation(PageMutation::ReplaceSimpleTableCell(pending))
                if pending.target() == next.target()
        );
        if same_target {
            queue.remove(index);
            removed = true;
        } else {
            index += 1;
        }
    }
    removed
}
