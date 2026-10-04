use gpui::ListState;

use crate::ui::board_workspace::page::editor::render::PageBlockRenderer;
use crate::ui::surface::{
    PageFlowLogicalViewportBasis, PageFlowObservationToken, PageFlowOuterExtentAuthority,
    PageFlowOuterItemExtent, PageFlowOuterItemRemap, PageFlowSemanticAnchorTransaction,
};
use crate::ui::{LoadedCardPageData, PageDocumentOuterItemId};

const PAGE_FLOW_OUTER_EXTENT_WINDOW_CAP: usize = 64;

struct PageFlowOuterExtentSource<'a> {
    renderer: &'a PageBlockRenderer,
    observation: &'a PageFlowObservationToken,
    data: &'a LoadedCardPageData,
    list_state: &'a ListState,
}

pub(super) fn outer_extent_authority(
    renderer: &PageBlockRenderer,
    observation: &PageFlowObservationToken,
    data: &LoadedCardPageData,
    list_state: &ListState,
    anchor: &PageFlowSemanticAnchorTransaction,
) -> Option<PageFlowOuterExtentAuthority> {
    let source = PageFlowOuterExtentSource {
        renderer,
        observation,
        data,
        list_state,
    };
    let item_count = data.flow.root.nodes.len() + 2;
    let logical = list_state.logical_scroll_top();
    let required = required_outer_index(anchor, item_count)?;
    let logical_item = logical.item_ix.min(item_count - 1);
    let required_item = required.min(item_count - 1);
    let mut first = logical_item.min(required_item);
    let last = logical_item.max(required_item);
    if last - first >= PAGE_FLOW_OUTER_EXTENT_WINDOW_CAP {
        return None;
    }
    let mut items = source.measured_items(first..=last)?;
    extend_outer_extent_window(&source, anchor, required_item, &mut first, &mut items)?;
    Some(PageFlowOuterExtentAuthority::from_measured_window(
        item_count, first, items,
    ))
}

fn extend_outer_extent_window(
    source: &PageFlowOuterExtentSource<'_>,
    anchor: &PageFlowSemanticAnchorTransaction,
    target_index: usize,
    first: &mut usize,
    items: &mut Vec<PageFlowOuterItemExtent>,
) -> Option<()> {
    let Some(projection) = anchor.latest_projection.as_ref() else {
        return Some(());
    };
    assert_eq!(projection.outer_index, target_index);
    let target = target_index - *first;
    let desired = projection.outer_local_y.pixels() - anchor.baseline.viewport_relative_y.pixels();
    let before = items[..target]
        .iter()
        .map(PageFlowOuterItemExtent::pixels)
        .sum::<f32>();
    let after = items[target + 1..]
        .iter()
        .map(PageFlowOuterItemExtent::pixels)
        .sum::<f32>();
    let before_deficit = (-desired - before).max(0.0);
    let after_deficit = (desired - items[target].pixels() - after).max(0.0);
    extend_before(source, first, items, before_deficit)?;
    extend_after(source, *first, items, after_deficit)
}

fn extend_before(
    source: &PageFlowOuterExtentSource<'_>,
    first: &mut usize,
    items: &mut Vec<PageFlowOuterItemExtent>,
    mut deficit: f32,
) -> Option<()> {
    while deficit > 0.0 && *first > 0 && items.len() < PAGE_FLOW_OUTER_EXTENT_WINDOW_CAP {
        *first -= 1;
        let item = source.measured_item(*first)?;
        deficit -= item.pixels();
        items.insert(0, item);
    }
    Some(())
}

fn extend_after(
    source: &PageFlowOuterExtentSource<'_>,
    first: usize,
    items: &mut Vec<PageFlowOuterItemExtent>,
    mut deficit: f32,
) -> Option<()> {
    let item_count = source.data.flow.root.nodes.len() + 2;
    while deficit > 0.0 && items.len() < PAGE_FLOW_OUTER_EXTENT_WINDOW_CAP {
        let next = first + items.len();
        if next == item_count {
            break;
        }
        let item = source.measured_item(next)?;
        deficit -= item.pixels();
        items.push(item);
    }
    Some(())
}

impl PageFlowOuterExtentSource<'_> {
    fn measured_items(
        &self,
        indices: impl IntoIterator<Item = usize>,
    ) -> Option<Vec<PageFlowOuterItemExtent>> {
        indices
            .into_iter()
            .map(|index| self.measured_item(index))
            .collect()
    }

    fn measured_item(&self, index: usize) -> Option<PageFlowOuterItemExtent> {
        let outer_item = outer_item_id(self.data, index)?;
        let extent = self
            .renderer
            .flow
            .state()
            .observations
            .borrow()
            .outer_extent(self.observation, &outer_item)
            .or_else(|| {
                self.list_state
                    .bounds_for_item(index)
                    .map(|bounds| bounds.size.height.as_f32())
            })?;
        Some(PageFlowOuterItemExtent::from_measured_pixels(
            outer_item, extent,
        ))
    }
}

fn required_outer_index(
    anchor: &PageFlowSemanticAnchorTransaction,
    item_count: usize,
) -> Option<usize> {
    if let Some(projection) = &anchor.latest_projection {
        return Some(projection.outer_index);
    }
    match &anchor.outer_remap {
        PageFlowOuterItemRemap::Start => Some(0),
        PageFlowOuterItemRemap::Tail { .. } => item_count.checked_sub(1),
        PageFlowOuterItemRemap::Item { new_index, .. } => Some(*new_index),
    }
}

pub(super) fn logical_viewport_basis(
    data: &LoadedCardPageData,
    list_state: &ListState,
) -> PageFlowLogicalViewportBasis {
    let logical = list_state.logical_scroll_top();
    let item_count = data.flow.root.nodes.len() + 2;
    if logical.item_ix == item_count {
        return PageFlowLogicalViewportBasis::tail(item_count);
    }
    PageFlowLogicalViewportBasis::item(
        outer_item_id(data, logical.item_ix)
            .expect("logical Notion viewport item must retain an outer identity"),
        logical.item_ix,
        logical.offset_in_item.as_f32(),
    )
}

fn outer_item_id(data: &LoadedCardPageData, index: usize) -> Option<PageDocumentOuterItemId> {
    if index == 0 {
        return Some(PageDocumentOuterItemId::Lead);
    }
    if index == data.flow.root.nodes.len() + 1 {
        return Some(PageDocumentOuterItemId::Footer);
    }
    data.flow
        .root
        .nodes
        .get(index.checked_sub(1)?)
        .map(|node| PageDocumentOuterItemId::Flow(node.node_id()))
}
