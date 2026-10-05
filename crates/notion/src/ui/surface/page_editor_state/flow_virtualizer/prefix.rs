const PAGE_FLOW_MATERIAL_EXTENT_DELTA: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PageFlowExtentChange {
    Unchanged,
    WithinTolerance,
    Material,
}

impl PageFlowExtentChange {
    pub(super) fn between(previous: f32, next: f32) -> Self {
        assert!(previous.is_finite() && previous >= 0.0);
        assert!(next.is_finite() && next >= 0.0);
        let delta = (next - previous).abs();
        if delta == 0.0 {
            Self::Unchanged
        } else if delta <= PAGE_FLOW_MATERIAL_EXTENT_DELTA {
            Self::WithinTolerance
        } else {
            Self::Material
        }
    }

    pub(super) const fn is_material(self) -> bool {
        matches!(self, Self::Material)
    }

    pub(super) const fn admits_exact_observation(self) -> bool {
        !matches!(self, Self::WithinTolerance)
    }
}

#[derive(Clone)]
pub(super) struct PageFlowPrefixExtents {
    values: Vec<f32>,
    tree: Vec<f32>,
}

pub(super) struct PageFlowMaxExtents {
    len: usize,
    leaf_start: usize,
    tree: Vec<f32>,
    canonical_max: f32,
}

impl PageFlowMaxExtents {
    pub(super) fn new(values: impl IntoIterator<Item = f32>) -> Self {
        let values = values.into_iter().collect::<Vec<_>>();
        let leaf_start = values.len().max(1).next_power_of_two();
        let mut tree = vec![0.0; leaf_start * 2];
        tree[leaf_start..leaf_start + values.len()].copy_from_slice(&values);
        for index in (1..leaf_start).rev() {
            tree[index] = tree[index * 2].max(tree[index * 2 + 1]);
        }
        let canonical_max = tree[1];
        Self {
            len: values.len(),
            leaf_start,
            tree,
            canonical_max,
        }
    }

    pub(super) fn set(&mut self, index: usize, value: f32) -> PageFlowExtentChange {
        assert!(index < self.len);
        assert!(value.is_finite() && value >= 0.0);
        let mut tree_index = self.leaf_start + index;
        let leaf_change = PageFlowExtentChange::between(self.tree[tree_index], value);
        if !leaf_change.is_material() {
            return leaf_change;
        }
        self.tree[tree_index] = value;
        while tree_index > 1 {
            tree_index /= 2;
            self.tree[tree_index] = self.tree[tree_index * 2].max(self.tree[tree_index * 2 + 1]);
        }
        let max_change = PageFlowExtentChange::between(self.canonical_max, self.tree[1]);
        if max_change.is_material() {
            self.canonical_max = self.tree[1];
        }
        max_change
    }

    pub(super) fn max(&self) -> f32 {
        self.canonical_max
    }
}

impl PageFlowPrefixExtents {
    pub(super) fn new(values: impl IntoIterator<Item = f32>) -> Self {
        let values = values.into_iter().collect::<Vec<_>>();
        assert!(values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0));
        let mut tree = vec![0.0; values.len() + 1];
        for tree_index in 1..tree.len() {
            tree[tree_index] += values[tree_index - 1];
            let parent = tree_index + (tree_index & tree_index.wrapping_neg());
            if parent < tree.len() {
                tree[parent] += tree[tree_index];
            }
        }
        Self { values, tree }
    }

    pub(super) fn len(&self) -> usize {
        self.values.len()
    }

    pub(super) fn value(&self, index: usize) -> f32 {
        self.values[index]
    }

    pub(super) fn set(&mut self, index: usize, value: f32) -> PageFlowExtentChange {
        assert!(value.is_finite() && value >= 0.0);
        let previous = self.values[index];
        let change = PageFlowExtentChange::between(previous, value);
        if !change.is_material() {
            return change;
        }
        let delta = value - previous;
        self.values[index] = value;
        let mut tree_index = index + 1;
        while tree_index < self.tree.len() {
            self.tree[tree_index] += delta;
            tree_index += tree_index & tree_index.wrapping_neg();
        }
        change
    }

    pub(super) fn prefix(&self, end: usize) -> f32 {
        let mut sum = 0.0;
        let mut tree_index = end.min(self.values.len());
        while tree_index > 0 {
            sum += self.tree[tree_index];
            tree_index &= tree_index - 1;
        }
        sum
    }

    pub(super) fn total(&self) -> f32 {
        self.prefix(self.values.len())
    }

    pub(super) fn index_at_offset(&self, offset: f32) -> usize {
        if self.values.is_empty() || offset <= 0.0 {
            return 0;
        }
        let target = offset.min(self.total());
        let mut index = 0;
        let mut sum = 0.0;
        let mut step = self.tree.len().next_power_of_two() >> 1;
        while step > 0 {
            let candidate = index + step;
            if candidate < self.tree.len() && sum + self.tree[candidate] <= target {
                index = candidate;
                sum += self.tree[candidate];
            }
            step >>= 1;
        }
        index.min(self.values.len().saturating_sub(1))
    }
}
