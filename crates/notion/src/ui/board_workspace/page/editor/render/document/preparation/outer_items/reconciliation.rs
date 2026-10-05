use std::{collections::HashMap, ops::Range};

use gpui::{px, ListOffset, ListState};

use crate::ui::{
    surface::{PageFlowOuterItemMatch, PageFlowOuterItemRemap, PageFlowOuterLocalY},
    PageDocumentOuterItemId,
};

use super::PageDocumentFocusPreparation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ui::board_workspace::page::editor::render::document::preparation) enum PageDocumentOuterReconcileMode
{
    ExistingLinear,
    RecursiveFlow,
}

enum PageDocumentScrollAnchor {
    Start,
    Tail,
    KeyOrNearestSurvivor {
        key: PageDocumentOuterItemId,
        previous_index: usize,
        offset: gpui::Pixels,
    },
    UnkeyedIndex(usize),
}

pub(super) fn reconcile_page_document_list(
    list_state: &ListState,
    focus: &PageDocumentFocusPreparation,
    mode: PageDocumentOuterReconcileMode,
) -> Option<PageFlowOuterItemRemap> {
    let previous_count = list_state.item_count();
    let unchanged = previous_count == focus.current_order.len() && !focus.order_changed;
    if unchanged && mode == PageDocumentOuterReconcileMode::ExistingLinear {
        return None;
    }
    let anchor = capture_scroll_anchor(
        previous_count,
        &focus.previous_order,
        list_state.logical_scroll_top(),
    );
    if !unchanged {
        splice_page_document_list(list_state, focus, previous_count);
    }
    match mode {
        PageDocumentOuterReconcileMode::ExistingLinear => {
            list_state
                .scroll_to(anchor.resolve_linear(&focus.previous_order, &focus.current_order));
            None
        }
        PageDocumentOuterReconcileMode::RecursiveFlow => Some(if unchanged {
            anchor.remap_unchanged(&focus.current_order)
        } else {
            anchor.remap_recursive(&focus.previous_order, &focus.current_order)
        }),
    }
}

fn splice_page_document_list(
    list_state: &ListState,
    focus: &PageDocumentFocusPreparation,
    previous_count: usize,
) {
    let (old_range, new_range) =
        page_document_splice_ranges(previous_count, &focus.previous_order, &focus.current_order);
    list_state.splice_focusable(
        old_range,
        focus.handles[new_range].iter().cloned().map(Some),
    );
}

fn capture_scroll_anchor(
    previous_count: usize,
    previous_order: &[PageDocumentOuterItemId],
    offset: ListOffset,
) -> PageDocumentScrollAnchor {
    if previous_count == 0 {
        return PageDocumentScrollAnchor::Start;
    }
    if previous_order.len() != previous_count {
        return PageDocumentScrollAnchor::UnkeyedIndex(offset.item_ix);
    }
    if offset.item_ix == previous_count {
        return PageDocumentScrollAnchor::Tail;
    }
    assert!(offset.item_ix < previous_count);
    PageDocumentScrollAnchor::KeyOrNearestSurvivor {
        key: previous_order[offset.item_ix].clone(),
        previous_index: offset.item_ix,
        offset: offset.offset_in_item,
    }
}

