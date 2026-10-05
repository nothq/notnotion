use std::collections::{HashMap, HashSet};

use crate::ui::{CardPage, CardPageBlock, CardPageBlockKind, CardPageLayoutBlock};

use super::super::LoadedCardPageVisibleRow;

pub(super) struct CanonicalProjection {
    pub(super) numbered_indices: Vec<usize>,
    pub(super) visible_rows: Vec<LoadedCardPageVisibleRow>,
    pub(super) callout_child_rows: Vec<Vec<usize>>,
    pub(super) visible_block_mask: Vec<bool>,
    pub(super) flow_visible_block_mask: Vec<bool>,
    pub(super) collapsed_hidden_owner_indices: Vec<Option<usize>>,
    pub(super) next_root_numbered_index: usize,
}

#[derive(Clone, Copy)]
struct CanonicalRowContext {
    nearest_visible_row_index: Option<usize>,
    column_block_index: Option<usize>,
    callout_container_row_index: Option<usize>,
    descendants_visible: bool,
    collapsed_toggle_block_index: Option<usize>,
}

impl Default for CanonicalRowContext {
    fn default() -> Self {
        Self {
            nearest_visible_row_index: None,
            column_block_index: None,
            callout_container_row_index: None,
            descendants_visible: true,
            collapsed_toggle_block_index: None,
        }
    }
}

struct CanonicalProjectionPass<'a> {
    page: &'a CardPage,
    expanded_toggle_ids: Option<&'a HashSet<String>>,
    visible_rows: Vec<LoadedCardPageVisibleRow>,
    callout_child_rows: Vec<Vec<usize>>,
    contexts: Vec<CanonicalRowContext>,
    preceding_block_indices: HashMap<&'a str, usize>,
    sibling_numbered_sequences: HashMap<&'a str, usize>,
    last_visible_sibling_rows: HashMap<&'a str, usize>,
    numbered_indices: Vec<usize>,
    visible_block_mask: Vec<bool>,
    flow_visible_block_mask: Vec<bool>,
    collapsed_hidden_owner_indices: Vec<Option<usize>>,
}

impl<'a> CanonicalProjectionPass<'a> {
    fn new(page: &'a CardPage, expanded_toggle_ids: Option<&'a HashSet<String>>) -> Self {
        let block_count = page.blocks.len();
        Self {
            page,
            expanded_toggle_ids,
            visible_rows: Vec::with_capacity(block_count),
            callout_child_rows: Vec::with_capacity(block_count),
            contexts: Vec::with_capacity(block_count),
            preceding_block_indices: HashMap::with_capacity(block_count),
            sibling_numbered_sequences: HashMap::new(),
            last_visible_sibling_rows: HashMap::new(),
            numbered_indices: vec![0; block_count],
            visible_block_mask: Vec::with_capacity(block_count),
            flow_visible_block_mask: Vec::with_capacity(block_count),
            collapsed_hidden_owner_indices: Vec::with_capacity(block_count),
        }
    }

    fn build(mut self) -> CanonicalProjection {
        let page = self.page;
        for (block_index, block) in page.blocks.iter().enumerate() {
            self.visit_block(block_index, block);
        }
        let next_root_numbered_index = self
            .sibling_numbered_sequences
            .get(self.page.block_id.as_str())
            .copied()
            .unwrap_or_default()
            + 1;
        CanonicalProjection {
            numbered_indices: self.numbered_indices,
            visible_rows: self.visible_rows,
            callout_child_rows: self.callout_child_rows,
            visible_block_mask: self.visible_block_mask,
            flow_visible_block_mask: self.flow_visible_block_mask,
            collapsed_hidden_owner_indices: self.collapsed_hidden_owner_indices,
            next_root_numbered_index,
        }
    }

    fn visit_block(&mut self, block_index: usize, block: &'a CardPageBlock) {
        let parent_context = self.parent_context(block);
        let hidden_owner_index = parent_context.collapsed_toggle_block_index;
        let flow_visible = parent_context.descendants_visible
            && !block.is_opaque_unavailable()
            && block.simple_table_row_content().is_none();
        let (context, block_visible) = if block.is_opaque_unavailable() {
            (parent_context, false)
        } else {
            match (block.simple_table_row_content(), block.layout_content()) {
                (Some(_), _) => (parent_context, false),
                (None, Some(CardPageLayoutBlock::Column { .. })) => {
                    (self.visit_column(block_index, block, parent_context), false)
                }
                (
                    None,
                    Some(CardPageLayoutBlock::ColumnList | CardPageLayoutBlock::Passthrough { .. }),
                ) => self.visit_layout_container(block, parent_context),
                (None, None) if parent_context.descendants_visible => (
                    self.visit_visible_block(block_index, block, parent_context),
                    true,
                ),
                (None, None) => (parent_context, false),
            }
        };
        self.visible_block_mask.push(block_visible);
        self.flow_visible_block_mask.push(flow_visible);
        self.collapsed_hidden_owner_indices.push(hidden_owner_index);
        self.preceding_block_indices
            .insert(block.block_id.as_str(), block_index);
        self.contexts.push(context);
    }

    fn parent_context(&self, block: &CardPageBlock) -> CanonicalRowContext {
        if block.parent_block_id == self.page.block_id {
            return CanonicalRowContext::default();
        }
        let parent_index = *self
            .preceding_block_indices
            .get(block.parent_block_id.as_str())
            .expect("page block hierarchy must be parent-closed before shaping visible rows");
        self.contexts[parent_index]
    }

