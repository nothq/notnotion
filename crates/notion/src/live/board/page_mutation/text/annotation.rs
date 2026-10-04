use super::super::wire::{
    AddAnnotationArgs, AnnotationAnchor, AnnotationBoundary, InsertTextArgs, RecordPointer,
    RemoveAnnotationArgs, SaveOperation,
};
use super::{text_offset_utf8_to_utf16, title_insertion_point, CrdtClock};
use crate::live::board::page_state::{CrdtItemId, CrdtOperationId, CrdtTitleState};
use crate::model::{
    EditPageBlockTextRequest, PageTextAnnotation, PageTextAnnotationKind, PageTextEditTarget,
};

mod contract;
mod index;

pub(in crate::live::board::page_mutation) use contract::annotation_tuple;
use contract::{addition_end_expands, annotation_key, removal_expands};
use index::{
    annotation_segments, boundary, item_after_origin, operation_id_at, selection_end,
    selection_start, SliceSegment,
};

struct InsertedSpan {
    text_instance_id: String,
    search_label: String,
    first_id: CrdtOperationId,
    last_id: CrdtOperationId,
    origin_id: CrdtItemId,
    next_id: CrdtItemId,
}

/// The annotation anchors that wrap exactly one visible CRDT item.
pub(super) struct MentionTokenBoundaries {
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) start: AnnotationBoundary,
    pub(super) end: AnnotationBoundary,
}

/// Anchors around the single visible item at `offset_utf16`, matching the
/// `before`/`after` pair Notion emits for a mention token's annotation.
pub(super) fn mention_token_boundaries(
    title: &CrdtTitleState,
    offset_utf16: usize,
) -> Result<MentionTokenBoundaries, String> {
    let segments = annotation_segments(&title.tree, offset_utf16, offset_utf16 + 1)?;
    let [segment] = segments.as_slice() else {
        return Err(format!(
            "mention token at UTF-16 offset {offset_utf16} does not map to one CRDT item"
        ));
    };
    Ok(MentionTokenBoundaries {
        text_instance_id: segment.slice.text_instance_id.clone(),
        search_label: segment.slice.search_label.clone(),
        start: selection_start(segment, true)?,
        end: selection_end(segment, false, false)?,
    })
}

pub(super) fn token_boundary_before(id: CrdtItemId) -> AnnotationBoundary {
    boundary(id, AnnotationAnchor::Before)
}

pub(super) fn token_boundary_after(id: CrdtItemId) -> AnnotationBoundary {
    boundary(id, AnnotationAnchor::After)
}

pub(in crate::live::board::page_mutation) fn plan_selection_text_edit(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    target: &PageTextEditTarget,
    request: &EditPageBlockTextRequest,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let PageTextEditTarget::Selection {
        start_utf8,
        end_utf8,
        ..
    } = target
    else {
        return Err("a selected page-text mutation requires a selection target".to_string());
    };
    let start_utf16 = text_offset_utf8_to_utf16(title, *start_utf8)?;
    let end_utf16 = text_offset_utf8_to_utf16(title, *end_utf8)?;
    let segments = annotation_segments(&title.tree, start_utf16, end_utf16)?;
    let mut operations = Vec::new();
    append_selection_removals(
        &mut operations,
        &pointer,
        &segments,
        request.annotation_removals(),
        clock,
    )?;
    append_selection_additions(
        &mut operations,
        &pointer,
        &segments,
        request.annotation_additions(),
        clock,
    )?;
    Ok(operations)
}

pub(in crate::live::board::page_mutation) fn plan_typing_text_edit(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    target: &PageTextEditTarget,
    request: &EditPageBlockTextRequest,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let PageTextEditTarget::Typing {
        offset_utf8, text, ..
    } = target
    else {
        return Err("a typing page-text mutation requires a typing target".to_string());
    };
    let offset_utf16 = text_offset_utf8_to_utf16(title, *offset_utf8)?;
    let insertion = title_insertion_point(title, offset_utf16)?;
    let next_id = item_after_origin(
        &title.tree,
        &insertion.text_instance_id,
        &insertion.origin_id,
    )?;
    let length = text.encode_utf16().count();
    let first_id = clock.reserve_after(&insertion.origin_id, length)?;
    let last_id = operation_id_at(&first_id, length - 1)?;
    let span = InsertedSpan {
        text_instance_id: insertion.text_instance_id.clone(),
        search_label: insertion.search_label.clone(),
        first_id: first_id.clone(),
        last_id,
        origin_id: insertion.origin_id.clone(),
        next_id,
    };
    let mut operations = vec![SaveOperation::insert_text(
        pointer.clone(),
        InsertTextArgs {
            operation_type: "insertText",
            text_instance_id: insertion.text_instance_id,
            search_label: insertion.search_label,
            id: first_id,
            origin_id: insertion.origin_id,
            content: text.to_string(),
            prev_items: insertion.prev_items,
        },
    )];
    append_typing_removals(
        &mut operations,
        &pointer,
        &span,
        request.annotation_removals(),
        clock,
    )?;
    append_typing_additions(
        &mut operations,
        &pointer,
        &span,
        request.annotation_additions(),
        clock,
    )?;
    Ok(operations)
}

