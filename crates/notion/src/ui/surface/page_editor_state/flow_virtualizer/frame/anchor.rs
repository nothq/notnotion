use std::{collections::HashSet, sync::Arc};

use crate::ui::PageDocumentOuterItemId;

use super::super::PageFlowPinTarget;
use super::{PageFlowOuterItemMatch, PageFlowOuterItemRemap};

#[derive(Clone, Copy, Debug)]
pub(crate) struct PageFlowOuterLocalY(f32);

#[derive(Clone, Copy, Debug)]
pub(crate) struct PageFlowViewportRelativeY(f32);

#[derive(Clone, Copy, Debug)]
pub(crate) struct PageFlowResolvedOuterOffset {
    pub(crate) outer_index: usize,
    pub(crate) offset_in_item: PageFlowOuterLocalY,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowOuterItemExtent {
    key: PageDocumentOuterItemId,
    extent: PageFlowOuterLocalY,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowOuterExtentAuthority {
    item_count: usize,
    first_index: usize,
    items: Arc<[PageFlowOuterItemExtent]>,
    prefix_extents: Arc<[f32]>,
}

#[derive(Clone, Debug)]
pub(crate) enum PageFlowLogicalViewportBasis {
    Item {
        key: PageDocumentOuterItemId,
        index: usize,
        offset_in_item: PageFlowOuterLocalY,
    },
    Tail {
        item_count: usize,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowSemanticAnchorBaseline {
    pub(crate) target: PageFlowPinTarget,
    pub(crate) viewport_relative_y: PageFlowViewportRelativeY,
    pub(crate) intent: PageFlowSemanticAnchorIntent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageFlowSemanticAnchorIntent {
    ViewportPreservation,
    RevealFocus,
    PointerTableFocus,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowSemanticAnchorProjection {
    pub(crate) target: PageFlowPinTarget,
    pub(crate) outer_item: PageDocumentOuterItemId,
    pub(crate) outer_index: usize,
    pub(crate) outer_local_y: PageFlowOuterLocalY,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowSemanticAnchorTransaction {
    pub(crate) baseline: PageFlowSemanticAnchorBaseline,
    pub(crate) latest_projection: Option<PageFlowSemanticAnchorProjection>,
    pub(crate) outer_remap: PageFlowOuterItemRemap,
    pub(crate) outer_order_changed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowDeferredScrollbarAnchor {
    transaction: PageFlowSemanticAnchorTransaction,
    latest_viewport: PageFlowLogicalViewportBasis,
}

impl PageFlowOuterLocalY {
    pub(crate) fn new(value: f32) -> Self {
        assert!(value.is_finite() && value >= 0.0);
        Self(if value == 0.0 { 0.0 } else { value })
    }

    pub(crate) const fn pixels(self) -> f32 {
        self.0
    }
}

impl PageFlowViewportRelativeY {
    pub(crate) fn new(value: f32) -> Self {
        assert!(value.is_finite());
        Self(if value == 0.0 { 0.0 } else { value })
    }

    pub(crate) const fn pixels(self) -> f32 {
        self.0
    }
}

impl PageFlowOuterItemExtent {
    pub(crate) fn from_measured_pixels(key: PageDocumentOuterItemId, extent: f32) -> Self {
        Self {
            key,
            extent: PageFlowOuterLocalY::new(extent),
        }
    }

    pub(crate) const fn pixels(&self) -> f32 {
        self.extent.pixels()
    }
}

impl PageFlowLogicalViewportBasis {
    pub(crate) fn item(key: PageDocumentOuterItemId, index: usize, offset_in_item: f32) -> Self {
        Self::Item {
            key,
            index,
            offset_in_item: PageFlowOuterLocalY::new(offset_in_item),
        }
    }

    pub(crate) const fn tail(item_count: usize) -> Self {
        Self::Tail { item_count }
    }
}

impl PageFlowOuterExtentAuthority {
    pub(crate) fn from_measured_window(
        item_count: usize,
        first_index: usize,
        items: Vec<PageFlowOuterItemExtent>,
    ) -> Self {
        assert!(!items.is_empty());
        let window_end = first_index
            .checked_add(items.len())
            .expect("Notion flow outer extent window must not overflow");
        assert!(window_end <= item_count);
        let mut keys = HashSet::with_capacity(items.len());
        let mut prefix_extents = Vec::with_capacity(items.len() + 1);
        let mut total = 0.0_f32;
        prefix_extents.push(total);
        for item in &items {
            assert!(keys.insert(item.key.clone()));
            total += item.extent.pixels();
            assert!(total.is_finite());
            prefix_extents.push(total);
        }
        Self {
            item_count,
            first_index,
            items: items.into(),
            prefix_extents: prefix_extents.into(),
        }
    }

    pub(crate) fn semantic_baseline(
        &self,
        projection: &PageFlowSemanticAnchorProjection,
        viewport: &PageFlowLogicalViewportBasis,
    ) -> PageFlowSemanticAnchorBaseline {
        let target_y = self.projection_y(projection);
        let viewport_y = self.viewport_y(viewport);
        PageFlowSemanticAnchorBaseline {
            target: projection.target.clone(),
            viewport_relative_y: PageFlowViewportRelativeY::new(target_y - viewport_y),
            intent: PageFlowSemanticAnchorIntent::ViewportPreservation,
        }
    }

    fn resolve_target(
        &self,
        baseline: &PageFlowSemanticAnchorBaseline,
        projection: &PageFlowSemanticAnchorProjection,
    ) -> Option<PageFlowResolvedOuterOffset> {
        assert_eq!(baseline.target, projection.target);
        let desired_top = self.projection_y(projection) - baseline.viewport_relative_y.pixels();
        if desired_top < 0.0 {
            return (self.first_index == 0).then(|| self.resolved(0, 0.0));
        }
        let window_count = self.items.len();
        let window_end = self.first_index + window_count;
        let total_extent = self.prefix_extents[window_count];
        if desired_top > total_extent {
            return (window_end == self.item_count).then(|| self.resolved(self.item_count, 0.0));
        }
        if desired_top == total_extent {
            return Some(self.resolved(window_end, 0.0));
        }
        let local_index = self.prefix_extents[..window_count]
            .partition_point(|prefix| *prefix <= desired_top)
            .saturating_sub(1);
        Some(self.resolved(
            self.first_index + local_index,
            desired_top - self.prefix_extents[local_index],
        ))
    }

    fn resolve_fallback(&self, remap: &PageFlowOuterItemRemap) -> PageFlowResolvedOuterOffset {
        let index = match remap {
            PageFlowOuterItemRemap::Start => 0,
            PageFlowOuterItemRemap::Tail { item_count } => {
                assert_eq!(*item_count, self.item_count);
                *item_count
            }
            PageFlowOuterItemRemap::Item {
                key,
                new_index,
                match_kind,
            } => {
                let item = self.item(*new_index);
                assert_eq!(&item.key, key);
                let offset = match match_kind {
                    PageFlowOuterItemMatch::Exact { captured_offset } => {
                        captured_offset.pixels().min(item.extent.pixels())
                    }
                    PageFlowOuterItemMatch::NearestSurvivor => 0.0,
                };
                return self.resolved(*new_index, offset);
            }
        };
        self.resolved(index, 0.0)
    }

    fn projection_y(&self, projection: &PageFlowSemanticAnchorProjection) -> f32 {
        let item = self.item(projection.outer_index);
        assert_eq!(&item.key, &projection.outer_item);
        assert!(projection.outer_local_y.pixels() <= item.extent.pixels());
        self.prefix_extents[self.local_index(projection.outer_index)]
            + projection.outer_local_y.pixels()
    }

    fn viewport_y(&self, viewport: &PageFlowLogicalViewportBasis) -> f32 {
        match viewport {
            PageFlowLogicalViewportBasis::Item {
                key,
                index,
                offset_in_item,
            } => {
                let item = self.item(*index);
                assert_eq!(&item.key, key);
                self.prefix_extents[self.local_index(*index)]
                    + offset_in_item.pixels().min(item.extent.pixels())
            }
            PageFlowLogicalViewportBasis::Tail { item_count } => {
                assert_eq!(*item_count, self.item_count);
                assert_eq!(self.first_index + self.items.len(), self.item_count);
                self.prefix_extents[self.items.len()]
            }
        }
    }

    fn item(&self, index: usize) -> &PageFlowOuterItemExtent {
        self.items
            .get(self.local_index(index))
            .expect("Notion flow outer extent index must remain authoritative")
    }

    fn local_index(&self, index: usize) -> usize {
        let local_index = index
            .checked_sub(self.first_index)
            .expect("Notion flow outer extent window must cover the requested index");
        assert!(local_index < self.items.len());
        local_index
    }

    fn resolved(&self, outer_index: usize, offset_in_item: f32) -> PageFlowResolvedOuterOffset {
        assert!(outer_index <= self.item_count);
        PageFlowResolvedOuterOffset {
            outer_index,
            offset_in_item: PageFlowOuterLocalY::new(offset_in_item),
        }
    }
}

impl PageFlowSemanticAnchorTransaction {
    pub(crate) fn new(
        baseline: PageFlowSemanticAnchorBaseline,
        outer_remap: PageFlowOuterItemRemap,
        outer_order_changed: bool,
    ) -> Self {
        Self {
            baseline,
            latest_projection: None,
            outer_remap,
            outer_order_changed,
        }
    }

    pub(crate) fn update_projection(&mut self, projection: PageFlowSemanticAnchorProjection) {
        assert_eq!(self.baseline.target, projection.target);
        self.latest_projection = Some(projection);
    }

    pub(crate) fn resolved_outer_offset(
        &self,
        extents: &PageFlowOuterExtentAuthority,
    ) -> Option<PageFlowResolvedOuterOffset> {
        if let Some(projection) = &self.latest_projection {
            return extents.resolve_target(&self.baseline, projection);
        }
        Some(extents.resolve_fallback(&self.outer_remap))
    }
}

impl PageFlowDeferredScrollbarAnchor {
    pub(crate) fn new(
        transaction: PageFlowSemanticAnchorTransaction,
        viewport: PageFlowLogicalViewportBasis,
    ) -> Self {
        Self {
            transaction,
            latest_viewport: viewport,
        }
    }

    pub(crate) fn update_latest(
        &mut self,
        transaction: PageFlowSemanticAnchorTransaction,
        viewport: PageFlowLogicalViewportBasis,
    ) {
        if !self.is_compatible_with(&transaction) {
            *self = Self::new(transaction, viewport);
            return;
        }
        if transaction.baseline.intent == PageFlowSemanticAnchorIntent::ViewportPreservation {
            self.transaction.baseline = transaction.baseline;
        }
        self.transaction.latest_projection = transaction
            .latest_projection
            .or_else(|| self.transaction.latest_projection.clone());
        self.transaction.outer_remap = transaction.outer_remap;
        self.transaction.outer_order_changed |= transaction.outer_order_changed;
        self.latest_viewport = viewport;
    }

    pub(crate) fn target(&self) -> &PageFlowPinTarget {
        &self.transaction.baseline.target
    }

    pub(crate) fn is_compatible_with(
        &self,
        transaction: &PageFlowSemanticAnchorTransaction,
    ) -> bool {
        self.transaction.baseline.target == transaction.baseline.target
            && self.transaction.baseline.intent == transaction.baseline.intent
    }

    pub(crate) fn resume(
        mut self,
        mut current: PageFlowSemanticAnchorTransaction,
        viewport: PageFlowLogicalViewportBasis,
        extents: &PageFlowOuterExtentAuthority,
    ) -> Option<PageFlowSemanticAnchorTransaction> {
        self.latest_viewport = viewport;
        if !self.is_compatible_with(&current) {
            return None;
        }
        current.outer_order_changed |= self.transaction.outer_order_changed;
        current.latest_projection = current
            .latest_projection
            .or(self.transaction.latest_projection);
        if current.baseline.intent == PageFlowSemanticAnchorIntent::ViewportPreservation {
            let projection = current.latest_projection.as_ref()?;
            current.baseline = extents.semantic_baseline(projection, &self.latest_viewport);
        } else {
            current.baseline = self.transaction.baseline;
        }
        Some(current)
    }
}
