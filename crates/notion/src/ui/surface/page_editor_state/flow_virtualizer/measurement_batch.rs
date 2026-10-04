use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    ops::Range,
    sync::Arc,
};

use super::{
    cache::PageFlowMeasurementKey, PageFlowLayoutFrameToken, PageFlowSemanticAnchorTransaction,
};

const PAGE_DOCUMENT_LEAD_ITEM_COUNT: usize = 1;
const PAGE_FLOW_MEASUREMENT_BATCH_CAPACITY: usize = 128;

#[derive(Clone, Copy)]
pub(super) struct PageFlowMeasuredExtent(f32);

#[derive(Default)]
pub(super) struct PageFlowMeasurementBatch {
    entries: PageFlowMeasurementQueue,
}

#[derive(Default)]
pub(super) struct PageFlowMeasurementContinuation {
    entries: PageFlowMeasurementQueue,
}

#[derive(Default)]
struct PageFlowMeasurementQueue {
    order: VecDeque<PageFlowMeasurementKey>,
    entries: HashMap<PageFlowMeasurementKey, PageFlowMeasuredExtent>,
}

#[derive(Default)]
pub(super) struct PageFlowDirtyRootRanges {
    root_indices: BTreeSet<usize>,
}

pub(crate) struct PageFlowLayoutCommit {
    pub(crate) frame: PageFlowLayoutFrameToken,
    pub(crate) dirty_outer_ranges: Arc<[Range<usize>]>,
    pub(crate) semantic_anchor: Option<PageFlowSemanticAnchorTransaction>,
    pub(crate) measurements_pending: bool,
}

impl PageFlowMeasuredExtent {
    pub(super) fn new(extent: f32) -> Self {
        assert!(extent.is_finite() && extent >= 0.0);
        Self(extent)
    }

    pub(super) const fn pixels(self) -> f32 {
        self.0
    }
}

impl PageFlowMeasurementBatch {
    pub(super) fn from_continuation(continuation: &mut PageFlowMeasurementContinuation) -> Self {
        let mut batch = Self::default();
        while batch.entries.len() < PAGE_FLOW_MEASUREMENT_BATCH_CAPACITY {
            let Some((key, extent)) = continuation.entries.pop_front() else {
                break;
            };
            batch.entries.insert(key, extent);
        }
        batch
    }

    pub(super) fn admit(
        &mut self,
        continuation: &mut PageFlowMeasurementContinuation,
        key: PageFlowMeasurementKey,
        extent: PageFlowMeasuredExtent,
    ) {
        if self.entries.update(&key, extent) || continuation.entries.update(&key, extent) {
            return;
        }
        if self.entries.len() < PAGE_FLOW_MEASUREMENT_BATCH_CAPACITY {
            self.entries.insert(key, extent);
        } else {
            continuation.entries.insert(key, extent);
        }
    }

    pub(super) fn take_entries(&mut self) -> Vec<(PageFlowMeasurementKey, PageFlowMeasuredExtent)> {
        self.entries.take_entries()
    }

    pub(super) fn spill_into(mut self, continuation: &mut PageFlowMeasurementContinuation) {
        continuation.entries.prepend(self.entries.take_entries());
    }
}

impl PageFlowMeasurementContinuation {
    pub(super) fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    pub(super) fn clear(&mut self) {
        self.entries = PageFlowMeasurementQueue::default();
    }
}

impl PageFlowMeasurementQueue {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn update(&mut self, key: &PageFlowMeasurementKey, extent: PageFlowMeasuredExtent) -> bool {
        let Some(current) = self.entries.get_mut(key) else {
            return false;
        };
        *current = extent;
        true
    }

    fn insert(&mut self, key: PageFlowMeasurementKey, extent: PageFlowMeasuredExtent) {
        assert!(!self.entries.contains_key(&key));
        self.order.push_back(key.clone());
        self.entries.insert(key, extent);
    }

    fn pop_front(&mut self) -> Option<(PageFlowMeasurementKey, PageFlowMeasuredExtent)> {
        let key = self.order.pop_front()?;
        let extent = self
            .entries
            .remove(&key)
            .expect("queued Notion flow measurement must retain its extent");
        Some((key, extent))
    }

    fn take_entries(&mut self) -> Vec<(PageFlowMeasurementKey, PageFlowMeasuredExtent)> {
        let mut entries = Vec::with_capacity(self.len());
        while let Some(entry) = self.pop_front() {
            entries.push(entry);
        }
        entries
    }

    fn prepend(&mut self, entries: Vec<(PageFlowMeasurementKey, PageFlowMeasuredExtent)>) {
        let current = self.take_entries();
        for (key, extent) in entries.into_iter().chain(current) {
            if !self.update(&key, extent) {
                self.insert(key, extent);
            }
        }
    }
}

impl PageFlowDirtyRootRanges {
    pub(super) fn extend_indices(&mut self, indices: impl IntoIterator<Item = usize>) {
        self.root_indices.extend(indices);
    }

    pub(super) fn extend_journal(
        &mut self,
        journal: &super::frame::PageFlowPlanningMutationJournal,
    ) {
        self.root_indices.extend(
            journal
                .mutations()
                .map(|mutation| mutation.root_outer_index),
        );
    }

    pub(super) fn outer_list_ranges(&self) -> Arc<[Range<usize>]> {
        let mut ranges = Vec::<Range<usize>>::new();
        for root_index in self.root_indices.iter().copied() {
            let outer_index = root_index + PAGE_DOCUMENT_LEAD_ITEM_COUNT;
            if let Some(previous) = ranges
                .last_mut()
                .filter(|previous| previous.end == outer_index)
            {
                previous.end += 1;
            } else {
                ranges.push(outer_index..outer_index + 1);
            }
        }
        ranges.into()
    }
}