fn append_selection_removals(
    operations: &mut Vec<SaveOperation>,
    pointer: &RecordPointer,
    segments: &[SliceSegment<'_>],
    removals: &[PageTextAnnotationKind],
    clock: &mut CrdtClock,
) -> Result<(), String> {
    for kind in removals {
        for segment in segments {
            let start = selection_start(segment, removal_expands(*kind))?;
            let end = selection_end(segment, true, true)?;
            operations.push(SaveOperation::remove_annotation(
                pointer.clone(),
                RemoveAnnotationArgs {
                    operation_type: "removeAnnotation",
                    text_instance_id: segment.slice.text_instance_id.clone(),
                    search_label: segment.slice.search_label.clone(),
                    id: clock.reserve(1)?,
                    start,
                    end,
                    annotation_key: annotation_key(*kind).to_string(),
                },
            ));
        }
    }
    Ok(())
}

fn append_selection_additions(
    operations: &mut Vec<SaveOperation>,
    pointer: &RecordPointer,
    segments: &[SliceSegment<'_>],
    additions: &[PageTextAnnotation],
    clock: &mut CrdtClock,
) -> Result<(), String> {
    for annotation in additions {
        let end_expands = addition_end_expands(annotation);
        for segment in segments {
            operations.push(SaveOperation::add_annotation(
                pointer.clone(),
                AddAnnotationArgs {
                    operation_type: "addAnnotation",
                    text_instance_id: segment.slice.text_instance_id.clone(),
                    search_label: segment.slice.search_label.clone(),
                    id: clock.reserve(1)?,
                    start: selection_start(segment, true)?,
                    end: selection_end(segment, end_expands, false)?,
                    annotation: annotation_tuple(annotation),
                },
            ));
        }
    }
    Ok(())
}

fn append_typing_removals(
    operations: &mut Vec<SaveOperation>,
    pointer: &RecordPointer,
    span: &InsertedSpan,
    removals: &[PageTextAnnotationKind],
    clock: &mut CrdtClock,
) -> Result<(), String> {
    for kind in removals {
        let start = if removal_expands(*kind) {
            boundary(
                CrdtItemId::Operation(span.first_id.clone()),
                AnnotationAnchor::Before,
            )
        } else {
            boundary(span.origin_id.clone(), AnnotationAnchor::After)
        };
        operations.push(SaveOperation::remove_annotation(
            pointer.clone(),
            RemoveAnnotationArgs {
                operation_type: "removeAnnotation",
                text_instance_id: span.text_instance_id.clone(),
                search_label: span.search_label.clone(),
                id: clock.reserve(1)?,
                start,
                end: boundary(span.next_id.clone(), AnnotationAnchor::Before),
                annotation_key: annotation_key(*kind).to_string(),
            },
        ));
    }
    Ok(())
}

fn append_typing_additions(
    operations: &mut Vec<SaveOperation>,
    pointer: &RecordPointer,
    span: &InsertedSpan,
    additions: &[PageTextAnnotation],
    clock: &mut CrdtClock,
) -> Result<(), String> {
    for annotation in additions {
        let end = if addition_end_expands(annotation) {
            boundary(span.next_id.clone(), AnnotationAnchor::Before)
        } else {
            boundary(
                CrdtItemId::Operation(span.last_id.clone()),
                AnnotationAnchor::After,
            )
        };
        operations.push(SaveOperation::add_annotation(
            pointer.clone(),
            AddAnnotationArgs {
                operation_type: "addAnnotation",
                text_instance_id: span.text_instance_id.clone(),
                search_label: span.search_label.clone(),
                id: clock.reserve(1)?,
                start: boundary(
                    CrdtItemId::Operation(span.first_id.clone()),
                    AnnotationAnchor::Before,
                ),
                end,
                annotation: annotation_tuple(annotation),
            },
        ));
    }
    Ok(())
}
