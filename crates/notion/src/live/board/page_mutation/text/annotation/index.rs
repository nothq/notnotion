use super::super::super::wire::{AnnotationAnchor, AnnotationBoundary};
use crate::live::board::page_state::{
    CrdtItemId, CrdtOperationId, SerializedTextItem, SerializedTextSlice, SerializedTextSliceTree,
};

pub(super) struct SliceSegment<'a> {
    pub(super) slice: &'a SerializedTextSlice,
    pub(super) local_start: usize,
    pub(super) local_end: usize,
    continues_from_previous: bool,
    continues_into_next: bool,
}

pub(super) fn annotation_segments(
    tree: &SerializedTextSliceTree,
    start: usize,
    end: usize,
) -> Result<Vec<SliceSegment<'_>>, String> {
    if start >= end {
        return Err("a title annotation range must be non-empty".to_string());
    }
    let mut segments = Vec::new();
    let mut slice_start = 0;
    for key in tree.ordered_keys() {
        let slice = &tree.node(key).text_slice;
        let slice_end = slice_start + visible_slice_len(slice);
        let intersection_start = start.max(slice_start);
        let intersection_end = end.min(slice_end);
        if intersection_start < intersection_end {
            segments.push(SliceSegment {
                slice,
                local_start: intersection_start - slice_start,
                local_end: intersection_end - slice_start - 1,
                continues_from_previous: start < slice_start,
                continues_into_next: slice_end < end,
            });
        }
        slice_start = slice_end;
    }
    if end > slice_start {
        return Err(format!(
            "annotation range {start}..{end} exceeds title UTF-16 length {slice_start}"
        ));
    }
    if segments.is_empty() {
        return Err("title annotation range contains no visible text".to_string());
    }
    Ok(segments)
}

pub(super) fn selection_start(
    segment: &SliceSegment<'_>,
    expands: bool,
) -> Result<AnnotationBoundary, String> {
    if segment.continues_from_previous {
        return Ok(boundary(
            segment.slice.start_item_id()?,
            AnnotationAnchor::After,
        ));
    }
    if expands {
        Ok(boundary(
            visible_item_at(segment.slice, segment.local_start)?,
            AnnotationAnchor::Before,
        ))
    } else {
        Ok(boundary(
            visible_item_before(segment.slice, segment.local_start)?,
            AnnotationAnchor::After,
        ))
    }
}

pub(super) fn selection_end(
    segment: &SliceSegment<'_>,
    expands: bool,
    is_removal: bool,
) -> Result<AnnotationBoundary, String> {
    if segment.continues_into_next {
        return Ok(boundary(
            segment.slice.end_item_id()?,
            AnnotationAnchor::Before,
        ));
    }
    if expands || is_removal {
        Ok(boundary(
            visible_item_after(segment.slice, segment.local_end)?,
            AnnotationAnchor::Before,
        ))
    } else {
        Ok(boundary(
            visible_item_at(segment.slice, segment.local_end)?,
            AnnotationAnchor::After,
        ))
    }
}

pub(super) fn item_after_origin(
    tree: &SerializedTextSliceTree,
    text_instance_id: &str,
    origin_id: &CrdtItemId,
) -> Result<CrdtItemId, String> {
    let mut contains_text_instance = false;
    for key in tree.ordered_keys() {
        let slice = &tree.node(key).text_slice;
        if slice.text_instance_id != text_instance_id {
            continue;
        }
        contains_text_instance = true;
        for (index, item) in slice.items.iter().enumerate() {
            match (origin_id, item) {
                (CrdtItemId::Boundary(boundary), SerializedTextItem::Start { .. })
                    if boundary == "start" =>
                {
                    return following_item_id(slice, index);
                }
                (CrdtItemId::Operation(origin), SerializedTextItem::Split { id, .. })
                    if origin == id =>
                {
                    return following_item_id(slice, index);
                }
                (CrdtItemId::Operation(origin), SerializedTextItem::Text { id, length, .. })
                    if operation_is_within(origin, id, *length)? =>
                {
                    let offset = origin.1 - id.1;
                    if offset + 1 < length.unsigned_abs() {
                        return Ok(CrdtItemId::Operation(operation_id_at(
                            id,
                            offset as usize + 1,
                        )?));
                    }
                    return following_item_id(slice, index);
                }
                _ => {}
            }
        }
    }
    if !contains_text_instance {
        return Err(format!(
            "title CRDT does not contain text instance {text_instance_id}"
        ));
    }
    Err(format!(
        "title CRDT text instance {text_instance_id} does not contain insertion origin {origin_id:?}"
    ))
}

