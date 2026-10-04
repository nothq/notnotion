use std::collections::VecDeque;
use std::fmt::{Display, Formatter};

use super::super::super::rich_text::{
    PageProjectedText, PageTextProjectionTarget, PageWrite, PageWriteOperation,
    PageWriteTextProjection,
};
use crate::model::{
    CardPage, CardPageSimpleTableCellIndex, CardPageWritableSimpleTableCell, NotionPageBlockKind,
};
use crate::ui::surface::PageMutationCoordinator;

mod block_mutation;
mod column_ratio;
mod hierarchy;
mod mention;
mod mutation;
mod postcondition;
mod selection;
mod text;
mod text_offset;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ReplayPageSide {
    QueueBaseline,
    Authority,
}

#[derive(Debug)]
pub(crate) enum PageWriteReplayError {
    PageIdentity {
        baseline_id: String,
        authority_id: String,
    },
    RequestPageIdentity {
        expected_id: String,
        request_id: String,
    },
    MissingBlock {
        operation: &'static str,
        block_id: String,
        side: ReplayPageSide,
    },
    InvalidTextTarget {
        operation: &'static str,
        block_id: String,
        side: ReplayPageSide,
    },
    InvalidTextOffset {
        block_id: String,
        offset: usize,
        text_len: usize,
        encoding: &'static str,
    },
    HierarchyConflict {
        operation: &'static str,
        detail: String,
    },
    UnsupportedBlockKind {
        operation: &'static str,
        kind: NotionPageBlockKind,
    },
    InvalidRequest {
        operation: &'static str,
        detail: String,
    },
}

pub(super) struct RebasedPageWrites {
    pub(super) queue: VecDeque<PageWrite>,
    pub(super) optimistic_page: CardPage,
}

pub(super) struct PageWriteProjector {
    pub(super) local: CardPage,
    pub(super) authority: CardPage,
}

impl PageMutationCoordinator {
    pub(crate) fn rebase_queued_page_writes(
        &mut self,
        baseline: &CardPage,
        authority: &CardPage,
    ) -> Result<Option<CardPage>, PageWriteReplayError> {
        let Some(queue) = self.pending_writes_mut(&baseline.block_id) else {
            return Ok(None);
        };
        if queue.is_empty() {
            return Ok(None);
        }
        let rebased = rebase_page_write_queue(baseline, authority, queue)?;
        *queue = rebased.queue;
        Ok(Some(rebased.optimistic_page))
    }
}

pub(super) fn rebase_page_write_queue(
    baseline: &CardPage,
    authority: &CardPage,
    queue: &VecDeque<PageWrite>,
) -> Result<RebasedPageWrites, PageWriteReplayError> {
    let mut projector = PageWriteProjector::new(baseline, authority)?;
    let mut rebased = queue.clone();
    for write in &mut rebased {
        projector.apply_write(write)?;
    }
    projector
        .authority
        .validate_block_hierarchy()
        .map_err(|detail| PageWriteReplayError::HierarchyConflict {
            operation: "queued page replay",
            detail,
        })?;
    Ok(RebasedPageWrites {
        queue: rebased,
        optimistic_page: projector.authority,
    })
}

pub(super) fn project_page_write(
    baseline: &CardPage,
    write: &PageWrite,
) -> Result<CardPage, PageWriteReplayError> {
    project_page_write_operation(baseline, &write.operation)
}

/// Applies one page write to `baseline` the way the editor projects it, for
/// backends that keep their own page authority.
fn project_page_write_operation(
    baseline: &CardPage,
    operation: &PageWriteOperation,
) -> Result<CardPage, PageWriteReplayError> {
    let mut projected = baseline.clone();
    match operation {
        PageWriteOperation::RichText(request) => {
            text::apply_rich_text_request(&mut projected, request)?
        }
        PageWriteOperation::Mutation(mutation) => {
            mutation::apply_page_mutation(&mut projected, mutation)?
        }
    }
    projected.validate_block_hierarchy().map_err(|detail| {
        PageWriteReplayError::HierarchyConflict {
            operation: "page write projection",
            detail,
        }
    })?;
    Ok(projected)
}

pub(super) use postcondition::{classify_failed_page_write, FailedPageWriteDisposition};

