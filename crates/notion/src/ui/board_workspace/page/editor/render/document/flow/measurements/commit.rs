use gpui::{px, App, ListOffset, ListState, Window};

use super::PageFlowRootCommit;
use crate::ui::board_workspace::page::editor::render::PageBlockRenderer;
use crate::ui::surface::PageFlowCommittedFocus;
use crate::ui::surface::{
    PageFlowLayoutCommit, PageFlowObservationToken, PageFlowOuterExtentAuthority,
    PageFlowSemanticAnchorIntent, PageFlowSemanticAnchorTransaction,
};
use crate::ui::LoadedCardPageData;

mod outer_extents;

use outer_extents::{logical_viewport_basis, outer_extent_authority};

enum PageFlowAnchorExtentState {
    ScrollbarDragging,
    Ready(PageFlowOuterExtentAuthority),
    Pending,
}

struct PageFlowSemanticAnchorCommit<'a> {
    layout: &'a PageFlowLayoutCommit,
    observation: &'a PageFlowObservationToken,
    data: &'a LoadedCardPageData,
    list_state: &'a ListState,
    extents: PageFlowAnchorExtentState,
}

pub(super) fn commit_page_flow_layout_frame(
    renderer: &PageBlockRenderer,
    root_commit: &PageFlowRootCommit,
    window: &mut Window,
    cx: &mut App,
) {
    let PageFlowRootCommit {
        frame,
        observation,
        data,
        list_state,
    } = root_commit;
    let data: &LoadedCardPageData = data;
    // The basis was stamped during preparation, before the list laid out and
    // possibly adjusted its own offset. This runs deferred, after that layout,
    // so it is both safe to read the list here and correct to re-stamp it
    // against the offset this generation's bounds were measured at.
    renderer
        .flow
        .state()
        .observations
        .borrow_mut()
        .observe_scroll_basis(observation, list_state.logical_scroll_top());
    let Some(commit) = renderer
        .flow
        .state()
        .virtualizer
        .borrow_mut()
        .commit_layout_frame(frame)
    else {
        return;
    };
    let scrollbar_dragging = list_state.is_scrollbar_dragging();
    apply_dirty_outer_ranges(&commit, list_state);
    let extents =
        if scrollbar_dragging {
            PageFlowAnchorExtentState::ScrollbarDragging
        } else if let Some(extents) = commit.semantic_anchor.as_ref().and_then(|anchor| {
            outer_extent_authority(renderer, observation, data, list_state, anchor)
        }) {
            PageFlowAnchorExtentState::Ready(extents)
        } else {
            PageFlowAnchorExtentState::Pending
        };
    let anchor_applied = apply_semantic_anchor(
        renderer,
        PageFlowSemanticAnchorCommit {
            layout: &commit,
            observation,
            data,
            list_state,
            extents,
        },
        window,
        cx,
    );
    if !commit.dirty_outer_ranges.is_empty() || commit.measurements_pending || !anchor_applied {
        renderer.notifier.notify(cx);
    }
}

fn apply_dirty_outer_ranges(commit: &PageFlowLayoutCommit, list_state: &ListState) {
    for range in commit.dirty_outer_ranges.iter().cloned() {
        list_state.remeasure_items(range);
    }
}

