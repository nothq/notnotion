use std::collections::VecDeque;

use crate::model::{EditPageBlockTextRequest, PageMutation, PageTextEditTarget};
use crate::ui::board_workspace::{PageWrite, PageWriteOperation};
use crate::ui::{Arc, NotionWorkspaceApi};

use super::{PageMutationCoordinator, PageMutationExecution, PageMutationQueueHead};

mod coalescing;
mod column_resize;

use coalescing::remove_pending_simple_table_cell_writes;
use column_resize::coalesce_pending_column_resize;

impl PageMutationCoordinator {
    pub(crate) fn enqueue_write(
        &mut self,
        page_id: &str,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
        first_write_baseline: Option<Arc<crate::model::CardPage>>,
        write: PageWrite,
    ) {
        let remove_lane = {
            let lane = self.lanes.entry(page_id.to_string()).or_default();
            lane.bind_workspace_api(workspace_api);
            lane.bind_first_write_baseline(first_write_baseline);
            let head = match lane.execution {
                PageMutationExecution::Writing(_) => PageMutationQueueHead::InFlight,
                PageMutationExecution::Idle | PageMutationExecution::Recovering(_) => {
                    PageMutationQueueHead::Pending
                }
            };
            enqueue_page_write(&mut lane.queue, write, head);
            lane.clear_cancelled_idle_queue_state();
            lane.can_remove()
        };
        if remove_lane {
            self.lanes.remove(page_id);
        }
    }
}

fn enqueue_page_write(
    queue: &mut VecDeque<PageWrite>,
    write: PageWrite,
    queue_head: PageMutationQueueHead,
) {
    match &write.operation {
        PageWriteOperation::Mutation(_) => enqueue_page_mutation_write(queue, write, queue_head),
        PageWriteOperation::RichText(_) => enqueue_page_rich_text_write(queue, write, queue_head),
    }
}

fn enqueue_page_mutation_write(
    queue: &mut VecDeque<PageWrite>,
    write: PageWrite,
    queue_head: PageMutationQueueHead,
) {
    if coalesce_pending_column_resize(queue, &write, queue_head) {
        return;
    }
    if remove_pending_simple_table_cell_writes(queue, &write, queue_head) {
        queue.push_back(write);
        return;
    }
    if queue_head.tail_is_pending(queue.len()) && coalesce_last_mutation(queue, &write) {
        return;
    }
    queue.push_back(write);
}

fn coalesce_last_mutation(queue: &mut VecDeque<PageWrite>, write: &PageWrite) -> bool {
    match (queue.back_mut(), write) {
        (
            Some(PageWrite {
                operation: PageWriteOperation::Mutation(PageMutation::SetPageIcon(pending)),
                text_projection,
            }),
            PageWrite {
                operation: PageWriteOperation::Mutation(PageMutation::SetPageIcon(next)),
                text_projection: next_projection,
            },
        ) if pending.block_id() == next.block_id() => {
            *pending = next.clone();
            text_projection.clone_from(next_projection);
            true
        }
        (
            Some(PageWrite {
                operation: PageWriteOperation::Mutation(PageMutation::ReplaceBlockText(pending)),
                text_projection,
            }),
            PageWrite {
                operation: PageWriteOperation::Mutation(PageMutation::ReplaceBlockText(next)),
                text_projection: next_projection,
            },
        ) if pending.block_id == next.block_id => {
            pending.text.clone_from(&next.text);
            text_projection.clone_from(next_projection);
            true
        }
        _ => false,
    }
}

fn enqueue_page_rich_text_write(
    queue: &mut VecDeque<PageWrite>,
    write: PageWrite,
    queue_head: PageMutationQueueHead,
) {
    if queue_head.tail_is_pending(queue.len()) && coalesce_last_rich_text(queue, &write) {
        return;
    }
    queue.push_back(write);
}

fn coalesce_last_rich_text(queue: &mut VecDeque<PageWrite>, write: &PageWrite) -> bool {
    let (
        Some(PageWrite {
            operation: PageWriteOperation::RichText(pending),
            text_projection,
        }),
        PageWrite {
            operation: PageWriteOperation::RichText(next),
            text_projection: next_projection,
        },
    ) = (queue.back_mut(), write)
    else {
        return false;
    };
    let Some(merged) = merge_adjacent_page_rich_text_typing(pending, next) else {
        return false;
    };
    *pending = merged;
    text_projection.clone_from(next_projection);
    true
}

fn merge_adjacent_page_rich_text_typing(
    pending: &EditPageBlockTextRequest,
    next: &EditPageBlockTextRequest,
) -> Option<EditPageBlockTextRequest> {
    if pending.page_block_id != next.page_block_id
        || pending.annotation_removals() != next.annotation_removals()
        || pending.annotation_additions() != next.annotation_additions()
    {
        return None;
    }
    let [PageTextEditTarget::Typing {
        block_id: pending_block_id,
        offset_utf8: pending_offset,
        text: pending_text,
    }] = pending.targets()
    else {
        return None;
    };
    let [PageTextEditTarget::Typing {
        block_id: next_block_id,
        offset_utf8: next_offset,
        text: next_text,
    }] = next.targets()
    else {
        return None;
    };
    if pending_block_id != next_block_id
        || pending_offset.checked_add(pending_text.len()) != Some(*next_offset)
    {
        return None;
    }
    let mut text = pending_text.clone();
    text.push_str(next_text);
    Some(
        EditPageBlockTextRequest::new(
            pending.page_block_id.clone(),
            vec![PageTextEditTarget::Typing {
                block_id: pending_block_id.clone(),
                offset_utf8: *pending_offset,
                text,
            }],
            pending.annotation_removals().to_vec(),
            pending.annotation_additions().to_vec(),
        )
        .expect("adjacent page typing requests must remain valid when merged"),
    )
}
