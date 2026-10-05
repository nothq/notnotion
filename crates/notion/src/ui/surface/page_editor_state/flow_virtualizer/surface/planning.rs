use std::ops::Range;

use crate::ui::PageFlowSequenceId;

use super::super::{
    metrics::PageFlowProjectionMetrics, pins::PageFlowPinnedNodes, prefix::PageFlowPrefixExtents,
    PageFlowPlannedNode, PageFlowRenderSpan, PageFlowViewport,
};

const PAGE_FLOW_OVERSCAN: f32 = 384.0;
const PAGE_FLOW_FIRST_PAINT_WINDOW: f32 = 768.0;

pub(super) fn visible_node_range(
    extents: &PageFlowPrefixExtents,
    viewport: PageFlowViewport,
) -> Range<usize> {
    if extents.len() == 0 {
        return 0..0;
    }
    let (start, end) = match viewport {
        PageFlowViewport::Known { start, end } => (
            (start - PAGE_FLOW_OVERSCAN).max(0.0),
            end + PAGE_FLOW_OVERSCAN,
        ),
        PageFlowViewport::UnknownLeading => (0.0, PAGE_FLOW_FIRST_PAINT_WINDOW),
        PageFlowViewport::UnknownAtAnchor { offset } => (
            (offset - PAGE_FLOW_OVERSCAN).max(0.0),
            offset + PAGE_FLOW_FIRST_PAINT_WINDOW + PAGE_FLOW_OVERSCAN,
        ),
    };
    let first = extents.index_at_offset(start);
    let last = extents
        .index_at_offset(end)
        .saturating_add(1)
        .min(extents.len());
    first..last.max(first + 1).min(extents.len())
}

pub(super) fn render_windows(
    projection: &PageFlowProjectionMetrics,
    sequence_id: &PageFlowSequenceId,
    visible: Range<usize>,
    pins: &PageFlowPinnedNodes,
) -> Vec<Range<usize>> {
    let mut windows = Vec::with_capacity(1 + pins.iter().count());
    if !visible.is_empty() {
        windows.push(visible);
    }
    for node_id in pins.iter() {
        let Some(location) = projection.node_locations.get(node_id) else {
            continue;
        };
        if &location.sequence_id == sequence_id {
            windows.push(location.index..location.index + 1);
        }
    }
    windows.sort_by_key(|range| range.start);
    merge_render_windows(windows)
}

pub(super) fn render_spans(
    extents: &PageFlowPrefixExtents,
    windows: &[Range<usize>],
) -> Vec<PageFlowRenderSpan> {
    let mut spans = Vec::with_capacity(windows.len() * 2 + 1);
    let mut cursor = 0;
    for window in windows {
        push_spacer(
            &mut spans,
            extents.prefix(window.start) - extents.prefix(cursor),
        );
        let nodes = window
            .clone()
            .map(|index| PageFlowPlannedNode {
                index,
                offset: extents.prefix(index),
                extent: extents.value(index),
            })
            .collect::<Vec<_>>()
            .into();
        spans.push(PageFlowRenderSpan::Nodes(nodes));
        cursor = window.end;
    }
    push_spacer(&mut spans, extents.total() - extents.prefix(cursor));
    spans
}

fn merge_render_windows(windows: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(windows.len());
    for window in windows {
        if let Some(previous) = merged
            .last_mut()
            .filter(|previous| window.start <= previous.end)
        {
            previous.end = previous.end.max(window.end);
        } else {
            merged.push(window);
        }
    }
    merged
}

fn push_spacer(spans: &mut Vec<PageFlowRenderSpan>, extent: f32) {
    if extent > 0.0 {
        spans.push(PageFlowRenderSpan::Spacer { extent });
    }
}