fn apply_semantic_anchor(
    renderer: &PageBlockRenderer,
    commit: PageFlowSemanticAnchorCommit<'_>,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(mut anchor) = commit.layout.semantic_anchor.clone() else {
        return true;
    };
    // Deferral comes first: a scrollbar drag must stash its anchor even when the
    // layout is otherwise stable, or there is nothing to resume when it ends.
    let extents = match &commit.extents {
        PageFlowAnchorExtentState::ScrollbarDragging => {
            defer_scrollbar_anchor(
                renderer,
                commit.layout,
                commit.data,
                commit.list_state,
                &anchor,
            );
            return true;
        }
        PageFlowAnchorExtentState::Ready(extents) => extents,
        PageFlowAnchorExtentState::Pending => return false,
    };
    if can_skip_stable_viewport_anchor(renderer, &commit, &anchor) {
        return true;
    }
    anchor = resume_deferred_anchor(renderer, &commit, extents, anchor);
    let Some(resolved) = anchor.resolved_outer_offset(extents) else {
        return false;
    };
    let previous = commit.list_state.logical_scroll_top();
    commit.list_state.scroll_to(ListOffset {
        item_ix: resolved.outer_index,
        offset_in_item: px(resolved.offset_in_item.pixels()),
    });
    let current = commit.list_state.logical_scroll_top();
    if previous.item_ix != current.item_ix || previous.offset_in_item != current.offset_in_item {
        renderer.notifier.notify(cx);
    }
    complete_mounted_focus(renderer, &commit, &anchor, window, cx);
    true
}

fn can_skip_stable_viewport_anchor(
    renderer: &PageBlockRenderer,
    commit: &PageFlowSemanticAnchorCommit<'_>,
    anchor: &PageFlowSemanticAnchorTransaction,
) -> bool {
    anchor.baseline.intent == PageFlowSemanticAnchorIntent::ViewportPreservation
        && !anchor.outer_order_changed
        && commit.layout.dirty_outer_ranges.is_empty()
        && !renderer
            .flow
            .state()
            .virtualizer
            .borrow()
            .has_deferred_scrollbar_anchor(&commit.layout.frame)
}

fn defer_scrollbar_anchor(
    renderer: &PageBlockRenderer,
    commit: &PageFlowLayoutCommit,
    data: &LoadedCardPageData,
    list_state: &ListState,
    anchor: &PageFlowSemanticAnchorTransaction,
) {
    let viewport = logical_viewport_basis(data, list_state);
    renderer
        .flow
        .state()
        .virtualizer
        .borrow_mut()
        .defer_scrollbar_anchor(&commit.frame, anchor.clone(), viewport);
}

fn resume_deferred_anchor(
    renderer: &PageBlockRenderer,
    commit: &PageFlowSemanticAnchorCommit<'_>,
    extents: &PageFlowOuterExtentAuthority,
    current: PageFlowSemanticAnchorTransaction,
) -> PageFlowSemanticAnchorTransaction {
    let viewport = logical_viewport_basis(commit.data, commit.list_state);
    renderer
        .flow
        .state()
        .virtualizer
        .borrow_mut()
        .resume_deferred_scrollbar_anchor(&commit.layout.frame, current, viewport, extents)
}

fn complete_mounted_focus(
    renderer: &PageBlockRenderer,
    commit: &PageFlowSemanticAnchorCommit<'_>,
    anchor: &PageFlowSemanticAnchorTransaction,
    window: &mut Window,
    cx: &mut App,
) {
    let observation = commit.observation;
    let data = commit.data;
    let target = &anchor.baseline.target;
    if !matches!(
        anchor.baseline.intent,
        PageFlowSemanticAnchorIntent::RevealFocus | PageFlowSemanticAnchorIntent::PointerTableFocus
    ) {
        return;
    }
    let mounted = renderer
        .flow
        .state()
        .observations
        .borrow()
        .mounted_projection(observation, target)
        .is_some();
    if mounted {
        if renderer.complete_recursive_page_block_focus(data, target, window, cx) {
            renderer.flow.set_committed_focus(None);
            return;
        }
        let table_target = renderer
            .tables
            .editor()
            .borrow()
            .as_ref()
            .filter(|editor| editor.page_id == data.page.block_id && editor.pending_focus.is_some())
            .and_then(|editor| data.document_unit_key_for_table_cell(&editor.address))
            .map(crate::ui::surface::PageFlowPinTarget::Unit);
        if table_target.as_ref() == Some(target) {
            renderer
                .flow
                .set_committed_focus(observation.mount_authority().map(|authority| {
                    PageFlowCommittedFocus {
                        authority,
                        target: target.clone(),
                    }
                }));
            renderer.notifier.notify(cx);
        }
    }
}
