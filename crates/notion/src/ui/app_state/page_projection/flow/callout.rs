use std::sync::Arc;

use super::super::{LoadedCardPageVisibleRow, PageVisibleRowLayouts};
use super::{PageFlowColumns, PageFlowNode, PageFlowSequence};

mod presentation;
mod transform;

use presentation::ROOT_LEADING_INSET;
pub(crate) use presentation::{
    PageFlowCalloutPresentationSpec, PageFlowCalloutSegment, PageFlowColumnsPresentationSpec,
    PageFlowExtentEnvelope,
};
pub(crate) use transform::{
    PageFlowDecoratorContentOriginTransform, PageFlowDecoratorLayerLayout, PAGE_FLOW_BLOCK_INDENT,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct PageFlowCalloutPath(Option<Arc<PageFlowCalloutPathNode>>);

#[derive(Debug)]
struct PageFlowCalloutPathNode {
    parent: PageFlowCalloutPath,
    callout_block_id: Arc<str>,
    visible_row_index: usize,
    depth: usize,
}

#[derive(Clone, Debug)]
struct PageFlowCalloutPathLayer {
    callout_block_id: Arc<str>,
    visible_row_index: usize,
    depth: usize,
}

impl PageFlowCalloutPath {
    pub(super) fn push(&self, callout_block_id: Arc<str>, visible_row_index: usize) -> Self {
        Self(Some(Arc::new(PageFlowCalloutPathNode {
            parent: self.clone(),
            callout_block_id,
            visible_row_index,
            depth: self.depth() + 1,
        })))
    }

    fn parent(&self) -> Option<&Self> {
        self.0.as_ref().map(|node| &node.parent)
    }

    fn depth(&self) -> usize {
        self.0.as_ref().map_or(0, |node| node.depth)
    }

    pub(crate) fn common_prefix(&self, other: &Self) -> Self {
        let mut left = self.clone();
        let mut right = other.clone();
        while left.depth() > right.depth() {
            left = left.parent().cloned().unwrap_or_default();
        }
        while right.depth() > left.depth() {
            right = right.parent().cloned().unwrap_or_default();
        }
        while !left.same_tail(&right) {
            left = left.parent().cloned().unwrap_or_default();
            right = right.parent().cloned().unwrap_or_default();
        }
        left
    }

    fn suffix_after(&self, common: &Self) -> Vec<PageFlowCalloutPathLayer> {
        assert!(
            self.common_prefix(common).same_tail(common),
            "Callout common path must be a prefix"
        );
        let mut current = self.clone();
        let mut suffix = Vec::with_capacity(self.depth() - common.depth());
        while current.depth() > common.depth() {
            suffix.push(
                current
                    .layer()
                    .expect("non-root Callout path must have a layer"),
            );
            current = current
                .parent()
                .cloned()
                .expect("Callout suffix must reach its declared common prefix");
        }
        assert!(current.same_tail(common));
        suffix.reverse();
        suffix
    }

    fn layer(&self) -> Option<PageFlowCalloutPathLayer> {
        self.0.as_ref().map(|node| PageFlowCalloutPathLayer {
            callout_block_id: node.callout_block_id.clone(),
            visible_row_index: node.visible_row_index,
            depth: node.depth,
        })
    }

    fn same_tail(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(left), Some(right)) => {
                left.depth == right.depth
                    && left.visible_row_index == right.visible_row_index
                    && left.callout_block_id == right.callout_block_id
            }
            (None, Some(_)) | (Some(_), None) => false,
        }
    }

    pub(crate) fn painted_prefix(&self) -> PageFlowPaintedCalloutPrefix {
        PageFlowPaintedCalloutPrefix(self.clone())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowPaintedCalloutPrefix(PageFlowCalloutPath);

impl PageFlowPaintedCalloutPrefix {
    pub(crate) fn assert_matches_visible_rows(&self, visible_row_indices: &[usize]) {
        assert_eq!(self.0.depth(), visible_row_indices.len());
        assert_eq!(
            self.0 .0.as_ref().map(|node| node.visible_row_index),
            visible_row_indices.last().copied()
        );
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowCalloutLayoutRevision(Arc<PageFlowCalloutLayoutRevisionIdentity>);

#[derive(Debug)]
struct PageFlowCalloutLayoutRevisionIdentity;

impl PageFlowCalloutLayoutRevision {
    pub(crate) fn fresh() -> Self {
        Self(Arc::new(PageFlowCalloutLayoutRevisionIdentity))
    }
}

impl PartialEq for PageFlowCalloutLayoutRevision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PageFlowCalloutLayoutRevision {}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowDecoratorPlanRevision(Arc<PageFlowDecoratorPlanRevisionIdentity>);

#[derive(Debug)]
struct PageFlowDecoratorPlanRevisionIdentity;

impl PageFlowDecoratorPlanRevision {
    fn fresh() -> Self {
        Self(Arc::new(PageFlowDecoratorPlanRevisionIdentity))
    }
}

impl PartialEq for PageFlowDecoratorPlanRevision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PageFlowDecoratorPlanRevision {}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowDecoratorLayer {
    pub(crate) callout_block_id: Arc<str>,
    pub(crate) visible_row_index: usize,
    pub(crate) depth: usize,
    pub(crate) presentation: PageFlowCalloutPresentationSpec,
    pub(crate) layout: PageFlowDecoratorLayerLayout,
}

#[derive(Clone, Debug)]
pub(crate) struct PageFlowDecoratorPlan {
    pub(crate) layers: Arc<[PageFlowDecoratorLayer]>,
    pub(crate) root_leading: bool,
    pub(crate) envelope: PageFlowExtentEnvelope,
    pub(crate) content_origin: PageFlowDecoratorContentOriginTransform,
    pub(crate) revision: PageFlowDecoratorPlanRevision,
}

impl PageFlowDecoratorPlan {
    pub(super) fn unresolved() -> Self {
        Self::new(Vec::new(), false)
    }

    fn new(layers: Vec<PageFlowDecoratorLayer>, root_leading: bool) -> Self {
        let mut envelope = PageFlowExtentEnvelope::default();
        for layer in layers.iter().rev() {
            envelope = envelope.wrapped_by(layer.presentation.extent_envelope());
        }
        if root_leading {
            envelope = envelope.wrapped_by(PageFlowExtentEnvelope::new(ROOT_LEADING_INSET, 0.0));
        }
        let leading = if root_leading {
            ROOT_LEADING_INSET
        } else {
            0.0
        };
        let content_origin = PageFlowDecoratorContentOriginTransform::from_layers(&layers, leading);
        Self {
            layers: layers.into(),
            root_leading,
            envelope,
            content_origin,
            revision: PageFlowDecoratorPlanRevision::fresh(),
        }
    }

    pub(crate) fn root_leading_inset(&self) -> f32 {
        if self.root_leading {
            ROOT_LEADING_INSET
        } else {
            0.0
        }
    }
}

pub(super) fn finalize_decorator_plans(
    sequence: &mut PageFlowSequence,
    root: bool,
    rows: &[LoadedCardPageVisibleRow],
    layouts: &PageVisibleRowLayouts,
) {
    let suffixes = sequence
        .nodes
        .iter()
        .map(|node| node_callout_path(node).suffix_after(&sequence.common_callout_path))
        .collect::<Vec<_>>();
    let previous_prefixes = joined_prefixes(&suffixes, true);
    let next_prefixes = joined_prefixes(&suffixes, false);
    for (index, node) in Arc::make_mut(&mut sequence.nodes).iter_mut().enumerate() {
        let layers = suffixes[index]
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| PageFlowDecoratorLayer {
                callout_block_id: layer.callout_block_id.clone(),
                visible_row_index: layer.visible_row_index,
                depth: layer.depth,
                presentation: PageFlowCalloutPresentationSpec::notion(segment(
                    layer_index < previous_prefixes[index],
                    layer_index < next_prefixes[index],
                )),
                layout: PageFlowDecoratorLayerLayout::from_row(
                    rows,
                    layouts,
                    layer.visible_row_index,
                ),
            })
            .collect();
        *node_decorator_plan_mut(node) = PageFlowDecoratorPlan::new(layers, root && index == 0);
        if let PageFlowNode::Columns(columns) = node {
            for column in Arc::make_mut(&mut columns.columns) {
                finalize_decorator_plans(&mut column.sequence, false, rows, layouts);
            }
        }
    }
}

fn joined_prefixes(suffixes: &[Vec<PageFlowCalloutPathLayer>], previous: bool) -> Vec<usize> {
    (0..suffixes.len())
        .map(|index| {
            let neighbor = if previous {
                index.checked_sub(1)
            } else {
                (index + 1 < suffixes.len()).then_some(index + 1)
            };
            neighbor.map_or(0, |neighbor| {
                common_layer_count(&suffixes[index], &suffixes[neighbor])
            })
        })
        .collect()
}

fn common_layer_count(
    left: &[PageFlowCalloutPathLayer],
    right: &[PageFlowCalloutPathLayer],
) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| {
            left.depth == right.depth
                && left.visible_row_index == right.visible_row_index
                && left.callout_block_id == right.callout_block_id
        })
        .count()
}

