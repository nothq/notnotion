use std::ops::Range;

use crate::ui::{CardPage, CardPageBlock};

use super::super::super::super::page_sections::{
    page_block_ends_section, page_block_section_units, page_block_starts_new_section,
    page_row_is_callout_render_unit, PAGE_SECTION_MAX_UNITS,
};
use super::super::{
    PageFlowDecoratorPlan, PageFlowLocation, PageFlowNode, PageFlowNodeKey, PageFlowNodePath,
    PageFlowOuterItem, PageFlowSection, PageFlowUnitAdjacency,
};
use super::{
    LoadedCardPageDocumentUnit, LoadedCardPageVisibleRow, PageFlowBuilder, PendingFlowUnit,
    SequenceBuild,
};

impl PageFlowBuilder<'_> {
    pub(super) fn flush_pending_sections(&mut self, sequence: &mut SequenceBuild) {
        let pending = std::mem::take(&mut sequence.pending_units);
        for span in pending_section_spans(self.page, self.rows, self.document_units, &pending) {
            let callout_tail = pending[span.start].callout_tail_visible_row_index;
            let next = pending
                .get(span.end)
                .filter(|next| next.callout_tail_visible_row_index == callout_tail);
            self.push_section(sequence, &pending[span], next);
        }
    }

    fn push_section(
        &mut self,
        sequence: &mut SequenceBuild,
        units: &[PendingFlowUnit],
        next: Option<&PendingFlowUnit>,
    ) {
        let first = units
            .first()
            .expect("Notion flow section must not be empty");
        let last = units.last().expect("Notion flow section must not be empty");
        assert_section_callout_tail(units, first);
        let key = PageFlowNodeKey::Section {
            lane: sequence.lane.clone(),
            first_unit: self.document_units[first.document_unit_index].key().clone(),
            last_unit: self.document_units[last.document_unit_index].key().clone(),
        };
        let node_path = PageFlowNodePath::push(sequence.parent_node_path.as_ref(), key.clone());
        let outer_item = sequence
            .outer_item
            .clone()
            .unwrap_or_else(|| PageFlowOuterItem {
                index: sequence.nodes.len(),
                key: key.clone(),
            });
        self.insert_node_path(&key, &node_path, &outer_item);
        for unit in units {
            self.insert_unit_location(unit, &node_path, &outer_item);
        }
        let unit_adjacencies = section_unit_adjacencies(self.document_units, units, next);
        sequence.nodes.push(PageFlowNode::Section(PageFlowSection {
            key,
            document_unit_range: first.document_unit_index..last.document_unit_index + 1,
            callout_path: first.callout_path.clone(),
            decorator_plan: PageFlowDecoratorPlan::unresolved(),
            unit_adjacencies,
        }));
    }

    fn insert_unit_location(
        &mut self,
        unit: &PendingFlowUnit,
        node_path: &PageFlowNodePath,
        outer_item: &PageFlowOuterItem,
    ) {
        let index = unit.document_unit_index;
        assert!(
            !self.assigned_document_units[index],
            "document unit assigned twice"
        );
        self.assigned_document_units[index] = true;
        let unit_key = self.document_units[index].key().clone();
        let location = PageFlowLocation {
            document_unit_index: index,
            outer_item: outer_item.clone(),
            node_path: node_path.clone(),
        };
        assert!(
            self.unit_locations
                .insert(unit_key.clone(), location)
                .is_none(),
            "duplicate Notion document-unit identity: {unit_key:?}"
        );
    }
}

fn assert_section_callout_tail(units: &[PendingFlowUnit], first: &PendingFlowUnit) {
    assert!(
        units.iter().all(|unit| {
            unit.callout_tail_visible_row_index == first.callout_tail_visible_row_index
        }),
        "Notion flow section must not cross a Callout decorator boundary"
    );
}

