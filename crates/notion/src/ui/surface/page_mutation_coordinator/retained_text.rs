use std::collections::{HashMap, HashSet};

use super::PageMutationRunToken;
use crate::model::{
    CardPage, CardPageEditableBlock, CardPageSimpleTableCell, CardPageSimpleTableCellAddress,
    CardPageSimpleTableCellIndex, CardPageSimpleTableCellRoundTrip,
    CardPageWritableSimpleTableCell,
};
use crate::ui::board_workspace::{
    PageProjectedText, PageTextProjectionTarget, PageWriteTextProjection,
};

#[derive(Clone, Debug)]
struct PageCommittedText {
    token: PageMutationRunToken,
    projection: PageProjectedText,
}

#[derive(Default)]
pub(super) struct PageCommittedTextOverlays {
    title: Option<PageCommittedText>,
    blocks: HashMap<String, PageCommittedText>,
    cells: HashMap<CardPageSimpleTableCellAddress, PageCommittedText>,
}

impl PageCommittedTextOverlays {
    pub(super) fn promote(
        &mut self,
        token: PageMutationRunToken,
        projection: PageWriteTextProjection,
    ) {
        let retirements = projection
            .retirements
            .into_iter()
            .map(|retirement| retirement.into_block_id())
            .collect::<HashSet<_>>();
        for block_id in &retirements {
            self.blocks.remove(block_id);
        }
        self.cells.retain(|target, _| {
            !retirements.contains(target.table_block_id())
                && !retirements.contains(target.row_block_id())
        });
        for update in projection.updates {
            self.promote_update(token, update);
        }
    }

    pub(super) fn overlay(&mut self, page: &mut CardPage, authority: Option<PageMutationRunToken>) {
        self.reconcile_title(&mut page.title, authority);
        for block in &mut page.blocks {
            let Some(overlay) = self.blocks.get(&block.block_id) else {
                continue;
            };
            let Some(editable) = block.editable_content_mut() else {
                continue;
            };
            if authority_acknowledges(authority, overlay) && overlay.matches_editable(editable) {
                self.blocks.remove(&block.block_id);
                continue;
            }
            editable.text.clone_from(&overlay.projection.text);
            editable
                .annotations
                .clone_from(&overlay.projection.annotations);
        }
        self.reconcile_cells(page, authority);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.title.is_none() && self.blocks.is_empty() && self.cells.is_empty()
    }

    fn promote_update(&mut self, token: PageMutationRunToken, update: PageProjectedText) {
        match update.target.clone() {
            PageTextProjectionTarget::Title => {
                self.title = Some(PageCommittedText {
                    token,
                    projection: update,
                });
            }
            PageTextProjectionTarget::Block(block_id) => {
                self.blocks.insert(
                    block_id,
                    PageCommittedText {
                        token,
                        projection: update,
                    },
                );
            }
            PageTextProjectionTarget::SimpleTableCell(target) => {
                self.cells.insert(
                    target,
                    PageCommittedText {
                        token,
                        projection: update,
                    },
                );
            }
        }
    }

    fn reconcile_cells(&mut self, page: &mut CardPage, authority: Option<PageMutationRunToken>) {
        if self.cells.is_empty() {
            return;
        }
        let index = CardPageSimpleTableCellIndex::new(page)
            .expect("loaded page hierarchy must remain indexable for committed cell text");
        let mut acknowledged = Vec::new();
        for (target, overlay) in &self.cells {
            let cell = match index.cell(page, target) {
                Ok(cell) => cell,
                Err(_) => continue,
            };
            if authority_acknowledges(authority, overlay) && overlay.matches_cell(cell) {
                acknowledged.push(target.clone());
                continue;
            }
            if !matches!(
                cell.round_trip(),
                CardPageSimpleTableCellRoundTrip::Writable
            ) {
                continue;
            }
            let replacement = CardPageWritableSimpleTableCell::new(
                overlay.projection.text.clone(),
                overlay.projection.annotations.clone(),
            )
            .expect("committed cell projection must retain writable rich text");
            index
                .replace(page, target, replacement)
                .expect("resolved committed cell projection must remain replaceable");
        }
        for target in acknowledged {
            self.cells.remove(&target);
        }
    }

    fn reconcile_title(&mut self, title: &mut String, authority: Option<PageMutationRunToken>) {
        let Some(overlay) = self.title.as_ref() else {
            return;
        };
        if authority_acknowledges(authority, overlay) && title == &overlay.projection.text {
            self.title = None;
            return;
        }
        title.clone_from(&overlay.projection.text);
    }
}

impl PageCommittedText {
    fn matches_editable(&self, editable: &CardPageEditableBlock) -> bool {
        self.projection.matches_editable(editable)
    }

    fn matches_cell(&self, cell: &CardPageSimpleTableCell) -> bool {
        self.projection.matches_simple_table_cell(cell)
    }
}

fn authority_acknowledges(
    authority: Option<PageMutationRunToken>,
    overlay: &PageCommittedText,
) -> bool {
    authority.is_some_and(|authority| {
        authority.epoch == overlay.token.epoch && authority.write_id >= overlay.token.write_id
    })
}
