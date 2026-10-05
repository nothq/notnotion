use std::{collections::HashMap, sync::Arc};

use crate::ui::{PageDocumentOuterItemId, PageFlowNodeId};

use super::{prefix::PageFlowExtentChange, PageFlowRenderGeneration, PageFlowSurfaceKey};

mod anchor;

pub(crate) use anchor::{
    PageFlowDeferredScrollbarAnchor, PageFlowLogicalViewportBasis, PageFlowOuterExtentAuthority,
    PageFlowOuterItemExtent, PageFlowOuterLocalY, PageFlowSemanticAnchorBaseline,
    PageFlowSemanticAnchorIntent, PageFlowSemanticAnchorProjection,
    PageFlowSemanticAnchorTransaction, PageFlowViewportRelativeY,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageFlowLayoutFrameGeneration(pub(super) u64);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageFlowLayoutFrameToken {
    page_id: Arc<str>,
    surface: PageFlowSurfaceKey,
    render_generation: PageFlowRenderGeneration,
    layout_generation: PageFlowLayoutFrameGeneration,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PageFlowOuterItemMatch {
    Exact {
        captured_offset: PageFlowOuterLocalY,
    },
    NearestSurvivor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageFlowLayoutCommitSchedule {
    Stale,
    AlreadyScheduled,
    Schedule,
}

#[derive(Clone, Debug)]
pub(crate) enum PageFlowOuterItemRemap {
    Start,
    Tail {
        item_count: usize,
    },
    Item {
        key: PageDocumentOuterItemId,
        new_index: usize,
        match_kind: PageFlowOuterItemMatch,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum PageFlowLayoutFrameMode {
    Recursive(Option<PageFlowSemanticAnchorTransaction>),
}

#[derive(Clone, Debug)]
pub(super) struct PageFlowExtentMutation {
    pub(super) root_outer_index: usize,
    pub(super) earliest_extent: f32,
    pub(super) latest_extent: f32,
}

#[derive(Default)]
pub(super) struct PageFlowPlanningMutationJournal {
    by_node: HashMap<PageFlowNodeId, PageFlowExtentMutation>,
}

impl PageFlowLayoutFrameToken {
    pub(super) fn new(
        page_id: Arc<str>,
        surface: PageFlowSurfaceKey,
        render_generation: PageFlowRenderGeneration,
        layout_generation: PageFlowLayoutFrameGeneration,
    ) -> Self {
        Self {
            page_id,
            surface,
            render_generation,
            layout_generation,
        }
    }

    pub(crate) fn page_id(&self) -> &str {
        &self.page_id
    }

    pub(crate) const fn surface(&self) -> PageFlowSurfaceKey {
        self.surface
    }

    pub(crate) const fn render_generation(&self) -> PageFlowRenderGeneration {
        self.render_generation
    }

    pub(super) const fn layout_generation(&self) -> PageFlowLayoutFrameGeneration {
        self.layout_generation
    }
}

impl PageFlowLayoutFrameMode {
    pub(super) fn semantic_anchor(&self) -> Option<&PageFlowSemanticAnchorTransaction> {
        let Self::Recursive(anchor) = self;
        anchor.as_ref()
    }

    pub(super) fn semantic_anchor_mut(&mut self) -> Option<&mut PageFlowSemanticAnchorTransaction> {
        let Self::Recursive(anchor) = self;
        anchor.as_mut()
    }
}

impl PageFlowPlanningMutationJournal {
    pub(super) fn record(
        &mut self,
        node_id: PageFlowNodeId,
        root_outer_index: usize,
        previous_extent: f32,
        next_extent: f32,
    ) {
        assert!(previous_extent.is_finite() && previous_extent >= 0.0);
        assert!(next_extent.is_finite() && next_extent >= 0.0);
        self.by_node
            .entry(node_id)
            .and_modify(|mutation| {
                assert_eq!(mutation.root_outer_index, root_outer_index);
                mutation.latest_extent = next_extent;
            })
            .or_insert(PageFlowExtentMutation {
                root_outer_index,
                earliest_extent: previous_extent,
                latest_extent: next_extent,
            });
    }

    pub(super) fn mutations(&self) -> impl Iterator<Item = &PageFlowExtentMutation> {
        self.by_node.values().filter(|mutation| {
            PageFlowExtentChange::between(mutation.earliest_extent, mutation.latest_extent)
                .is_material()
        })
    }
}
