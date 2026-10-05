use std::collections::{HashMap, VecDeque};

use crate::ui::{PageDocumentUnitKey, PageFlowNodeId};

use super::{metrics::PageFlowSectionLayoutWitness, PageFlowExactLayoutWidth};

pub(super) const PAGE_FLOW_MEASUREMENTS_PER_PAGE: usize = 2_048;
pub(super) const PAGE_FLOW_MEASUREMENTS_GLOBAL: usize = 4_096;
const PAGE_FLOW_WIDTHS_PER_NODE: usize = 2;
const PAGE_FLOW_WITNESSES_PER_WIDTH: usize = 2;
const PAGE_FLOW_RECENCY_MIN_CAPACITY: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct PageFlowMeasurementKey {
    pub(super) node_id: PageFlowNodeId,
    pub(super) width: PageFlowExactLayoutWidth,
}

#[derive(Clone)]
struct PageFlowMeasurementAddress {
    node_id: PageFlowNodeId,
    width: PageFlowExactLayoutWidth,
    witness: PageFlowSectionLayoutWitness,
}

struct PageFlowMeasurementEntry {
    witness: PageFlowSectionLayoutWitness,
    extent: f32,
    touch: u64,
}

struct PageFlowWidthMeasurements {
    width: PageFlowExactLayoutWidth,
    entries: Vec<PageFlowMeasurementEntry>,
}

#[derive(Default)]
struct PageFlowNodeMeasurements {
    widths: Vec<PageFlowWidthMeasurements>,
}

#[derive(Default)]
pub(super) struct PageFlowMeasurementCache {
    nodes: HashMap<PageFlowNodeId, PageFlowNodeMeasurements>,
    order: VecDeque<(PageFlowMeasurementAddress, u64)>,
    next_touch: u64,
    len: usize,
}

impl PageFlowMeasurementCache {
    pub(super) fn exact_extent(
        &mut self,
        key: &PageFlowMeasurementKey,
        witness: &PageFlowSectionLayoutWitness,
    ) -> Option<f32> {
        let extent = self.entry(key, witness)?.extent;
        self.touch(PageFlowMeasurementAddress {
            node_id: key.node_id.clone(),
            width: key.width,
            witness: witness.clone(),
        });
        Some(extent)
    }