pub(super) fn operation_id_at(
    start: &CrdtOperationId,
    offset: usize,
) -> Result<CrdtOperationId, String> {
    let offset = u64::try_from(offset).map_err(|_| "CRDT operation offset is too large")?;
    Ok(CrdtOperationId(
        start.0.clone(),
        start
            .1
            .checked_add(offset)
            .ok_or_else(|| "CRDT operation range overflow".to_string())?,
    ))
}

fn visible_slice_len(slice: &SerializedTextSlice) -> usize {
    slice
        .items
        .iter()
        .filter_map(|item| match item {
            SerializedTextItem::Text { length, .. } if *length > 0 => Some(*length as usize),
            _ => None,
        })
        .sum()
}

fn visible_item_at(slice: &SerializedTextSlice, index: usize) -> Result<CrdtItemId, String> {
    let mut cursor = 0;
    for item in &slice.items {
        let SerializedTextItem::Text { id, length, .. } = item else {
            continue;
        };
        if *length <= 0 {
            continue;
        }
        let length = *length as usize;
        if index < cursor + length {
            return Ok(CrdtItemId::Operation(operation_id_at(id, index - cursor)?));
        }
        cursor += length;
    }
    Err(format!("cannot resolve visible CRDT item at index {index}"))
}

fn visible_item_before(slice: &SerializedTextSlice, index: usize) -> Result<CrdtItemId, String> {
    if index == 0 {
        slice.start_item_id()
    } else {
        visible_item_at(slice, index - 1)
    }
}

fn visible_item_after(slice: &SerializedTextSlice, index: usize) -> Result<CrdtItemId, String> {
    if index + 1 == visible_slice_len(slice) {
        slice.end_item_id()
    } else {
        visible_item_at(slice, index + 1)
    }
}

fn following_item_id(slice: &SerializedTextSlice, index: usize) -> Result<CrdtItemId, String> {
    for item in &slice.items[index + 1..] {
        match item {
            SerializedTextItem::Start { .. } => {
                return Ok(CrdtItemId::Boundary("start".to_string()));
            }
            SerializedTextItem::End { .. } => return Ok(CrdtItemId::Boundary("end".to_string())),
            SerializedTextItem::Split { id, .. } => return Ok(CrdtItemId::Operation(id.clone())),
            SerializedTextItem::Text { id, length, .. } if *length != 0 => {
                return Ok(CrdtItemId::Operation(id.clone()));
            }
            SerializedTextItem::Text { .. } => {}
        }
    }
    Err("title CRDT insertion origin has no following item".to_string())
}

fn operation_is_within(
    operation: &CrdtOperationId,
    start: &CrdtOperationId,
    length: i64,
) -> Result<bool, String> {
    if operation.0 != start.0 {
        return Ok(false);
    }
    let end = start
        .1
        .checked_add(length.unsigned_abs())
        .ok_or_else(|| "title CRDT text operation range overflow".to_string())?;
    Ok(operation.1 >= start.1 && operation.1 < end)
}

pub(super) fn boundary(id: CrdtItemId, anchor: AnnotationAnchor) -> AnnotationBoundary {
    AnnotationBoundary { id, anchor }
}
