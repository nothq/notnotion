use std::sync::Arc;

use gpui::{point, Bounds, IntoElement, Pixels};

use crate::ui::surface::{PageFlowLaneObservation, PageFlowLayoutWidth, PageFlowUnitObservation};
use crate::ui::{LoadedCardPageData, PageFlowLaneKey, PageFlowNodeKey, PageFlowSection};

use super::super::super::super::{div, px, AnyElement, App, ParentElement, Styled};
use super::super::super::units::PageFlowSectionRowsRender;
use super::super::super::PageBlockRenderer;
use super::super::normalize_page_block_drag_row_bounds;
use super::PageFlowRenderContext;

struct PageFlowSectionRowsObservation {
    data: Arc<LoadedCardPageData>,
    lane: PageFlowLaneKey,
    document_unit_indices: Arc<[usize]>,
    token: crate::ui::surface::PageFlowObservationToken,
}

pub(super) fn render_page_flow_section(
    renderer: &PageBlockRenderer,
    context: PageFlowRenderContext<'_>,
    section: &PageFlowSection,
    layout_width: PageFlowLayoutWidth,
    cx: &mut App,
) -> AnyElement {
    assert!(section.document_unit_range.len() <= PageFlowSection::MAX_UNITS);
    let indices = section.document_unit_range.clone().collect::<Vec<_>>();
    let rows = renderer.render_page_flow_section_rows(
        PageFlowSectionRowsRender {
            data: context.data,
            section,
            nesting_offsets: context.nesting_offsets,
            drag_selection: context.drag_selection,
            scroll_target: context.scroll_target,
            observation: context.observation,
        },
        cx,
    );
    let observation = PageFlowSectionRowsObservation {
        data: context.data.clone(),
        lane: section_lane(section).clone(),
        document_unit_indices: indices.into(),
        token: context.observation.clone(),
    };
    let row_count = observation.document_unit_indices.len();
    let observations = renderer.flow.clone();
    div()
        .w_full()
        .max_w(px(layout_width.pixels()))
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .children(rows)
        .on_children_prepainted(move |mut bounds, _, _cx| {
            bounds.truncate(row_count);
            assert_eq!(bounds.len(), row_count);
            normalize_page_block_drag_row_bounds(&mut bounds);
            observe_page_flow_section_rows(&observations, &observation, bounds);
        })
        .into_any_element()
}

fn observe_page_flow_section_rows(
    flow: &crate::ui::surface::PageDocumentFlowRuntime,
    observation: &PageFlowSectionRowsObservation,
    bounds: Vec<Bounds<Pixels>>,
) {
    assert_eq!(observation.document_unit_indices.len(), bounds.len());
    let rows = observation
        .document_unit_indices
        .iter()
        .copied()
        .zip(bounds)
        .map(|(index, bounds)| PageFlowUnitObservation::new(&observation.data, index, bounds))
        .collect::<Vec<_>>();
    let lane_bounds = enclosing_unit_bounds(&rows);
    flow.state().observations.borrow_mut().observe_lane(
        &observation.token,
        PageFlowLaneObservation::new(observation.lane.clone(), lane_bounds, rows),
    );
}

fn section_lane(section: &PageFlowSection) -> &PageFlowLaneKey {
    match &section.key {
        PageFlowNodeKey::Section { lane, .. } => lane,
        PageFlowNodeKey::Columns { .. } => unreachable!("Section must retain its lane key"),
    }
}

fn enclosing_unit_bounds(rows: &[PageFlowUnitObservation]) -> Bounds<Pixels> {
    let first = rows
        .first()
        .expect("a rendered Notion flow Section must contain a unit")
        .bounds;
    rows.iter().skip(1).fold(first, |bounds, row| {
        Bounds::from_corners(
            point(
                bounds.left().min(row.bounds.left()),
                bounds.top().min(row.bounds.top()),
            ),
            point(
                bounds.right().max(row.bounds.right()),
                bounds.bottom().max(row.bounds.bottom()),
            ),
        )
    })
}
