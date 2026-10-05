use std::collections::HashMap;

use gpui::{Context, FocusHandle, ListState};

use super::super::super::super::{Arc, LoadedCardPageData, SurfaceState};
use crate::ui::surface::{
    PageDocumentFocusRegistryEntry, PageEditorState, PageFlowOuterItemRemap, PageFlowSurfaceKey,
};
use crate::ui::PageDocumentOuterItemId;

mod reconciliation;

use reconciliation::reconcile_page_document_list;
pub(super) use reconciliation::PageDocumentOuterReconcileMode;

struct PageDocumentFocusPreparation {
    handles: Arc<[FocusHandle]>,
    previous_order: Arc<[PageDocumentOuterItemId]>,
    current_order: Arc<[PageDocumentOuterItemId]>,
    order_changed: bool,
}

/// The outer list to reconcile, and how.
pub(super) struct PageDocumentListReconciliation<'a> {
    pub(super) list_state: &'a ListState,
    pub(super) mode: PageDocumentOuterReconcileMode,
}

pub(super) struct PageDocumentOuterPreparation {
    pub(super) handles: Arc<[FocusHandle]>,
    pub(super) remap: Option<PageFlowOuterItemRemap>,
    pub(super) order_changed: bool,
}

impl PageDocumentOuterPreparation {
    pub(super) fn into_existing_linear_handles(self) -> Arc<[FocusHandle]> {
        assert!(self.remap.is_none());
        self.handles
    }
}

impl PageEditorState {
    pub(super) fn prepare_page_document_outer_items(
        &self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        reconciliation: PageDocumentListReconciliation<'_>,
        cx: &mut Context<SurfaceState>,
    ) -> PageDocumentOuterPreparation {
        let PageDocumentListReconciliation {
            list_state,
            mode: reconcile_mode,
        } = reconciliation;
        let focus = self.page_document_focus_preparation(data, surface, cx);
        let remap = reconcile_page_document_list(list_state, &focus, reconcile_mode);
        PageDocumentOuterPreparation {
            handles: focus.handles,
            remap,
            order_changed: focus.order_changed,
        }
    }

    fn page_document_focus_preparation(
        &self,
        data: &Arc<LoadedCardPageData>,
        surface: PageFlowSurfaceKey,
        cx: &mut Context<SurfaceState>,
    ) -> PageDocumentFocusPreparation {
        let mut registry = self.flow.state().focus_handles.borrow_mut();
        let registry_key = (data.page.block_id.clone(), surface);
        if let Some(entry) = registry
            .get(&registry_key)
            .filter(|entry| entry.matches_projection(data))
        {
            return PageDocumentFocusPreparation {
                handles: entry.ordered_handles.clone(),
                previous_order: entry.order.clone(),
                current_order: entry.order.clone(),
                order_changed: false,
            };
        }
        let (focus, entry) =
            rebuild_page_document_focus_preparation(registry.remove(&registry_key), data, cx);
        registry.insert(registry_key, entry);
        focus
    }
}

fn rebuild_page_document_focus_preparation(
    previous: Option<PageDocumentFocusRegistryEntry>,
    data: &Arc<LoadedCardPageData>,
    cx: &mut Context<SurfaceState>,
) -> (PageDocumentFocusPreparation, PageDocumentFocusRegistryEntry) {
    let previous_order = previous.as_ref().map_or_else(
        || Arc::<[PageDocumentOuterItemId]>::from([]),
        |entry| entry.order.clone(),
    );
    let mut previous_handles = previous
        .map(|entry| entry.handles_by_item)
        .unwrap_or_default();
    let current_order: Arc<[PageDocumentOuterItemId]> = page_document_item_keys(data).into();
    let order_changed = previous_order.as_ref() != current_order.as_ref();
    let (handles, handles_by_item) =
        page_document_focus_handles(&current_order, &mut previous_handles, cx);
    let entry = PageDocumentFocusRegistryEntry {
        allocation: Arc::downgrade(data),
        projection_generation: data.flow_projection_generation,
        order: current_order.clone(),
        ordered_handles: handles.clone(),
        handles_by_item,
    };
    (
        PageDocumentFocusPreparation {
            handles,
            previous_order,
            current_order,
            order_changed,
        },
        entry,
    )
}

/// Focus handles in outer item order, and the same handles keyed by item.
type OuterFocusHandles = (
    Arc<[FocusHandle]>,
    HashMap<PageDocumentOuterItemId, FocusHandle>,
);

fn page_document_focus_handles(
    order: &[PageDocumentOuterItemId],
    previous: &mut HashMap<PageDocumentOuterItemId, FocusHandle>,
    cx: &mut Context<SurfaceState>,
) -> OuterFocusHandles {
    let mut current = HashMap::with_capacity(order.len());
    let handles = order
        .iter()
        .map(|key| {
            let handle = previous.remove(key).unwrap_or_else(|| cx.focus_handle());
            assert!(
                current.insert(key.clone(), handle.clone()).is_none(),
                "Notion outer list item identities must be unique"
            );
            handle
        })
        .collect::<Vec<_>>()
        .into();
    (handles, current)
}

fn page_document_item_keys(data: &LoadedCardPageData) -> Vec<PageDocumentOuterItemId> {
    let mut keys = Vec::with_capacity(data.flow.root.nodes.len() + 2);
    keys.push(PageDocumentOuterItemId::Lead);
    keys.extend(
        data.flow
            .root
            .nodes
            .iter()
            .map(|node| PageDocumentOuterItemId::Flow(node.node_id())),
    );
    keys.push(PageDocumentOuterItemId::Footer);
    keys
}