fn page_document_splice_ranges(
    previous_count: usize,
    previous: &[PageDocumentOuterItemId],
    current: &[PageDocumentOuterItemId],
) -> (Range<usize>, Range<usize>) {
    if previous.len() != previous_count {
        return (0..previous_count, 0..current.len());
    }
    let prefix = previous
        .iter()
        .zip(current)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = previous[prefix..]
        .iter()
        .rev()
        .zip(current[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    (
        prefix..previous.len() - suffix,
        prefix..current.len() - suffix,
    )
}

impl PageDocumentScrollAnchor {
    fn remap_unchanged(self, current: &[PageDocumentOuterItemId]) -> PageFlowOuterItemRemap {
        match self {
            Self::Start => PageFlowOuterItemRemap::Start,
            Self::Tail => PageFlowOuterItemRemap::Tail {
                item_count: current.len(),
            },
            Self::UnkeyedIndex(index) => remap_unkeyed(index, current),
            Self::KeyOrNearestSurvivor {
                key,
                previous_index,
                offset,
            } => {
                assert_eq!(current.get(previous_index), Some(&key));
                PageFlowOuterItemRemap::Item {
                    key,
                    new_index: previous_index,
                    match_kind: PageFlowOuterItemMatch::Exact {
                        captured_offset: PageFlowOuterLocalY::new(offset.as_f32()),
                    },
                }
            }
        }
    }

    fn resolve_linear(
        self,
        previous: &[PageDocumentOuterItemId],
        current: &[PageDocumentOuterItemId],
    ) -> ListOffset {
        match self {
            Self::Start => list_offset(0),
            Self::Tail => list_offset(current.len()),
            Self::UnkeyedIndex(index) => list_offset(index.min(current.len())),
            Self::KeyOrNearestSurvivor {
                key,
                previous_index,
                offset,
            } => resolve_linear_key(key, previous_index, offset, previous, current),
        }
    }

    fn remap_recursive(
        self,
        previous: &[PageDocumentOuterItemId],
        current: &[PageDocumentOuterItemId],
    ) -> PageFlowOuterItemRemap {
        match self {
            Self::Start => PageFlowOuterItemRemap::Start,
            Self::Tail => PageFlowOuterItemRemap::Tail {
                item_count: current.len(),
            },
            Self::UnkeyedIndex(index) => remap_unkeyed(index, current),
            Self::KeyOrNearestSurvivor {
                key,
                previous_index,
                offset,
            } => remap_key(key, previous_index, offset, previous, current),
        }
    }
}

fn resolve_linear_key(
    key: PageDocumentOuterItemId,
    previous_index: usize,
    offset: gpui::Pixels,
    previous: &[PageDocumentOuterItemId],
    current: &[PageDocumentOuterItemId],
) -> ListOffset {
    let positions = current_positions(current);
    if let Some(index) = positions.get(&key).copied() {
        return ListOffset {
            item_ix: index,
            offset_in_item: offset,
        };
    }
    list_offset(nearest_survivor(previous_index, previous, &positions))
}

fn remap_key(
    key: PageDocumentOuterItemId,
    previous_index: usize,
    offset: gpui::Pixels,
    previous: &[PageDocumentOuterItemId],
    current: &[PageDocumentOuterItemId],
) -> PageFlowOuterItemRemap {
    let positions = current_positions(current);
    if let Some(new_index) = positions.get(&key).copied() {
        return PageFlowOuterItemRemap::Item {
            key,
            new_index,
            match_kind: PageFlowOuterItemMatch::Exact {
                captured_offset: PageFlowOuterLocalY::new(offset.as_f32()),
            },
        };
    }
    let new_index = nearest_survivor(previous_index, previous, &positions);
    PageFlowOuterItemRemap::Item {
        key: current[new_index].clone(),
        new_index,
        match_kind: PageFlowOuterItemMatch::NearestSurvivor,
    }
}

fn remap_unkeyed(index: usize, current: &[PageDocumentOuterItemId]) -> PageFlowOuterItemRemap {
    if index >= current.len() {
        return PageFlowOuterItemRemap::Tail {
            item_count: current.len(),
        };
    }
    PageFlowOuterItemRemap::Item {
        key: current[index].clone(),
        new_index: index,
        match_kind: PageFlowOuterItemMatch::NearestSurvivor,
    }
}

fn current_positions(
    current: &[PageDocumentOuterItemId],
) -> HashMap<&PageDocumentOuterItemId, usize> {
    current
        .iter()
        .enumerate()
        .map(|(index, key)| (key, index))
        .collect()
}

fn nearest_survivor(
    previous_index: usize,
    previous: &[PageDocumentOuterItemId],
    positions: &HashMap<&PageDocumentOuterItemId, usize>,
) -> usize {
    previous[previous_index + 1..]
        .iter()
        .find_map(|key| positions.get(key).copied())
        .or_else(|| {
            previous[..previous_index]
                .iter()
                .rev()
                .find_map(|key| positions.get(key).copied())
        })
        .unwrap_or(0)
}

fn list_offset(item_ix: usize) -> ListOffset {
    ListOffset {
        item_ix,
        offset_in_item: px(0.0),
    }
}
