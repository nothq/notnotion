use std::ops::Range;

use crate::ui::{CardPage, CardPageBlockColor, CardPageBlockColorValue, CardPageBlockKind};

use super::super::LoadedCardPageVisibleRow;
use super::{CanonicalProjection, DocumentProjection};

pub(super) struct CalloutProjection {
    pub(super) layer_row_indices: Vec<usize>,
    pub(super) layer_row_ranges: Vec<Range<usize>>,
    pub(super) render_unit_ranges: Vec<Range<usize>>,
    pub(super) first_text_input_row_indices: Vec<Option<usize>>,
    pub(super) inherited_text_colors: Vec<Option<CardPageBlockColorValue>>,
    pub(super) effective_colors: Vec<Option<CardPageBlockColor>>,
}

struct CalloutMetadata {
    layer_row_indices: Vec<usize>,
    layer_row_ranges: Vec<Range<usize>>,
    inherited_text_colors: Vec<Option<CardPageBlockColorValue>>,
    effective_colors: Vec<Option<CardPageBlockColor>>,
}

struct CalloutMetadataBuilder<'a> {
    page: &'a CardPage,
    layer_row_indices: Vec<usize>,
    layer_row_ranges: Vec<Range<usize>>,
    inherited_text_colors: Vec<Option<CardPageBlockColorValue>>,
    effective_colors: Vec<Option<CardPageBlockColor>>,
    descendant_text_colors: Vec<Option<CardPageBlockColorValue>>,
}

impl<'a> CalloutMetadataBuilder<'a> {
    fn new(page: &'a CardPage, row_count: usize) -> Self {
        Self {
            page,
            layer_row_indices: Vec::new(),
            layer_row_ranges: Vec::with_capacity(row_count),
            inherited_text_colors: Vec::with_capacity(row_count),
            effective_colors: Vec::with_capacity(row_count),
            descendant_text_colors: Vec::with_capacity(row_count),
        }
    }

    fn build(mut self, rows: &[LoadedCardPageVisibleRow]) -> CalloutMetadata {
        for (row_index, row) in rows.iter().enumerate() {
            self.push_row(row_index, row);
        }
        CalloutMetadata {
            layer_row_indices: self.layer_row_indices,
            layer_row_ranges: self.layer_row_ranges,
            inherited_text_colors: self.inherited_text_colors,
            effective_colors: self.effective_colors,
        }
    }

    fn push_row(&mut self, row_index: usize, row: &LoadedCardPageVisibleRow) {
        let inherited_text_color = row
            .callout_parent_row_index
            .and_then(|parent_index| self.descendant_text_colors[parent_index]);
        self.inherited_text_colors.push(inherited_text_color);

        let layer_start = self.layer_row_indices.len();
        if let Some(parent_index) = row.callout_parent_row_index {
            self.layer_row_indices
                .extend_from_within(self.layer_row_ranges[parent_index].clone());
        }
        let page = self.page;
        let block = &page.blocks[row.block_index];
        let effective_color = block.editable_content().and_then(|editable| {
            (editable.kind == CardPageBlockKind::Callout).then(|| {
                let color = inherited_page_block_color(block.color, inherited_text_color);
                self.layer_row_indices.push(row_index);
                color
            })
        });
        self.effective_colors.push(effective_color);
        self.descendant_text_colors
            .push(effective_color.and_then(callout_descendant_text_color));
        self.layer_row_ranges
            .push(layer_start..self.layer_row_indices.len());
    }
}

pub(super) fn build_callout_projection(
    page: &CardPage,
    canonical: &CanonicalProjection,
    document: &DocumentProjection,
) -> CalloutProjection {
    let subtree_end_indices = visible_subtree_end_indices(&canonical.visible_rows);
    let render_unit_prefix = build_render_unit_prefix(&document.document_unit_ranges);
    let render_unit_ranges = build_render_unit_ranges(
        page,
        &canonical.visible_rows,
        &render_unit_prefix,
        &subtree_end_indices,
    );
    let first_text_input_row_indices = build_first_text_input_row_indices(
        page,
        &canonical.visible_rows,
        &document.text_input_block_mask,
        &subtree_end_indices,
    );
    let metadata = CalloutMetadataBuilder::new(page, canonical.visible_rows.len())
        .build(&canonical.visible_rows);
    CalloutProjection {
        layer_row_indices: metadata.layer_row_indices,
        layer_row_ranges: metadata.layer_row_ranges,
        render_unit_ranges,
        first_text_input_row_indices,
        inherited_text_colors: metadata.inherited_text_colors,
        effective_colors: metadata.effective_colors,
    }
}

