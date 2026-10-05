use super::super::super::page_state::{
    CrdtItemId, SerializedAnnotationAnchor, SerializedAnnotationBoundary,
    SerializedAnnotationOperation, SerializedTextItem, SerializedTextItemAnnotations,
    SerializedTextSlice,
};
use super::{
    InsertTextPrevAddAnnotation, InsertTextPrevAnnotationBoundary,
    InsertTextPrevAnnotationOperation, InsertTextPrevItem, InsertTextPrevRemoveAnnotation,
    InsertTextPrevSplitItem, InsertTextPrevStartItem, InsertTextPrevTextItem,
};

pub(in crate::live::board::page_mutation) fn insert_text_prev_items(
    slice: &SerializedTextSlice,
    origin_id: &CrdtItemId,
) -> Result<Vec<InsertTextPrevItem>, String> {
    if matches!(origin_id, CrdtItemId::Boundary(id) if id == "start") {
        return Ok(vec![insert_text_prev_start_item()]);
    }
    let Some(mut item) = serialized_item_containing_id(slice, origin_id)? else {
        return Err(format!(
            "title CRDT slice does not contain insertion origin {origin_id:?}"
        ));
    };
    let mut items = Vec::new();
    loop {
        items.push(insert_text_prev_item(item)?);
        if items.len() == 100 {
            break;
        }
        let item_origin_id = match item {
            SerializedTextItem::Split { origin_id, .. }
            | SerializedTextItem::Text { origin_id, .. } => origin_id,
            SerializedTextItem::Start { .. } | SerializedTextItem::End { .. } => unreachable!(),
        };
        if matches!(item_origin_id, CrdtItemId::Boundary(id) if id == "start") {
            items.push(insert_text_prev_start_item());
            break;
        }
        let origin_item =
            serialized_item_containing_id(slice, item_origin_id)?.ok_or_else(|| {
                format!("title CRDT slice does not contain previous item {item_origin_id:?}")
            })?;
        item = origin_item;
    }
    Ok(items)
}

fn insert_text_prev_start_item() -> InsertTextPrevItem {
    InsertTextPrevItem::Start(InsertTextPrevStartItem {
        operation_type: "start",
    })
}

pub(super) fn serialized_item_containing_id<'a>(
    slice: &'a SerializedTextSlice,
    item_id: &CrdtItemId,
) -> Result<Option<&'a SerializedTextItem>, String> {
    for item in &slice.items {
        let contains = match (item_id, item) {
            (CrdtItemId::Operation(candidate), SerializedTextItem::Split { id, .. }) => {
                candidate == id
            }
            (CrdtItemId::Operation(candidate), SerializedTextItem::Text { id, length, .. })
                if candidate.0 == id.0 =>
            {
                let end =
                    id.1.checked_add(length.unsigned_abs())
                        .ok_or_else(|| "title CRDT text operation range overflow".to_string())?;
                candidate.1 >= id.1 && candidate.1 < end
            }
            _ => false,
        };
        if contains {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

pub(super) fn insert_text_prev_item(
    item: &SerializedTextItem,
) -> Result<InsertTextPrevItem, String> {
    Ok(match item {
        SerializedTextItem::Split {
            id,
            origin_id,
            annotations,
        } => {
            let (annotation_ops_before, annotation_ops_after) =
                insert_text_prev_annotations(annotations);
            InsertTextPrevItem::Split(InsertTextPrevSplitItem {
                operation_type: "split",
                origin_id: origin_id.clone(),
                id: id.clone(),
                annotation_ops_before,
                annotation_ops_after,
            })
        }
        SerializedTextItem::Text {
            id,
            origin_id,
            length,
            content,
            annotations,
        } => {
            let deleted = *length < 0;
            let content = if deleted {
                None
            } else {
                Some(
                    content
                        .as_ref()
                        .ok_or_else(|| {
                            "visible title CRDT text item is missing content".to_string()
                        })?
                        .clone(),
                )
            };
            let (annotation_ops_before, annotation_ops_after) =
                insert_text_prev_annotations(annotations);
            InsertTextPrevItem::Text(InsertTextPrevTextItem {
                operation_type: "text",
                origin_id: origin_id.clone(),
                id: id.clone(),
                length: length.unsigned_abs(),
                annotation_ops_before,
                annotation_ops_after,
                deleted,
                content,
            })
        }
        SerializedTextItem::Start { .. } | SerializedTextItem::End { .. } => unreachable!(),
    })
}

type PrevAnnotationOperations = Option<Vec<InsertTextPrevAnnotationOperation>>;

pub(super) fn insert_text_prev_annotations(
    annotations: &SerializedTextItemAnnotations,
) -> (PrevAnnotationOperations, PrevAnnotationOperations) {
    (
        annotations
            .before()
            .map(|operations| operations.iter().map(insert_text_prev_annotation).collect()),
        annotations
            .after()
            .map(|operations| operations.iter().map(insert_text_prev_annotation).collect()),
    )
}

pub(super) fn insert_text_prev_annotation(
    operation: &SerializedAnnotationOperation,
) -> InsertTextPrevAnnotationOperation {
    match operation {
        SerializedAnnotationOperation::Add {
            text_instance_id,
            search_label,
            id,
            start,
            end,
            annotation,
        } => InsertTextPrevAnnotationOperation::Add(InsertTextPrevAddAnnotation {
            operation_type: "addAnnotation",
            text_instance_id: text_instance_id.clone(),
            search_label: search_label.clone(),
            id: id.clone(),
            start: insert_text_prev_annotation_boundary(start),
            end: insert_text_prev_annotation_boundary(end),
            annotation: annotation.clone(),
        }),
        SerializedAnnotationOperation::Remove {
            text_instance_id,
            search_label,
            id,
            start,
            end,
            annotation_key,
        } => InsertTextPrevAnnotationOperation::Remove(InsertTextPrevRemoveAnnotation {
            operation_type: "removeAnnotation",
            text_instance_id: text_instance_id.clone(),
            search_label: search_label.clone(),
            id: id.clone(),
            start: insert_text_prev_annotation_boundary(start),
            end: insert_text_prev_annotation_boundary(end),
            annotation_key: annotation_key.clone(),
        }),
    }
}

pub(super) fn insert_text_prev_annotation_boundary(
    boundary: &SerializedAnnotationBoundary,
) -> InsertTextPrevAnnotationBoundary {
    InsertTextPrevAnnotationBoundary {
        id: boundary.id.clone(),
        anchor: match boundary.anchor {
            SerializedAnnotationAnchor::Before => "before",
            SerializedAnnotationAnchor::After => "after",
        },
    }
}
