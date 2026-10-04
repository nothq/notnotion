use std::ops::Range;

use gpui::{Bounds, FocusHandle, IntoElement, Pixels};

use crate::ui::surface::{
    PageDocumentFlowRuntime, PageFlowLaneObservation, PageFlowObservationToken,
};

use super::super::super::{
    div, px, AnyElement, App, Arc, FluentBuilder, InteractiveElement, LoadedCardPageData,
    PageBlockDragScrollTarget, PageBlockDragSelection, ParentElement, Styled,
    PAGE_BLOCK_GUTTER_WIDTH,
};
use super::super::units::PageBlockSectionRowsRender;
use super::super::{
    PageBlockRenderer, PageDragRenderAction, PageLinearRowsObservation, PageRenderAction,
};

pub(super) struct PageBlockSectionRender<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) document_unit_range: Range<usize>,
    pub(super) nesting_offsets: &'a [f32],
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) generation: u64,
    pub(super) observation: &'a PageFlowObservationToken,
    pub(super) first_section: bool,
    pub(super) focus_handle: &'a FocusHandle,
}

struct PageBlockSectionObservation<'a> {
    data: &'a Arc<LoadedCardPageData>,
    document_unit_indices: Vec<usize>,
    row_bounds: Vec<Bounds<Pixels>>,
    scroll_target: PageBlockDragScrollTarget,
    generation: u64,
    observation: &'a PageFlowObservationToken,
    active_drag: bool,
}

pub(in crate::ui::board_workspace::page::editor::render) fn normalize_page_block_drag_row_bounds(
    bounds: &mut [Bounds<Pixels>],
) {
    for bounds in bounds {
        bounds.origin.x += px(PAGE_BLOCK_GUTTER_WIDTH);
        bounds.size.width -= px(PAGE_BLOCK_GUTTER_WIDTH);
    }
}

impl PageBlockRenderer {
    pub(super) fn render_page_block_section(
        &self,
        section: PageBlockSectionRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let indices = section.document_unit_range.clone().collect::<Vec<_>>();
        let observed_indices = indices.clone();
        let observed_data = section.data.clone();
        let row_count = indices.len();
        let flow = self.flow.clone();
        let actions = self.actions.clone();
        let scroll_target = section.scroll_target;
        let generation = section.generation;
        let observation_token = section.observation.clone();
        let rows = self.render_page_block_section_rows(
            PageBlockSectionRowsRender {
                data: section.data,
                document_unit_indices: indices,
                nesting_offsets: section.nesting_offsets,
                drag_selection: section.drag_selection,
                scroll_target: section.scroll_target,
                observation: section.observation,
            },
            cx,
        );
        div()
            .relative()
            .w_full()
            .when_some(self.column.maximum_width(), |section, width| {
                section.max_w(px(width))
            })
            .flex()
            .flex_col()
            .track_focus(section.focus_handle)
            .when(section.first_section, |section| section.pt(px(20.0)))
            .children(rows)
            .on_children_prepainted(move |mut bounds, window, cx| {
                bounds.truncate(row_count);
                normalize_page_block_drag_row_bounds(&mut bounds);
                let observation = PageBlockSectionObservation {
                    data: &observed_data,
                    document_unit_indices: observed_indices.clone(),
                    row_bounds: bounds,
                    scroll_target,
                    generation,
                    observation: &observation_token,
                    active_drag: cx.has_active_drag(),
                };
                observe_page_block_section_rows(&flow, &actions, observation, window, cx);
            })
            .into_any_element()
    }
}

fn observe_page_block_section_rows(
    flow: &PageDocumentFlowRuntime,
    actions: &crate::ui::view_actions::ViewActionSink<PageRenderAction>,
    observation: PageBlockSectionObservation<'_>,
    window: &mut gpui::Window,
    cx: &mut App,
) {
    let spatial = PageFlowLaneObservation::linear_section(
        observation.data,
        &observation.document_unit_indices,
        observation.row_bounds.clone(),
    );
    let current = flow
        .state()
        .observations
        .borrow_mut()
        .observe_lane(observation.observation, spatial);
    if !current || observation.data.has_column_structure() {
        return;
    }
    actions.emit(
        PageRenderAction::Drag(PageDragRenderAction::ObserveLinearRows(
            PageLinearRowsObservation {
                scroll_target: observation.scroll_target,
                data: observation.data.clone(),
                document_unit_indices: observation.document_unit_indices,
                row_bounds: observation.row_bounds,
                generation: observation.generation,
                active_drag: observation.active_drag,
            },
        )),
        window,
        cx,
    );
}