fn build_render_unit_prefix(document_unit_ranges: &[Range<usize>]) -> Vec<usize> {
    let mut prefix = Vec::with_capacity(document_unit_ranges.len() + 1);
    prefix.push(0);
    for range in document_unit_ranges {
        assert_eq!(
            range.start,
            *prefix
                .last()
                .expect("Notion document-unit prefix always contains its origin"),
            "Notion document-unit ranges must be contiguous"
        );
        prefix.push(range.end);
    }
    prefix
}

fn build_render_unit_ranges(
    page: &CardPage,
    rows: &[LoadedCardPageVisibleRow],
    render_unit_prefix: &[usize],
    subtree_end_indices: &[usize],
) -> Vec<Range<usize>> {
    rows.iter()
        .enumerate()
        .map(|(row_index, row)| {
            if !page.blocks[row.block_index]
                .editable_content()
                .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
            {
                return 0..0;
            }
            render_unit_prefix[row_index]..render_unit_prefix[subtree_end_indices[row_index]]
        })
        .collect()
}

fn build_first_text_input_row_indices(
    page: &CardPage,
    rows: &[LoadedCardPageVisibleRow],
    text_input_block_mask: &[bool],
    subtree_end_indices: &[usize],
) -> Vec<Option<usize>> {
    let next_text_input_row_indices =
        build_next_text_input_row_indices(rows, text_input_block_mask);
    rows.iter()
        .enumerate()
        .map(|(row_index, row)| {
            if !page.blocks[row.block_index]
                .editable_content()
                .is_some_and(|editable| editable.kind == CardPageBlockKind::Callout)
            {
                return None;
            }
            next_text_input_row_indices[row_index + 1]
                .filter(|next_row_index| *next_row_index < subtree_end_indices[row_index])
        })
        .collect()
}

fn build_next_text_input_row_indices(
    rows: &[LoadedCardPageVisibleRow],
    text_input_block_mask: &[bool],
) -> Vec<Option<usize>> {
    let mut next_text_input_row_index = None;
    let mut indices = vec![None; rows.len() + 1];
    for row_index in (0..rows.len()).rev() {
        let block_index = rows[row_index].block_index;
        if text_input_block_mask[block_index] {
            next_text_input_row_index = Some(row_index);
        }
        indices[row_index] = next_text_input_row_index;
    }
    indices
}

fn visible_subtree_end_indices(rows: &[LoadedCardPageVisibleRow]) -> Vec<usize> {
    let mut subtree_end_indices = vec![rows.len(); rows.len()];
    let mut open_rows = Vec::<usize>::new();
    for (row_index, row) in rows.iter().enumerate() {
        while open_rows
            .last()
            .is_some_and(|open_row_index| rows[*open_row_index].visual_depth >= row.visual_depth)
        {
            let closed_row_index = open_rows
                .pop()
                .expect("visible subtree stack must contain the checked row");
            subtree_end_indices[closed_row_index] = row_index;
        }
        open_rows.push(row_index);
    }
    subtree_end_indices
}

fn inherited_page_block_color(
    color: CardPageBlockColor,
    inherited_text_color: Option<CardPageBlockColorValue>,
) -> CardPageBlockColor {
    match (color, inherited_text_color) {
        (CardPageBlockColor::Text(CardPageBlockColorValue::Default), Some(value)) => {
            CardPageBlockColor::Text(value)
        }
        _ => color,
    }
}

fn callout_descendant_text_color(color: CardPageBlockColor) -> Option<CardPageBlockColorValue> {
    match color {
        CardPageBlockColor::Text(CardPageBlockColorValue::Default)
        | CardPageBlockColor::Background(_) => None,
        CardPageBlockColor::Text(value) => Some(value),
    }
}
