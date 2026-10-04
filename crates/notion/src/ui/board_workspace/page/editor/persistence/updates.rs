use super::PageMutationPlan;

use super::{
    super::rich_text::PageProjectedText,
    block_shape::VerifiedPageTextBlockKind,
    property_groups::{page_block_persists_color, PageBlockPropertyGroups},
    snapshot::PageSnapshotIndex,
};
use crate::model::{
    CardPage, CardPageBlock, CardPageEditableBlock, CardPageSimpleTableBlock,
    CardPageSimpleTableCellAddress, CardPageSimpleTableCellIndex, PageMutation, PageShellIcon,
    ReorderPageBlockSubtreesRequest, ReplacePageSimpleTableCellRequest, SetPageCodeLanguageRequest,
    SetPageCodeWrapRequest, SetPageIconRequest, SetPageToDoStateRequest,
};
use crate::ui::CardPageBlockKind;

pub(super) struct PageBlockAnnotationTransition<'a> {
    pub(super) page_id: &'a str,
    pub(super) block: &'a CardPageBlock,
    pub(super) current: &'a CardPageEditableBlock,
    pub(super) text_changed: bool,
}

struct PageBlockUpdate<'a> {
    page_id: &'a str,
    current: &'a CardPageBlock,
    target: &'a CardPageBlock,
}

struct SimpleTableCellHistory<'a> {
    current: &'a CardPage,
    target: &'a CardPage,
    current_cells: &'a CardPageSimpleTableCellIndex,
    target_cells: &'a CardPageSimpleTableCellIndex,
}

#[derive(Clone, Copy)]
struct EditablePageBlockUpdate<'a> {
    page_id: &'a str,
    block: &'a CardPageBlock,
    current: &'a CardPageEditableBlock,
    target: &'a CardPageEditableBlock,
}

impl PageMutationPlan {
    pub(super) fn persist_target_page_block_order(
        &mut self,
        target: &CardPage,
        requests: Vec<ReorderPageBlockSubtreesRequest>,
    ) {
        for request in requests {
            self.enqueue_page_mutation(&target.block_id, PageMutation::ReorderSubtrees(request));
        }
    }

    pub(super) fn persist_updated_page_blocks(
        &mut self,
        index: &PageSnapshotIndex<'_>,
        target: &CardPage,
    ) {
        let mut groups = PageBlockPropertyGroups::default();
        for block in &target.blocks {
            let Some(current) = index.current_block(&block.block_id) else {
                continue;
            };
            self.persist_page_block_update(
                PageBlockUpdate {
                    page_id: &target.block_id,
                    current,
                    target: block,
                },
                &mut groups,
            );
        }
        self.persist_simple_table_cell_updates(index, target);
        groups.persist(self, &target.block_id);
    }

    fn persist_simple_table_cell_updates(
        &mut self,
        snapshots: &PageSnapshotIndex<'_>,
        target: &CardPage,
    ) {
        let current = snapshots.current_page();
        let current_cells = CardPageSimpleTableCellIndex::new(current)
            .expect("recorded page history must retain a valid current table hierarchy");
        let target_cells = CardPageSimpleTableCellIndex::new(target)
            .expect("recorded page history must retain a valid target table hierarchy");
        let history = SimpleTableCellHistory {
            current,
            target,
            current_cells: &current_cells,
            target_cells: &target_cells,
        };
        for table_block in &target.blocks {
            let Some(table) = table_block.simple_table_content() else {
                continue;
            };
            let Some(current_block) = snapshots.current_block(&table_block.block_id) else {
                continue;
            };
            let current_table = current_block.simple_table_content().unwrap_or_else(|| {
                panic!(
                    "recorded block {} changed into a simple-table root without a supported mutation",
                    table_block.block_id
                )
            });
            assert_same_simple_table_shape(&table_block.block_id, current_table, table);
            for row_id in table.row_block_ids() {
                let Some(target_row) = snapshots.target_block(row_id) else {
                    continue;
                };
                if target_row.simple_table_row_content().is_none() {
                    continue;
                }
                for column in table.columns() {
                    self.persist_simple_table_cell_update(
                        &history,
                        CardPageSimpleTableCellAddress::new(
                            table_block.block_id.clone(),
                            row_id.clone(),
                            column.id().clone(),
                        )
                        .expect("recorded table history must retain typed cell addresses"),
                    );
                }
            }
        }
    }

    fn persist_simple_table_cell_update(
        &mut self,
        history: &SimpleTableCellHistory<'_>,
        address: CardPageSimpleTableCellAddress,
    ) {
        let SimpleTableCellHistory {
            current,
            target,
            current_cells,
            target_cells,
        } = *history;
        let current_cell = current_cells
            .cell(current, &address)
            .expect("recorded current table cell must resolve through its root schema");
        let target_cell = target_cells
            .cell(target, &address)
            .expect("recorded target table cell must resolve through its root schema");
        if current_cell == target_cell {
            return;
        }
        let request =
            ReplacePageSimpleTableCellRequest::new(current, address.clone(), target_cell.clone())
                .expect("recorded table-cell history must remain round-trip writable");
        let projected = PageProjectedText::simple_table_cell(address, request.cell())
            .expect("recorded target table-cell annotations must remain canonicalizable");
        if projected.matches_simple_table_cell(current_cell) {
            return;
        }
        self.enqueue_page_mutation(
            &target.block_id,
            PageMutation::ReplaceSimpleTableCell(request),
        );
    }

