use std::ops::Range;

use crate::model::{CardPageTextAnnotationSpan, PageTextAnnotation, PageTextAnnotationKind};

pub(crate) fn annotations_at_caret_in_spans(
    spans: &[CardPageTextAnnotationSpan],
    offset: usize,
) -> Vec<PageTextAnnotation> {
    let mut annotations = Vec::new();
    for span in spans
        .iter()
        .filter(|span| annotation_touches_caret(span, offset))
    {
        if annotations
            .iter()
            .any(|candidate: &PageTextAnnotation| candidate.kind() == span.annotation.kind())
        {
            continue;
        }
        annotations.push(span.annotation.clone());
    }
    annotations
}

pub(crate) fn range_has_annotation_in_spans(
    source: &[CardPageTextAnnotationSpan],
    range: Range<usize>,
    kind: PageTextAnnotationKind,
) -> bool {
    let mut covered_to = range.start;
    let mut spans = source
        .iter()
        .filter(|span| span.annotation.kind() == kind && span.end_utf8 > range.start)
        .collect::<Vec<_>>();
    spans.sort_by_key(|span| span.start_utf8);
    for span in spans {
        if span.start_utf8 > covered_to {
            return false;
        }
        covered_to = covered_to.max(span.end_utf8);
        if covered_to >= range.end {
            return true;
        }
    }
    false
}

pub(crate) fn apply_typing_annotations_to_spans(
    spans: &mut Vec<CardPageTextAnnotationSpan>,
    offset: usize,
    inserted_len: usize,
    removals: &[PageTextAnnotationKind],
    additions: &[PageTextAnnotation],
) {
    let mut inherited = annotations_at_caret_in_spans(spans, offset);
    shift_annotations_for_insertion(spans, offset, inserted_len);
    inherited.retain(|annotation| !removals.contains(&annotation.kind()));
    for addition in additions {
        inherited.retain(|annotation| annotation.kind() != addition.kind());
        inherited.push(addition.clone());
    }
    for annotation in inherited {
        replace_annotation_kind(
            spans,
            offset..offset + inserted_len,
            annotation.kind(),
            Some(annotation),
        );
    }
}

pub(crate) fn apply_annotation_actions_to_spans(
    spans: &mut Vec<CardPageTextAnnotationSpan>,
    range: Range<usize>,
    removals: &[PageTextAnnotationKind],
    additions: &[PageTextAnnotation],
) {
    for kind in removals {
        replace_annotation_kind(spans, range.clone(), *kind, None);
    }
    for annotation in additions {
        replace_annotation_kind(
            spans,
            range.clone(),
            annotation.kind(),
            Some(annotation.clone()),
        );
    }
}

pub(crate) fn apply_plain_text_annotation_replacement_to_spans(
    spans: &mut Vec<CardPageTextAnnotationSpan>,
    previous_text: &str,
    next_text: &str,
) {
    let (start, previous_end, inserted_len) = text_replacement(previous_text, next_text);
    apply_text_annotation_replacement_to_spans(spans, start..previous_end, inserted_len);
}

pub(crate) fn apply_text_annotation_replacement_to_spans(
    spans: &mut Vec<CardPageTextAnnotationSpan>,
    range: Range<usize>,
    inserted_len: usize,
) {
    if range.is_empty() {
        apply_typing_annotations_to_spans(spans, range.start, inserted_len, &[], &[]);
        return;
    }
    let delta = inserted_len as isize - range.len() as isize;
    *spans = spans
        .drain(..)
        .filter_map(|span| adjusted_replacement_span(span, &range, inserted_len, delta))
        .collect();
    sort_annotations(spans);
}

fn annotation_touches_caret(span: &CardPageTextAnnotationSpan, offset: usize) -> bool {
    // Mention tokens are atoms: text typed next to one never inherits it.
    if span.annotation.kind().is_mention() {
        return false;
    }
    span.start_utf8 == offset
        || (span.start_utf8 < offset
            && (span.end_utf8 > offset
                || span.end_utf8 == offset
                    && span.annotation.kind() != PageTextAnnotationKind::Link))
}

