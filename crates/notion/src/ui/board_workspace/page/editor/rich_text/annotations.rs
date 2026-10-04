use std::ops::Range;

use crate::model::{PageTextAnnotation, PageTextAnnotationKind, PageTextEditTarget};
use crate::ui::{CardPage, CardPageEditableBlock};

mod mention;
mod spans;
mod transform;

pub(crate) use mention::{
    apply_mention_insert_to_editable, apply_mention_update_to_editable, mention_at_offset,
    mention_spans,
};
pub(in crate::ui::board_workspace::page::editor) use spans::{
    annotations_at_caret_in_spans, apply_annotation_actions_to_spans,
    apply_plain_text_annotation_replacement_to_spans, apply_text_annotation_replacement_to_spans,
    apply_typing_annotations_to_spans, range_has_annotation_in_spans,
};
pub(in crate::ui::board_workspace::page::editor) use transform::{
    annotated_text_slice, apply_annotated_text, concatenate_annotated_text,
    replace_annotated_text_range, AnnotatedText,
};

pub(super) fn selection_has_annotation(
    page: &CardPage,
    targets: &[PageTextEditTarget],
    kind: PageTextAnnotationKind,
) -> bool {
    targets.iter().all(|target| {
        let PageTextEditTarget::Selection {
            block_id,
            start_utf8,
            end_utf8,
        } = target
        else {
            return false;
        };
        let editable = page
            .blocks
            .iter()
            .find(|block| block.block_id == *block_id)
            .and_then(|block| block.editable_content());
        editable.is_some_and(|editable| {
            range_has_annotation_in_spans(&editable.annotations, *start_utf8..*end_utf8, kind)
        })
    })
}

pub(super) fn selection_annotation_kinds(
    page: &CardPage,
    targets: &[PageTextEditTarget],
) -> Vec<PageTextAnnotationKind> {
    let mut kinds = Vec::new();
    for target in targets {
        let PageTextEditTarget::Selection {
            block_id,
            start_utf8,
            end_utf8,
        } = target
        else {
            continue;
        };
        let editable = page
            .blocks
            .iter()
            .find(|block| block.block_id == *block_id)
            .and_then(|block| block.editable_content())
            .expect("page-text target must reference an editable block");
        for kind in editable
            .annotations
            .iter()
            .filter(|span| span.start_utf8 < *end_utf8 && span.end_utf8 > *start_utf8)
            .map(|span| span.annotation.kind())
            .filter(|kind| !kind.is_mention())
        {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    kinds
}

pub(super) fn apply_selection_annotation(
    page: &mut CardPage,
    targets: &[PageTextEditTarget],
    removal: Option<PageTextAnnotationKind>,
    addition: Option<PageTextAnnotation>,
) {
    for target in targets {
        let PageTextEditTarget::Selection {
            block_id,
            start_utf8,
            end_utf8,
        } = target
        else {
            continue;
        };
        let editable = page
            .blocks
            .iter_mut()
            .find(|block| block.block_id == *block_id)
            .and_then(|block| block.editable_content_mut())
            .expect("page-text target must reference an editable block");
        let range = *start_utf8..*end_utf8;
        if let Some(kind) = removal {
            apply_annotation_actions_to_spans(
                &mut editable.annotations,
                range.clone(),
                &[kind],
                &[],
            );
        }
        if let Some(annotation) = addition.clone() {
            apply_annotation_actions_to_spans(&mut editable.annotations, range, &[], &[annotation]);
        }
    }
}

pub(super) fn annotations_at_caret(
    editable: &CardPageEditableBlock,
    offset: usize,
) -> Vec<PageTextAnnotation> {
    annotations_at_caret_in_spans(&editable.annotations, offset)
}

pub(crate) fn apply_typing_annotations(
    editable: &mut CardPageEditableBlock,
    offset: usize,
    inserted_len: usize,
    removals: &[PageTextAnnotationKind],
    additions: &[PageTextAnnotation],
) {
    apply_typing_annotations_to_spans(
        &mut editable.annotations,
        offset,
        inserted_len,
        removals,
        additions,
    );
}

pub(crate) fn apply_annotation_actions_to_range(
    editable: &mut CardPageEditableBlock,
    range: Range<usize>,
    removals: &[PageTextAnnotationKind],
    additions: &[PageTextAnnotation],
) {
    apply_annotation_actions_to_spans(&mut editable.annotations, range, removals, additions);
}

pub(crate) fn apply_plain_text_annotation_replacement(
    editable: &mut CardPageEditableBlock,
    previous_text: &str,
    next_text: &str,
) {
    apply_plain_text_annotation_replacement_to_spans(
        &mut editable.annotations,
        previous_text,
        next_text,
    );
}