    fn persist_page_block_update(
        &mut self,
        update: PageBlockUpdate<'_>,
        groups: &mut PageBlockPropertyGroups,
    ) {
        let icon_changed = !PageShellIcon::persisted_value_eq(
            update.current.icon.as_ref(),
            update.target.icon.as_ref(),
        ) && update.current.editable_content().is_some_and(|editable| {
            matches!(
                editable.kind,
                CardPageBlockKind::PageLink | CardPageBlockKind::Callout
            )
        }) && update.target.editable_content().is_some_and(|editable| {
            matches!(
                editable.kind,
                CardPageBlockKind::PageLink | CardPageBlockKind::Callout
            )
        });
        if icon_changed {
            let request =
                SetPageIconRequest::new(update.target.block_id.clone(), update.target.icon.clone())
                    .expect("page icon transitions must contain a valid icon");
            self.enqueue_page_mutation(update.page_id, PageMutation::SetPageIcon(request));
        }
        if update.current.color != update.target.color && page_block_persists_color(update.target) {
            groups.add_color(update.target.color, update.target.block_id.clone());
        }
        let (Some(current), Some(target)) = (
            update.current.editable_content(),
            update.target.editable_content(),
        ) else {
            return;
        };
        self.persist_editable_page_block_update(
            EditablePageBlockUpdate {
                page_id: update.page_id,
                block: update.target,
                current,
                target,
            },
            groups,
        );
    }

    fn persist_editable_page_block_update(
        &mut self,
        update: EditablePageBlockUpdate<'_>,
        groups: &mut PageBlockPropertyGroups,
    ) {
        let text_changed = update.current.text != update.target.text;
        self.persist_editable_text_kind_and_todo(update, text_changed);
        self.persist_editable_quote_and_code(update, groups);
        self.persist_page_block_annotation_transition(PageBlockAnnotationTransition {
            page_id: update.page_id,
            block: update.block,
            current: update.current,
            text_changed,
        });
    }

    fn persist_editable_text_kind_and_todo(
        &mut self,
        update: EditablePageBlockUpdate<'_>,
        text_changed: bool,
    ) {
        if text_changed {
            self.persist_page_block_text(
                update.page_id,
                &update.block.block_id,
                update.target.text.clone(),
            );
        }
        if update.current.kind != update.target.kind {
            VerifiedPageTextBlockKind::parse(update.current.kind)
                .expect("recorded block-kind history must start from verified text");
            let target = VerifiedPageTextBlockKind::parse(update.target.kind)
                .expect("recorded block-kind history must target verified text");
            self.enqueue_page_mutation(
                update.page_id,
                PageMutation::ConvertBlock(
                    target.conversion_request(update.block.block_id.clone()),
                ),
            );
        }
        if let Some(target_state) = update.target.to_do_state() {
            let current_state = update.current.to_do_state();
            if current_state != Some(target_state)
                && (target_state.is_checked() || current_state.is_some())
            {
                self.enqueue_page_mutation(
                    update.page_id,
                    PageMutation::SetToDoState(SetPageToDoStateRequest {
                        block_id: update.block.block_id.clone(),
                        state: target_state,
                    }),
                );
            }
        }
    }

    fn persist_editable_quote_and_code(
        &mut self,
        update: EditablePageBlockUpdate<'_>,
        groups: &mut PageBlockPropertyGroups,
    ) {
        if let Some(target_size) = update.target.quote_size() {
            if update.current.quote_size() != Some(target_size) {
                groups.add_quote_size(target_size, update.block.block_id.clone());
            }
        }
        if let Some(target_language) = update.target.code_language() {
            if update.current.code_language() != Some(target_language) {
                let request = SetPageCodeLanguageRequest::new(
                    update.block.block_id.clone(),
                    target_language.clone(),
                )
                .expect("persisted code-language targets must contain a block");
                self.enqueue_page_mutation(update.page_id, PageMutation::SetCodeLanguage(request));
            }
        }
        if let Some(target_wrap) = update.target.code_wrap() {
            if update.current.code_wrap() != Some(target_wrap) {
                let request =
                    SetPageCodeWrapRequest::new(update.block.block_id.clone(), target_wrap)
                        .expect("persisted code-wrap targets must contain a block");
                self.enqueue_page_mutation(update.page_id, PageMutation::SetCodeWrap(request));
            }
        }
    }
}

fn assert_same_simple_table_shape(
    table_block_id: &str,
    current: &CardPageSimpleTableBlock,
    target: &CardPageSimpleTableBlock,
) {
    assert!(
        current
            .columns()
            .iter()
            .map(|column| column.id())
            .eq(target.columns().iter().map(|column| column.id())),
        "recorded table {table_block_id} changed columns without a supported mutation"
    );
    assert_eq!(
        current.row_block_ids(),
        target.row_block_ids(),
        "recorded table {table_block_id} changed rows without a supported mutation"
    );
}