    fn visit_column(
        &mut self,
        block_index: usize,
        block: &'a CardPageBlock,
        mut context: CanonicalRowContext,
    ) -> CanonicalRowContext {
        if context.descendants_visible {
            context.column_block_index = Some(block_index);
            self.reset_sibling_runs(block.parent_block_id.as_str());
        }
        context
    }

    fn visit_layout_container(
        &mut self,
        block: &'a CardPageBlock,
        context: CanonicalRowContext,
    ) -> (CanonicalRowContext, bool) {
        if context.descendants_visible {
            self.reset_sibling_runs(block.parent_block_id.as_str());
        }
        (context, false)
    }

    fn reset_sibling_runs(&mut self, parent_block_id: &'a str) {
        self.sibling_numbered_sequences.insert(parent_block_id, 0);
        self.last_visible_sibling_rows.remove(parent_block_id);
    }

    fn visit_visible_block(
        &mut self,
        block_index: usize,
        block: &'a CardPageBlock,
        mut context: CanonicalRowContext,
    ) -> CanonicalRowContext {
        let row_index = self.visible_rows.len();
        let joins_previous_list_sibling = self.joins_previous_list_sibling(block);
        let row = self.visible_row(block_index, context, joins_previous_list_sibling);
        self.visible_rows.push(row);
        self.callout_child_rows.push(Vec::new());
        self.record_callout_child(context.callout_container_row_index, row_index);
        self.record_visible_sibling(block, row_index, joins_previous_list_sibling);
        self.advance_numbered_sequence(block, block_index);
        context.nearest_visible_row_index = Some(row_index);
        self.apply_container_semantics(block, block_index, row_index, &mut context);
        context
    }

    fn visible_row(
        &self,
        block_index: usize,
        context: CanonicalRowContext,
        joins_previous_list_sibling: bool,
    ) -> LoadedCardPageVisibleRow {
        let visible_parent_row_index = context.nearest_visible_row_index;
        let visual_depth = visible_parent_row_index
            .map_or(0, |row_index| self.visible_rows[row_index].visual_depth + 1);
        LoadedCardPageVisibleRow {
            block_index,
            visual_depth,
            visible_parent_row_index,
            column_block_index: context.column_block_index,
            callout_parent_row_index: context.callout_container_row_index,
            joins_previous_list_sibling,
            joins_next_list_sibling: false,
        }
    }

    fn joins_previous_list_sibling(&self, block: &CardPageBlock) -> bool {
        block
            .editable_content()
            .is_some_and(|editable| page_block_kind_joins_list_run(editable.kind))
            && self
                .last_visible_sibling_rows
                .get(block.parent_block_id.as_str())
                .copied()
                .is_some_and(|row_index| {
                    data_row_joins_list_run(self.page, &self.visible_rows[row_index])
                })
    }

    fn record_callout_child(&mut self, callout_row_index: Option<usize>, row_index: usize) {
        if let Some(callout_row_index) = callout_row_index {
            self.callout_child_rows[callout_row_index].push(row_index);
        }
    }

    fn record_visible_sibling(
        &mut self,
        block: &'a CardPageBlock,
        row_index: usize,
        joins_previous_list_sibling: bool,
    ) {
        if joins_previous_list_sibling {
            let previous_row_index = self
                .last_visible_sibling_rows
                .get(block.parent_block_id.as_str())
                .copied()
                .expect("joined list row requires a preceding sibling row");
            self.visible_rows[previous_row_index].joins_next_list_sibling = true;
        }
        self.last_visible_sibling_rows
            .insert(block.parent_block_id.as_str(), row_index);
    }

    fn advance_numbered_sequence(&mut self, block: &'a CardPageBlock, block_index: usize) {
        let sequence = self
            .sibling_numbered_sequences
            .entry(block.parent_block_id.as_str())
            .or_default();
        if block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::NumberedList)
        {
            *sequence += 1;
            self.numbered_indices[block_index] = *sequence;
        } else {
            *sequence = 0;
        }
    }

    fn apply_container_semantics(
        &self,
        block: &CardPageBlock,
        block_index: usize,
        row_index: usize,
        context: &mut CanonicalRowContext,
    ) {
        if block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::ToggleList)
            && !self
                .expanded_toggle_ids
                .is_some_and(|block_ids| block_ids.contains(&block.block_id))
        {
            context.descendants_visible = false;
            context.collapsed_toggle_block_index = Some(block_index);
        }
        if block
            .editable_content()
            .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
        {
            context.callout_container_row_index = Some(row_index);
        }
    }
}

pub(super) fn build_canonical_projection<'a>(
    page: &'a CardPage,
    expanded_toggle_ids: Option<&'a HashSet<String>>,
) -> CanonicalProjection {
    CanonicalProjectionPass::new(page, expanded_toggle_ids).build()
}

fn data_row_joins_list_run(page: &CardPage, row: &LoadedCardPageVisibleRow) -> bool {
    page.blocks[row.block_index]
        .editable_content()
        .is_some_and(|editable| page_block_kind_joins_list_run(editable.kind))
}

const fn page_block_kind_joins_list_run(kind: CardPageBlockKind) -> bool {
    matches!(
        kind,
        CardPageBlockKind::BulletedList
            | CardPageBlockKind::NumberedList
            | CardPageBlockKind::ToDoList
            | CardPageBlockKind::ToggleList
    )
}