impl PageWriteProjector {
    fn new(baseline: &CardPage, authority: &CardPage) -> Result<Self, PageWriteReplayError> {
        if baseline.block_id != authority.block_id {
            return Err(PageWriteReplayError::PageIdentity {
                baseline_id: baseline.block_id.clone(),
                authority_id: authority.block_id.clone(),
            });
        }
        Ok(Self {
            local: baseline.clone(),
            authority: authority.clone(),
        })
    }

    fn apply_write(&mut self, write: &mut PageWrite) -> Result<(), PageWriteReplayError> {
        match &mut write.operation {
            PageWriteOperation::RichText(request) => self.rebase_rich_text_request(request)?,
            PageWriteOperation::Mutation(mutation) => self.rebase_and_apply_mutation(mutation)?,
        }
        self.refresh_text_projection(&mut write.text_projection)
    }

    fn refresh_text_projection(
        &self,
        projection: &mut PageWriteTextProjection,
    ) -> Result<(), PageWriteReplayError> {
        let cell_index = projection
            .updates
            .iter()
            .any(|update| matches!(&update.target, PageTextProjectionTarget::SimpleTableCell(_)))
            .then(|| CardPageSimpleTableCellIndex::new(&self.authority))
            .transpose()
            .map_err(|detail| PageWriteReplayError::invalid_request("text projection", detail))?;
        for update in &mut projection.updates {
            *update = match &update.target {
                PageTextProjectionTarget::Title => {
                    PageProjectedText::title(self.authority.title.clone())
                }
                PageTextProjectionTarget::Block(block_id) => {
                    let block = hierarchy::block(&self.authority, block_id, "text projection")?;
                    PageProjectedText::block(block).ok_or_else(|| {
                        PageWriteReplayError::InvalidTextTarget {
                            operation: "text projection",
                            block_id: block_id.clone(),
                            side: ReplayPageSide::Authority,
                        }
                    })?
                }
                PageTextProjectionTarget::SimpleTableCell(target) => {
                    let cell = cell_index
                        .as_ref()
                        .expect("cell text projection must build a table index")
                        .cell(&self.authority, target)
                        .map_err(|detail| {
                            PageWriteReplayError::invalid_request("text projection", detail)
                        })?;
                    let cell = CardPageWritableSimpleTableCell::try_from(cell.clone()).map_err(
                        |detail| PageWriteReplayError::conflict("text projection", detail),
                    )?;
                    PageProjectedText::simple_table_cell(target.clone(), &cell).ok_or_else(
                        || {
                            PageWriteReplayError::invalid_request(
                                "text projection",
                                "simple-table cell annotations are invalid",
                            )
                        },
                    )?
                }
            };
        }
        Ok(())
    }
}

impl PageWriteReplayError {
    pub(super) fn conflict(operation: &'static str, detail: impl Into<String>) -> Self {
        Self::HierarchyConflict {
            operation,
            detail: detail.into(),
        }
    }

    pub(super) fn invalid_request(operation: &'static str, detail: impl Into<String>) -> Self {
        Self::InvalidRequest {
            operation,
            detail: detail.into(),
        }
    }
}

impl Display for PageWriteReplayError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PageIdentity {
                baseline_id,
                authority_id,
            } => write!(
                formatter,
                "queue baseline page {baseline_id} does not match authority {authority_id}"
            ),
            Self::RequestPageIdentity {
                expected_id,
                request_id,
            } => write!(
                formatter,
                "page write targets {request_id}, expected lane {expected_id}"
            ),
            Self::MissingBlock {
                operation,
                block_id,
                side,
            } => write!(
                formatter,
                "{operation} target {block_id} is missing from {side:?}"
            ),
            Self::InvalidTextTarget {
                operation,
                block_id,
                side,
            } => write!(
                formatter,
                "{operation} target {block_id} is not editable text in {side:?}"
            ),
            Self::InvalidTextOffset {
                block_id,
                offset,
                text_len,
                encoding,
            } => write!(
                formatter,
                "{encoding} offset {offset} is invalid for block {block_id} with length {text_len}"
            ),
            Self::HierarchyConflict { operation, detail }
            | Self::InvalidRequest { operation, detail } => {
                write!(formatter, "{operation} cannot be replayed: {detail}")
            }
            Self::UnsupportedBlockKind { operation, kind } => {
                write!(formatter, "{operation} cannot project block kind {kind:?}")
            }
        }
    }
}
