use std::{collections::HashMap, ops::Range};

use gpui::{App, Bounds, Entity, Pixels, Point};
use gpui_components::text_input::TextInput;

use crate::ui::surface::{PageEditorState, PageFlowLaneObservation, PageFlowSurfaceKey};
use crate::ui::{LoadedCardPageData, PageDocumentUnitKey, PageTextSelection};

use super::{DirectedTextRange, VisiblePageTextSelection};

type PageTextRow = (usize, String, Entity<TextInput>, Bounds<Pixels>);
type PageTextSelectionRange = (String, Range<usize>, bool);

/// A pointer position and the block, if any, whose own input already tracks it.
#[derive(Clone, Copy)]
pub(super) struct PageTextPointer<'a> {
    pub(super) position: Point<Pixels>,
    pub(super) excluded_block_id: Option<&'a str>,
}

struct PageTextLane<'a> {
    observation: &'a PageFlowLaneObservation,
    rows: Vec<PageTextRow>,
}

impl PageEditorState {
    pub(super) fn page_text_position(
        &self,
        data: &LoadedCardPageData,
        surface: PageFlowSurfaceKey,
        pointer: PageTextPointer<'_>,
        cx: &App,
    ) -> Option<(String, usize)> {
        let PageTextPointer {
            position,
            excluded_block_id,
        } = pointer;
        let observations = self.flow.state().observations.borrow();
        let lanes = observations.lanes(surface, data)?;
        let inputs = self.input.resource_state().block_inputs.borrow();
        let text_lanes = lanes
            .iter()
            .filter_map(|observation| {
                observed_page_text_lane(observation, data, &inputs, excluded_block_id, cx)
            })
            .collect::<Vec<_>>();
        drop(inputs);
        let lane = closest_page_text_lane(&text_lanes, position)?;
        let (_, block_id, input, bounds) = closest_page_text_row(&lane.rows, position)?;
        let offset = if position.y < bounds.top() {
            0
        } else if position.y > bounds.bottom() {
            input.read(cx).text().len()
        } else {
            input
                .read(cx)
                .byte_offset_for_window_position(position)
                .unwrap_or(0)
        };
        Some((block_id.clone(), offset))
    }
}

fn observed_page_text_lane<'a>(
    observation: &'a PageFlowLaneObservation,
    data: &LoadedCardPageData,
    inputs: &HashMap<String, Entity<TextInput>>,
    excluded_block_id: Option<&str>,
    cx: &App,
) -> Option<PageTextLane<'a>> {
    let rows = observation
        .rows
        .iter()
        .filter_map(|row| {
            let PageDocumentUnitKey::Block { block_id } = &row.unit_key else {
                return None;
            };
            if excluded_block_id == Some(block_id.as_ref()) {
                return None;
            }
            let location = data.flow.location(&row.unit_key)?;
            if location.document_unit_index != row.document_unit_index {
                return None;
            }
            let block_index = *data.editable_block_indices.get(block_id.as_ref())?;
            if !data.visible_block_mask[block_index] {
                return None;
            }
            let input = inputs.get(block_id.as_ref())?;
            Some((
                row.document_unit_index,
                block_id.to_string(),
                input.clone(),
                input.read(cx).last_window_bounds()?,
            ))
        })
        .collect::<Vec<_>>();
    (!rows.is_empty()).then_some(PageTextLane { observation, rows })
}

fn closest_page_text_lane<'a, 'b>(
    lanes: &'a [PageTextLane<'b>],
    position: Point<Pixels>,
) -> Option<&'a PageTextLane<'b>> {
    lanes.iter().min_by(|left, right| {
        left.observation
            .rank(position)
            .compare(right.observation.rank(position))
    })
}

fn closest_page_text_row(rows: &[PageTextRow], position: Point<Pixels>) -> Option<&PageTextRow> {
    rows.iter()
        .find(|(_, _, _, bounds)| bounds.contains(&position))
        .or_else(|| {
            rows.iter().min_by(|(_, _, _, left), (_, _, _, right)| {
                vertical_distance(*left, position).total_cmp(&vertical_distance(*right, position))
            })
        })
}

fn vertical_distance(bounds: Bounds<Pixels>, position: Point<Pixels>) -> f32 {
    if position.y < bounds.top() {
        (bounds.top() - position.y).as_f32()
    } else if position.y > bounds.bottom() {
        (position.y - bounds.bottom()).as_f32()
    } else {
        0.0
    }
}

pub(crate) fn page_text_selection_ranges(
    data: &LoadedCardPageData,
    selection: &PageTextSelection,
) -> Vec<PageTextSelectionRange> {
    let Some(visible_selection) = VisiblePageTextSelection::new(data, selection) else {
        return Vec::new();
    };
    visible_selection
        .block_indices()
        .filter_map(|index| {
            visible_selection.range_for(index).map(|(range, reversed)| {
                (data.page.blocks[index].block_id.clone(), range, reversed)
            })
        })
        .collect()
}

pub(crate) fn page_text_selection_range(
    data: &LoadedCardPageData,
    selection: &PageTextSelection,
    candidate_index: usize,
) -> Option<DirectedTextRange> {
    VisiblePageTextSelection::new(data, selection)?.range_for(candidate_index)
}
