use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
};

use gpui::{px, ListAlignment, ListState};

use crate::ui::{CardPage, CardPageBlock, CardPageBlockColor, CardPageBlockColorValue};

use super::{
    page_rows::build_loaded_card_page_row_projection,
    page_sections::build_loaded_card_page_sections, PageTextSelection,
};

mod disclosure_projection;
mod flow;
mod identity;
mod layout_revision;
mod projection_invariants;
mod row_layout;
mod table;
mod unit_lookup;

use flow::build_page_flow_projection;
pub(crate) use flow::{
    PageColumnFlow, PageDocumentOuterItemId, PageFlowCalloutLayoutRevision,
    PageFlowCalloutPresentationSpec, PageFlowCalloutSegment, PageFlowColumns,
    PageFlowColumnsPresentationSpec, PageFlowDecoratorLayer, PageFlowDecoratorPlan,
    PageFlowDecoratorPlanRevision, PageFlowExtentEnvelope, PageFlowLaneKey, PageFlowNode,
    PageFlowNodeId, PageFlowNodeKey, PageFlowNodePath, PageFlowPaintedCalloutPrefix,
    PageFlowProjection, PageFlowSection, PageFlowSequence, PageFlowSequenceId,
    PageFlowUnitAdjacency, PAGE_FLOW_BLOCK_INDENT,
};
pub(crate) use identity::PageDocumentListAllocation;
use identity::{build_page_block_indices, card_page_has_column_structure, card_pages_match};
use row_layout::PageVisibleRowLayouts;
pub(crate) use row_layout::{visible_row_spacing, PageVisibleRowSpacing};
pub(crate) use table::{
    LoadedCardPageDocumentUnit, LoadedCardPageSimpleTableAnnotationRun,
    LoadedCardPageSimpleTableCell, LoadedCardPageSimpleTableCellAccess,
    LoadedCardPageSimpleTableCellLocation, LoadedCardPageSimpleTableRowPosition,
    PageDocumentUnitKey, PageDocumentUnitLayoutRevision,
};
use unit_lookup::build_empty_toggle_placeholder_mask;