fn adjusted_replacement_span(
    mut span: CardPageTextAnnotationSpan,
    range: &Range<usize>,
    inserted_len: usize,
    delta: isize,
) -> Option<CardPageTextAnnotationSpan> {
    if span.end_utf8 <= range.start {
        return Some(span);
    }
    if span.start_utf8 >= range.end {
        span.start_utf8 = shift(span.start_utf8, delta);
        span.end_utf8 = shift(span.end_utf8, delta);
        return Some(span);
    }
    span.start_utf8 = span.start_utf8.min(range.start);
    span.end_utf8 = if span.end_utf8 > range.end {
        shift(span.end_utf8, delta)
    } else {
        range.start + inserted_len
    };
    (span.start_utf8 < span.end_utf8).then_some(span)
}

fn replace_annotation_kind(
    spans: &mut Vec<CardPageTextAnnotationSpan>,
    range: Range<usize>,
    kind: PageTextAnnotationKind,
    addition: Option<PageTextAnnotation>,
) {
    let mut next = Vec::with_capacity(spans.len() + usize::from(addition.is_some()));
    for span in spans.drain(..) {
        if span.annotation.kind() != kind
            || span.end_utf8 <= range.start
            || span.start_utf8 >= range.end
        {
            next.push(span);
            continue;
        }
        if span.start_utf8 < range.start {
            next.push(CardPageTextAnnotationSpan {
                end_utf8: range.start,
                ..span.clone()
            });
        }
        if span.end_utf8 > range.end {
            next.push(CardPageTextAnnotationSpan {
                start_utf8: range.end,
                ..span
            });
        }
    }
    if let Some(annotation) = addition {
        next.push(CardPageTextAnnotationSpan {
            start_utf8: range.start,
            end_utf8: range.end,
            annotation,
        });
    }
    merge_adjacent_annotations(&mut next);
    *spans = next;
}

fn shift_annotations_for_insertion(
    spans: &mut [CardPageTextAnnotationSpan],
    offset: usize,
    inserted_len: usize,
) {
    for span in spans {
        if span.start_utf8 >= offset {
            span.start_utf8 += inserted_len;
            span.end_utf8 += inserted_len;
        } else if span.end_utf8 > offset {
            span.end_utf8 += inserted_len;
        }
    }
}

fn text_replacement(previous: &str, next: &str) -> (usize, usize, usize) {
    let mut start = 0;
    for ((left_offset, left), (right_offset, right)) in
        previous.char_indices().zip(next.char_indices())
    {
        if left != right {
            break;
        }
        start = left_offset + left.len_utf8();
        debug_assert_eq!(start, right_offset + right.len_utf8());
    }
    let mut previous_end = previous.len();
    let mut next_end = next.len();
    while previous_end > start && next_end > start {
        let left = previous[..previous_end].chars().next_back().unwrap();
        let right = next[..next_end].chars().next_back().unwrap();
        if left != right {
            break;
        }
        previous_end -= left.len_utf8();
        next_end -= right.len_utf8();
    }
    (start, previous_end, next_end - start)
}

fn shift(offset: usize, delta: isize) -> usize {
    offset
        .checked_add_signed(delta)
        .expect("valid text edit offset")
}

pub(super) fn merge_adjacent_annotations(annotations: &mut Vec<CardPageTextAnnotationSpan>) {
    sort_annotations(annotations);
    let mut merged: Vec<CardPageTextAnnotationSpan> = Vec::with_capacity(annotations.len());
    for span in annotations.drain(..) {
        if let Some(previous) = merged.last_mut().filter(|previous| {
            previous.annotation == span.annotation
                && previous.end_utf8 == span.start_utf8
                && span.annotation.merges_with_adjacent()
        }) {
            previous.end_utf8 = span.end_utf8;
        } else {
            merged.push(span);
        }
    }
    *annotations = merged;
}

fn sort_annotations(annotations: &mut [CardPageTextAnnotationSpan]) {
    annotations.sort_by_key(|span| (span.start_utf8, span.end_utf8));
}