fn section_unit_adjacencies(
    document_units: &[LoadedCardPageDocumentUnit],
    units: &[PendingFlowUnit],
    following: Option<&PendingFlowUnit>,
) -> std::sync::Arc<[PageFlowUnitAdjacency]> {
    units
        .iter()
        .enumerate()
        .map(|(index, _)| PageFlowUnitAdjacency {
            next_owner_visible_row_index: units
                .get(index + 1)
                .or(following)
                .map(|unit| document_units[unit.document_unit_index].owner_visible_row_index()),
        })
        .collect::<Vec<_>>()
        .into()
}

pub(super) fn pending_section_spans(
    page: &CardPage,
    rows: &[LoadedCardPageVisibleRow],
    units: &[LoadedCardPageDocumentUnit],
    pending: &[PendingFlowUnit],
) -> Vec<Range<usize>> {
    assert_contiguous_document_units(pending);
    let mut spans = Vec::new();
    let mut start = 0;
    let mut section_units = 0;
    for index in 0..pending.len() {
        let block = unit_owner_block(page, rows, units, &pending[index]);
        if unit_is_isolated(rows, units, &pending[index], block) {
            push_nonempty_span(&mut spans, start, index);
            spans.push(index..index + 1);
            start = index + 1;
            section_units = 0;
            continue;
        }
        let path_changed = index > start
            && pending[index - 1].callout_tail_visible_row_index
                != pending[index].callout_tail_visible_row_index;
        if index > start && (path_changed || page_block_starts_new_section(block)) {
            spans.push(start..index);
            start = index;
            section_units = 0;
        }
        section_units += page_block_section_units(block);
        if section_ends(
            UnitOwners { page, rows, units },
            pending,
            index,
            section_units,
        ) {
            spans.push(start..index + 1);
            start = index + 1;
            section_units = 0;
        }
    }
    push_nonempty_span(&mut spans, start, pending.len());
    spans
}

/// The page rows and document units that resolve each pending unit's owner
/// block.
struct UnitOwners<'a> {
    page: &'a CardPage,
    rows: &'a [LoadedCardPageVisibleRow],
    units: &'a [LoadedCardPageDocumentUnit],
}

fn section_ends(
    owners: UnitOwners<'_>,
    pending: &[PendingFlowUnit],
    index: usize,
    section_units: usize,
) -> bool {
    let UnitOwners { page, rows, units } = owners;
    if index + 1 == pending.len() {
        return true;
    }
    let block = unit_owner_block(page, rows, units, &pending[index]);
    let next_block = unit_owner_block(page, rows, units, &pending[index + 1]);
    page_block_ends_section(block)
        || page_block_starts_new_section(next_block)
        || section_units >= PAGE_SECTION_MAX_UNITS
}

fn unit_is_isolated(
    rows: &[LoadedCardPageVisibleRow],
    units: &[LoadedCardPageDocumentUnit],
    pending: &PendingFlowUnit,
    block: &CardPageBlock,
) -> bool {
    let unit = &units[pending.document_unit_index];
    unit.is_simple_table_row()
        || page_row_is_callout_render_unit(&rows[unit.owner_visible_row_index()], block)
}

fn unit_owner_block<'a>(
    page: &'a CardPage,
    rows: &[LoadedCardPageVisibleRow],
    units: &[LoadedCardPageDocumentUnit],
    pending: &PendingFlowUnit,
) -> &'a CardPageBlock {
    let visible_row_index = units[pending.document_unit_index].owner_visible_row_index();
    &page.blocks[rows[visible_row_index].block_index]
}

fn assert_contiguous_document_units(pending: &[PendingFlowUnit]) {
    for pair in pending.windows(2) {
        assert_eq!(
            pair[0].document_unit_index + 1,
            pair[1].document_unit_index,
            "a Notion flow section run must not cross a column-group boundary"
        );
    }
}

fn push_nonempty_span(spans: &mut Vec<Range<usize>>, start: usize, end: usize) {
    if start < end {
        spans.push(start..end);
    }
}