    pub(super) fn insert(
        &mut self,
        key: PageFlowMeasurementKey,
        witness: PageFlowSectionLayoutWitness,
        extent: f32,
    ) {
        assert!(extent.is_finite() && extent >= 0.0);
        if let Some(entry) = self.entry_mut(&key, &witness) {
            entry.extent = extent;
            self.touch(PageFlowMeasurementAddress {
                node_id: key.node_id,
                width: key.width,
                witness,
            });
            return;
        }
        self.prepare_width(&key);
        self.prepare_witness(&key);
        let touch = self.allocate_touch();
        self.width_mut(&key)
            .expect("prepared Notion flow measurement width must exist")
            .entries
            .push(PageFlowMeasurementEntry {
                witness: witness.clone(),
                extent,
                touch,
            });
        self.len += 1;
        self.order.push_back((
            PageFlowMeasurementAddress {
                node_id: key.node_id,
                width: key.width,
                witness,
            },
            touch,
        ));
        while self.len > PAGE_FLOW_MEASUREMENTS_PER_PAGE {
            assert!(self.remove_oldest());
        }
        self.compact_recency();
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn remove_oldest(&mut self) -> bool {
        self.prune_stale_order();
        let Some((address, touch)) = self.order.pop_front() else {
            return false;
        };
        assert_eq!(
            self.entry_at(&address).map(|entry| entry.touch),
            Some(touch)
        );
        self.remove_entry(&address);
        self.compact_recency();
        true
    }

    pub(super) fn remove_unit(&mut self, unit_key: &PageDocumentUnitKey) {
        let mut removed = 0;
        self.nodes.retain(|_, node| {
            node.widths.retain_mut(|width| {
                let previous = width.entries.len();
                width
                    .entries
                    .retain(|entry| !entry.witness.contains_unit(unit_key));
                removed += previous - width.entries.len();
                !width.entries.is_empty()
            });
            !node.widths.is_empty()
        });
        if removed == 0 {
            return;
        }
        self.len -= removed;
        self.order
            .retain(|(address, _)| !address.witness.contains_unit(unit_key));
    }

    fn touch(&mut self, address: PageFlowMeasurementAddress) {
        let touch = self.allocate_touch();
        self.entry_at_mut(&address)
            .expect("touched Notion flow measurement must exist")
            .touch = touch;
        self.order.push_back((address, touch));
        self.compact_recency();
    }

    fn prune_stale_order(&mut self) {
        while self.order.front().is_some_and(|(address, touch)| {
            self.entry_at(address)
                .is_none_or(|entry| entry.touch != *touch)
        }) {
            self.order.pop_front();
        }
    }

    fn compact_recency(&mut self) {
        let capacity = self
            .len
            .saturating_mul(2)
            .max(PAGE_FLOW_RECENCY_MIN_CAPACITY);
        if self.order.len() <= capacity {
            return;
        }
        let nodes = &self.nodes;
        self.order.retain(|(address, touch)| {
            entry_in(nodes, address).is_some_and(|entry| entry.touch == *touch)
        });
        assert_eq!(self.order.len(), self.len);
    }

    fn allocate_touch(&mut self) -> u64 {
        let touch = self.next_touch;
        self.next_touch = self
            .next_touch
            .checked_add(1)
            .expect("Notion flow measurement recency must not overflow");
        touch
    }

    fn prepare_width(&mut self, key: &PageFlowMeasurementKey) {
        if self.width(key).is_some() {
            return;
        }
        let node = self.nodes.entry(key.node_id.clone()).or_default();
        if node.widths.len() == PAGE_FLOW_WIDTHS_PER_NODE {
            let oldest = node
                .widths
                .iter()
                .enumerate()
                .min_by_key(|(_, width)| width.entries.iter().map(|entry| entry.touch).max())
                .map(|(index, _)| index)
                .expect("bounded Notion flow widths must be nonempty");
            let removed = node.widths.remove(oldest);
            self.len -= removed.entries.len();
        }
        node.widths.push(PageFlowWidthMeasurements {
            width: key.width,
            entries: Vec::new(),
        });
    }

    fn prepare_witness(&mut self, key: &PageFlowMeasurementKey) {
        let width = self
            .width_mut(key)
            .expect("prepared Notion flow measurement width must exist");
        if width.entries.len() < PAGE_FLOW_WITNESSES_PER_WIDTH {
            return;
        }
        let oldest = width
            .entries
            .iter()
            .enumerate()
            .min_by_key(|(_, entry)| entry.touch)
            .map(|(index, _)| index)
            .expect("bounded Notion flow witnesses must be nonempty");
        width.entries.remove(oldest);
        self.len -= 1;
    }

    fn remove_entry(&mut self, address: &PageFlowMeasurementAddress) {
        let remove_node = {
            let node = self
                .nodes
                .get_mut(&address.node_id)
                .expect("removed Notion flow measurement node must exist");
            let width_index = node
                .widths
                .iter()
                .position(|width| width.width == address.width)
                .expect("removed Notion flow measurement width must exist");
            let entry_index = node.widths[width_index]
                .entries
                .iter()
                .position(|entry| entry.witness == address.witness)
                .expect("removed Notion flow measurement witness must exist");
            node.widths[width_index].entries.remove(entry_index);
            if node.widths[width_index].entries.is_empty() {
                node.widths.remove(width_index);
            }
            node.widths.is_empty()
        };
        if remove_node {
            self.nodes.remove(&address.node_id);
        }
        self.len -= 1;
    }

    fn width(&self, key: &PageFlowMeasurementKey) -> Option<&PageFlowWidthMeasurements> {
        self.nodes
            .get(&key.node_id)?
            .widths
            .iter()
            .find(|width| width.width == key.width)
    }

    fn width_mut(
        &mut self,
        key: &PageFlowMeasurementKey,
    ) -> Option<&mut PageFlowWidthMeasurements> {
        self.nodes
            .get_mut(&key.node_id)?
            .widths
            .iter_mut()
            .find(|width| width.width == key.width)
    }

    fn entry(
        &self,
        key: &PageFlowMeasurementKey,
        witness: &PageFlowSectionLayoutWitness,
    ) -> Option<&PageFlowMeasurementEntry> {
        self.width(key)?
            .entries
            .iter()
            .find(|entry| entry.witness == *witness)
    }

    fn entry_mut(
        &mut self,
        key: &PageFlowMeasurementKey,
        witness: &PageFlowSectionLayoutWitness,
    ) -> Option<&mut PageFlowMeasurementEntry> {
        self.width_mut(key)?
            .entries
            .iter_mut()
            .find(|entry| entry.witness == *witness)
    }

    fn entry_at(&self, address: &PageFlowMeasurementAddress) -> Option<&PageFlowMeasurementEntry> {
        entry_in(&self.nodes, address)
    }

    fn entry_at_mut(
        &mut self,
        address: &PageFlowMeasurementAddress,
    ) -> Option<&mut PageFlowMeasurementEntry> {
        self.nodes
            .get_mut(&address.node_id)?
            .widths
            .iter_mut()
            .find(|width| width.width == address.width)?
            .entries
            .iter_mut()
            .find(|entry| entry.witness == address.witness)
    }
}

fn entry_in<'a>(
    nodes: &'a HashMap<PageFlowNodeId, PageFlowNodeMeasurements>,
    address: &PageFlowMeasurementAddress,
) -> Option<&'a PageFlowMeasurementEntry> {
    nodes
        .get(&address.node_id)?
        .widths
        .iter()
        .find(|width| width.width == address.width)?
        .entries
        .iter()
        .find(|entry| entry.witness == address.witness)
}