fn segment(joins_previous: bool, joins_next: bool) -> PageFlowCalloutSegment {
    match (joins_previous, joins_next) {
        (false, false) => PageFlowCalloutSegment::Single,
        (false, true) => PageFlowCalloutSegment::First,
        (true, true) => PageFlowCalloutSegment::Middle,
        (true, false) => PageFlowCalloutSegment::Last,
    }
}

fn node_callout_path(node: &PageFlowNode) -> &PageFlowCalloutPath {
    match node {
        PageFlowNode::Section(section) => &section.callout_path,
        PageFlowNode::Columns(columns) => &columns.common_callout_path,
    }
}

fn node_decorator_plan_mut(node: &mut PageFlowNode) -> &mut PageFlowDecoratorPlan {
    match node {
        PageFlowNode::Section(section) => &mut section.decorator_plan,
        PageFlowNode::Columns(columns) => &mut columns.decorator_plan,
    }
}

impl PageFlowColumns {
    pub(crate) fn extent_envelope(&self) -> PageFlowExtentEnvelope {
        PageFlowColumnsPresentationSpec::NOTION
            .extent_envelope()
            .wrapped_by(self.decorator_plan.envelope)
    }
}

impl super::super::LoadedCardPageData {
    pub(crate) fn block_is_within_callout(
        &self,
        block_id: &str,
        callout_visible_row_index: usize,
    ) -> bool {
        let Some(block_index) = self.block_indices.get(block_id).copied() else {
            return false;
        };
        self.visible_rows
            .binary_search_by_key(&block_index, |row| row.block_index)
            .ok()
            .is_some_and(|row_index| {
                self.callout_layer_row_indices(row_index)
                    .contains(&callout_visible_row_index)
            })
    }
}
