use std::collections::HashSet;

use crate::ui::{
    PageDocumentOuterItemId, PageDocumentUnitKey, PageFlowNodeId, PageFlowNodePath,
    PageFlowProjection,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PageFlowPinTarget {
    Unit(PageDocumentUnitKey),
    Node(PageFlowNodeId),
    Outer(PageDocumentOuterItemId),
}

#[derive(Clone, Default)]
pub(crate) struct PageFlowPinSet {
    pub(crate) viewport_anchor: Option<PageFlowPinTarget>,
    pub(crate) active_input: Option<PageFlowPinTarget>,
    pub(crate) pending_focus: Option<PageFlowPinTarget>,
    pub(crate) rich_text_composition: Option<PageFlowPinTarget>,
    pub(crate) cross_block_composition: Option<PageFlowPinTarget>,
    pub(crate) active_table_cell: Option<PageFlowPinTarget>,
    pub(crate) pending_table_focus: Option<PageFlowPinTarget>,
    pub(crate) text_selection_anchor: Option<PageFlowPinTarget>,
    pub(crate) text_selection_focus: Option<PageFlowPinTarget>,
    pub(crate) block_selection_first: Option<PageFlowPinTarget>,
    pub(crate) block_selection_last: Option<PageFlowPinTarget>,
    pub(crate) context_menu: Option<PageFlowPinTarget>,
    pub(crate) page_link_icon_picker: Option<PageFlowPinTarget>,
    pub(crate) slash_menu: Option<PageFlowPinTarget>,
}

impl PageFlowPinSet {
    fn for_each(&self, mut visit: impl FnMut(&PageFlowPinTarget)) {
        visit_optional(&self.active_input, &mut visit);
        visit_optional(&self.viewport_anchor, &mut visit);
        visit_optional(&self.pending_focus, &mut visit);
        visit_optional(&self.rich_text_composition, &mut visit);
        visit_optional(&self.cross_block_composition, &mut visit);
        visit_optional(&self.active_table_cell, &mut visit);
        visit_optional(&self.pending_table_focus, &mut visit);
        visit_optional(&self.text_selection_anchor, &mut visit);
        visit_optional(&self.text_selection_focus, &mut visit);
        visit_optional(&self.block_selection_first, &mut visit);
        visit_optional(&self.block_selection_last, &mut visit);
        visit_optional(&self.context_menu, &mut visit);
        visit_optional(&self.page_link_icon_picker, &mut visit);
        visit_optional(&self.slash_menu, &mut visit);
    }
}

fn visit_optional(target: &Option<PageFlowPinTarget>, visit: &mut impl FnMut(&PageFlowPinTarget)) {
    if let Some(target) = target {
        visit(target);
    }
}

#[derive(Default)]
pub(super) struct PageFlowPinnedNodes {
    node_ids: HashSet<PageFlowNodeId>,
}

impl PageFlowPinnedNodes {
    pub(super) fn new(
        projection: &PageFlowProjection,
        pins: &PageFlowPinSet,
        semantic_anchor: Option<&PageFlowPinTarget>,
    ) -> Self {
        let mut result = Self::default();
        pins.for_each(|target| result.insert_target(projection, target));
        if let Some(target) = semantic_anchor {
            result.insert_target(projection, target);
        }
        result
    }

    fn insert_target(&mut self, projection: &PageFlowProjection, target: &PageFlowPinTarget) {
        let Some(mut path) = pin_node_path(projection, target) else {
            return;
        };
        loop {
            self.node_ids.insert(path.key().node_id());
            let Some(parent) = path.parent() else {
                break;
            };
            path = parent;
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &PageFlowNodeId> {
        self.node_ids.iter()
    }
}

pub(super) fn pin_node_path<'a>(
    projection: &'a PageFlowProjection,
    target: &PageFlowPinTarget,
) -> Option<&'a PageFlowNodePath> {
    match target {
        PageFlowPinTarget::Unit(key) => {
            projection.location(key).map(|location| &location.node_path)
        }
        PageFlowPinTarget::Node(id) => projection.node_path(id),
        PageFlowPinTarget::Outer(PageDocumentOuterItemId::Flow(id)) => projection.node_path(id),
        PageFlowPinTarget::Outer(
            PageDocumentOuterItemId::Lead | PageDocumentOuterItemId::Footer,
        ) => None,
    }
}