#[derive(Clone)]
pub struct LoadedCardPage {
    pub data: Arc<LoadedCardPageData>,
    pub list_state: ListState,
    pub(crate) list_allocation: PageDocumentListAllocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedCardPageSection {
    pub unit_range: Range<usize>,
}

#[derive(Clone)]
pub struct LoadedCardPageData {
    pub page: CardPage,
    pub(crate) flow_projection_generation: u64,
    authority: LoadedCardPageAuthority,
    pub numbered_indices: Vec<usize>,
    pub visible_rows: Vec<LoadedCardPageVisibleRow>,
    pub(crate) document_units: Vec<LoadedCardPageDocumentUnit>,
    pub(crate) document_unit_layout_revisions: Vec<PageDocumentUnitLayoutRevision>,
    pub(crate) callout_layout_revisions: Vec<Option<PageFlowCalloutLayoutRevision>>,
    pub(crate) flow: PageFlowProjection,
    pub(crate) document_unit_ranges: Vec<Range<usize>>,
    pub(crate) simple_table_cell_locations: HashMap<
        crate::model::CardPageSimpleTableCellAddress,
        LoadedCardPageSimpleTableCellLocation,
    >,
    pub(crate) callout_layer_row_indices: Vec<usize>,
    pub(crate) callout_layer_row_ranges: Vec<Range<usize>>,
    pub(crate) callout_render_unit_ranges: Vec<Range<usize>>,
    pub(crate) callout_first_text_input_row_indices: Vec<Option<usize>>,
    pub(crate) inherited_text_colors: Vec<Option<CardPageBlockColorValue>>,
    pub(crate) effective_callout_colors: Vec<Option<CardPageBlockColor>>,
    pub sections: Vec<LoadedCardPageSection>,
    pub(crate) text_input_block_mask: Vec<bool>,
    pub(crate) empty_toggle_placeholder_mask: Vec<bool>,
    pub(crate) visible_block_mask: Vec<bool>,
    pub(crate) flow_visible_block_mask: Vec<bool>,
    authoritative_has_column_structure: bool,
    pub(crate) collapsed_hidden_owner_indices: Arc<[Option<usize>]>,
    pub(crate) next_root_numbered_index: usize,
    block_indices: HashMap<String, usize>,
    pub(crate) editable_block_indices: HashMap<String, usize>,
    visible_row_layouts: PageVisibleRowLayouts,
}

#[derive(Clone)]
enum LoadedCardPageAuthority {
    MatchesVisible,
    Diverged(Arc<CardPage>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadedCardPageVisibleRow {
    pub block_index: usize,
    pub visual_depth: usize,
    pub visible_parent_row_index: Option<usize>,
    pub column_block_index: Option<usize>,
    pub(crate) callout_parent_row_index: Option<usize>,
    pub(crate) joins_previous_list_sibling: bool,
    pub(crate) joins_next_list_sibling: bool,
}

impl LoadedCardPage {
    pub fn new(page: CardPage) -> Self {
        Self::with_expanded_toggles(page, None)
    }

    pub(crate) fn with_expanded_toggles(
        page: CardPage,
        expanded_toggle_ids: Option<&HashSet<String>>,
    ) -> Self {
        Self::with_authority_state(
            page,
            LoadedCardPageAuthority::MatchesVisible,
            expanded_toggle_ids,
        )
    }

    pub(crate) fn with_authority(
        page: CardPage,
        authority: Arc<CardPage>,
        expanded_toggle_ids: Option<&HashSet<String>>,
    ) -> Self {
        assert_eq!(
            page.block_id, authority.block_id,
            "loaded page projection and authority must identify the same page"
        );
        let authority = if card_pages_match(&page, &authority) {
            LoadedCardPageAuthority::MatchesVisible
        } else {
            LoadedCardPageAuthority::Diverged(authority)
        };
        Self::with_authority_state(page, authority, expanded_toggle_ids)
    }

    fn with_authority_state(
        page: CardPage,
        authority: LoadedCardPageAuthority,
        expanded_toggle_ids: Option<&HashSet<String>>,
    ) -> Self {
        Self {
            data: Arc::new(LoadedCardPageData::new(
                page,
                authority,
                expanded_toggle_ids,
            )),
            list_state: ListState::new(0, ListAlignment::Top, px(128.0)),
            list_allocation: PageDocumentListAllocation::fresh(),
        }
    }
}

impl LoadedCardPageData {
    fn new(
        page: CardPage,
        authority: LoadedCardPageAuthority,
        expanded_toggle_ids: Option<&HashSet<String>>,
    ) -> Self {
        let projection = build_loaded_card_page_row_projection(&page, expanded_toggle_ids);
        let visible_row_layouts = PageVisibleRowLayouts::build(&page, &projection.visible_rows);
        let flow = build_page_flow_projection(&page, &projection, &visible_row_layouts);
        let sections = build_loaded_card_page_sections(
            &page.blocks,
            &projection.visible_rows,
            &projection.document_units,
        );
        let block_indices = build_page_block_indices(&page);
        let authoritative_has_column_structure =
            projection_invariants::validate_flow_projection(&page, &authority, &flow, &sections);
        let empty_toggle_placeholder_mask = build_empty_toggle_placeholder_mask(
            &page,
            &projection.visible_rows,
            expanded_toggle_ids,
        );
        let (document_unit_layout_revisions, callout_layout_revisions) =
            layout_revision::fresh_page_layout_revisions(
                &page,
                &projection.visible_rows,
                projection.document_units.len(),
            );
        Self {
            page,
            flow_projection_generation: 0,
            authority,
            numbered_indices: projection.numbered_indices,
            visible_rows: projection.visible_rows,
            document_units: projection.document_units,
            document_unit_layout_revisions,
            callout_layout_revisions,
            flow,
            document_unit_ranges: projection.document_unit_ranges,
            simple_table_cell_locations: projection.simple_table_cell_locations,
            callout_layer_row_indices: projection.callout_layer_row_indices,
            callout_layer_row_ranges: projection.callout_layer_row_ranges,
            callout_render_unit_ranges: projection.callout_render_unit_ranges,
            callout_first_text_input_row_indices: projection.callout_first_text_input_row_indices,
            inherited_text_colors: projection.inherited_text_colors,
            effective_callout_colors: projection.effective_callout_colors,
            sections,
            text_input_block_mask: projection.text_input_block_mask,
            empty_toggle_placeholder_mask,
            visible_block_mask: projection.visible_block_mask,
            flow_visible_block_mask: projection.flow_visible_block_mask,
            authoritative_has_column_structure,
            collapsed_hidden_owner_indices: projection.collapsed_hidden_owner_indices.into(),
            next_root_numbered_index: projection.next_root_numbered_index,
            block_indices: block_indices.all,
            editable_block_indices: block_indices.editable,
            visible_row_layouts,
        }
    }

    pub(crate) fn authority_arc(&self) -> Arc<CardPage> {
        match &self.authority {
            LoadedCardPageAuthority::MatchesVisible => Arc::new(self.page.clone()),
            LoadedCardPageAuthority::Diverged(authority) => authority.clone(),
        }
    }

    pub(crate) fn advance_authority(&mut self, authority: Arc<CardPage>) {
        assert_eq!(
            self.page.block_id, authority.block_id,
            "loaded page and advanced authority must identify the same page"
        );
        self.authoritative_has_column_structure = card_page_has_column_structure(&self.page)
            || card_page_has_column_structure(&authority);
        self.authority = if card_pages_match(&self.page, &authority) {
            LoadedCardPageAuthority::MatchesVisible
        } else {
            LoadedCardPageAuthority::Diverged(authority)
        };
    }

    fn rebuilt_visible_projection(
        &self,
        expanded_toggle_ids: Option<&HashSet<String>>,
    ) -> Option<Self> {
        let mut replacement = Self::new(
            self.page.clone(),
            self.authority.clone(),
            expanded_toggle_ids,
        );
        replacement.flow_projection_generation = self.flow_projection_generation.wrapping_add(1);
        replacement.reuse_document_unit_layout_revisions(self);
        let changed = !self.has_same_visible_projection(&replacement);
        if !changed {
            return None;
        }
        Some(replacement)
    }

    pub(crate) fn editable_block_is_visible(&self, block_id: &str) -> bool {
        self.editable_block_indices
            .get(block_id)
            .is_some_and(|index| self.visible_block_mask[*index])
    }

    pub(crate) fn block_has_text_input(&self, block_id: &str) -> bool {
        self.editable_block_indices
            .get(block_id)
            .is_some_and(|index| self.text_input_block_mask[*index])
    }

    pub(crate) fn callout_render_unit_range(&self, visible_row_index: usize) -> Range<usize> {
        self.callout_render_unit_ranges[visible_row_index].clone()
    }

    pub(crate) fn callout_layer_row_indices(&self, visible_row_index: usize) -> &[usize] {
        let range = self.callout_layer_row_ranges[visible_row_index].clone();
        &self.callout_layer_row_indices[range]
    }

    pub(crate) fn callout_layer_depth(&self, visible_row_index: usize) -> usize {
        self.callout_layer_row_ranges[visible_row_index].len()
    }

    pub(crate) fn page_block_render_depth(&self, visible_row_index: usize) -> usize {
        self.callout_layer_row_indices(visible_row_index)
            .first()
            .map_or(
                self.visible_rows[visible_row_index].visual_depth,
                |row_index| self.visible_rows[*row_index].visual_depth,
            )
    }

    pub(crate) fn callout_layer_depth_for_block_id(&self, block_id: &str) -> usize {
        let Some(block_index) = self.editable_block_indices.get(block_id).copied() else {
            return 0;
        };
        self.visible_rows
            .binary_search_by_key(&block_index, |row| row.block_index)
            .ok()
            .map_or(0, |row_index| self.callout_layer_depth(row_index))
    }

    pub(crate) fn inherited_text_color(
        &self,
        visible_row_index: usize,
    ) -> Option<CardPageBlockColorValue> {
        self.inherited_text_colors[visible_row_index]
    }

    pub(crate) fn effective_callout_color(&self, visible_row_index: usize) -> CardPageBlockColor {
        self.effective_callout_colors[visible_row_index]
            .expect("effective Callout colors are defined for Callout layer rows")
    }

    pub(crate) fn callout_first_text_input_row_index(
        &self,
        visible_row_index: usize,
    ) -> Option<usize> {
        self.callout_first_text_input_row_indices[visible_row_index]
    }

    pub(crate) fn block_is_visible(&self, block_id: &str) -> bool {
        if block_id == self.page.block_id {
            return true;
        }
        self.block_indices
            .get(block_id)
            .is_some_and(|index| self.visible_block_mask[*index])
    }

    pub(crate) const fn has_column_structure(&self) -> bool {
        self.authoritative_has_column_structure
    }

    pub(crate) fn page_block(&self, block_id: &str) -> Option<&CardPageBlock> {
        self.block_indices
            .get(block_id)
            .map(|index| &self.page.blocks[*index])
    }

    pub(crate) fn text_selection_is_visible(&self, selection: &PageTextSelection) -> bool {
        let Some(anchor_index) = self
            .editable_block_indices
            .get(&selection.anchor_block_id)
            .copied()
        else {
            return false;
        };
        let Some(focus_index) = self
            .editable_block_indices
            .get(&selection.focus_block_id)
            .copied()
        else {
            return false;
        };
        self.visible_block_mask[anchor_index] && self.visible_block_mask[focus_index]
    }

    pub(crate) fn has_same_visible_projection(&self, other: &Self) -> bool {
        self.numbered_indices == other.numbered_indices
            && self.visible_rows.len() == other.visible_rows.len()
            && self
                .visible_rows
                .iter()
                .zip(&other.visible_rows)
                .all(|(left, right)| {
                    left == right
                        && self.page.blocks[left.block_index].block_id
                            == other.page.blocks[right.block_index].block_id
                })
            && self.document_units == other.document_units
            && self.document_unit_ranges == other.document_unit_ranges
            && self.flow.has_same_authoritative_flow_semantics(&other.flow)
            && self.simple_table_cell_locations == other.simple_table_cell_locations
            && self.callout_layer_row_indices == other.callout_layer_row_indices
            && self.callout_layer_row_ranges == other.callout_layer_row_ranges
            && self.callout_render_unit_ranges == other.callout_render_unit_ranges
            && self.callout_first_text_input_row_indices
                == other.callout_first_text_input_row_indices
            && self.inherited_text_colors == other.inherited_text_colors
            && self.effective_callout_colors == other.effective_callout_colors
            && self.sections == other.sections
            && self.text_input_block_mask == other.text_input_block_mask
            && self.empty_toggle_placeholder_mask == other.empty_toggle_placeholder_mask
            && self.visible_block_mask == other.visible_block_mask
            && self.flow_visible_block_mask == other.flow_visible_block_mask
            && self.authoritative_has_column_structure == other.authoritative_has_column_structure
            && self.collapsed_hidden_owner_indices == other.collapsed_hidden_owner_indices
            && self.next_root_numbered_index == other.next_root_numbered_index
            && self.editable_block_indices == other.editable_block_indices
    }
}
