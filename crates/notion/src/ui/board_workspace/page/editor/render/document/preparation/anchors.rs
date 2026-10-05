use crate::ui::surface::{
    PageFlowCapturedSemanticAnchor, PageFlowOuterItemMatch, PageFlowOuterItemRemap, PageFlowPinSet,
    PageFlowPinTarget, PageFlowSemanticAnchorBaseline, PageFlowSemanticAnchorIntent,
    PageFlowViewportRelativeY,
};
use crate::ui::{LoadedCardPageData, PageDocumentListAllocation, PageDocumentOuterItemId};

/// The anchors captured before a recursive frame: the viewport's true top and
/// the table row a pointer focused.
pub(super) struct PageFlowAnchorCaptures {
    pub(super) true_top: Option<PageFlowCapturedSemanticAnchor>,
    pub(super) pointer_table: Option<PageFlowCapturedSemanticAnchor>,
}

pub(super) fn recursive_anchor_baseline(
    data: &LoadedCardPageData,
    pins: &PageFlowPinSet,
    captures: PageFlowAnchorCaptures,
    list_allocation: &PageDocumentListAllocation,
    remap: &PageFlowOuterItemRemap,
) -> Option<PageFlowSemanticAnchorBaseline> {
    let PageFlowAnchorCaptures {
        true_top: captured,
        pointer_table,
    } = captures;
    if let Some(target) = pins.pending_focus.as_ref() {
        return Some(focus_baseline(target.clone()));
    }
    if let Some(target) = pins.pending_table_focus.as_ref() {
        return Some(match pointer_table {
            Some(pointer) => {
                assert!(pointer.source_matches_list(list_allocation));
                assert_eq!(&pointer.baseline.target, target);
                pointer.baseline
            }
            None => focus_baseline(target.clone()),
        });
    }
    if let Some(captured) =
        captured.filter(|captured| page_flow_target_exists(data, &captured.baseline.target))
    {
        assert!(captured.source_matches_list(list_allocation));
        return Some(captured.baseline);
    }
    // The live capture is preferred, but when it is unavailable (for example a
    // scroll that crossed an outer item since the last observation) the remap
    // still pins the viewport. Stable frames are skipped at commit time, so a
    // fallback baseline here cannot fight an idle viewport.
    outer_remap_baseline(remap)
}

fn focus_baseline(target: PageFlowPinTarget) -> PageFlowSemanticAnchorBaseline {
    PageFlowSemanticAnchorBaseline {
        target,
        viewport_relative_y: PageFlowViewportRelativeY::new(0.0),
        intent: PageFlowSemanticAnchorIntent::RevealFocus,
    }
}

fn page_flow_target_exists(data: &LoadedCardPageData, target: &PageFlowPinTarget) -> bool {
    match target {
        PageFlowPinTarget::Unit(key) => data.flow.location(key).is_some(),
        PageFlowPinTarget::Node(id) => data.flow.node_path(id).is_some(),
        PageFlowPinTarget::Outer(PageDocumentOuterItemId::Flow(id)) => {
            data.flow.node_path(id).is_some()
        }
        PageFlowPinTarget::Outer(
            PageDocumentOuterItemId::Lead | PageDocumentOuterItemId::Footer,
        ) => true,
    }
}

fn outer_remap_baseline(remap: &PageFlowOuterItemRemap) -> Option<PageFlowSemanticAnchorBaseline> {
    let PageFlowOuterItemRemap::Item {
        key, match_kind, ..
    } = remap
    else {
        return None;
    };
    let relative_y = match match_kind {
        PageFlowOuterItemMatch::Exact { captured_offset } => -captured_offset.pixels(),
        PageFlowOuterItemMatch::NearestSurvivor => 0.0,
    };
    Some(PageFlowSemanticAnchorBaseline {
        target: PageFlowPinTarget::Outer(key.clone()),
        viewport_relative_y: PageFlowViewportRelativeY::new(relative_y),
        intent: PageFlowSemanticAnchorIntent::ViewportPreservation,
    })
}
