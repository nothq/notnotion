use super::super::wire::{
    insert_text_prev_items, CrdtIdRange, DeleteTextArgs, InsertTextArgs, InsertTextPrevItem,
    RecordPointer, SaveOperation,
};
use super::{
    CrdtClock, CrdtItemId, CrdtOperationId, CrdtTitleState, SerializedTextItem,
    SerializedTextSlice, SerializedTextSliceTree, TextSliceTarget,
};

struct VisibleSpan<'a> {
    slice: &'a SerializedTextSlice,
    id: &'a CrdtOperationId,
    content: &'a str,
    start_utf16: usize,
    end_utf16: usize,
}

type DeletionGroup<'a> = (&'a SerializedTextSlice, Vec<CrdtIdRange>);

pub(in crate::live::board::page_mutation) struct TextInsertionPoint {
    pub(in crate::live::board::page_mutation) text_instance_id: String,
    pub(in crate::live::board::page_mutation) search_label: String,
    pub(in crate::live::board::page_mutation) origin_id: CrdtItemId,
    pub(in crate::live::board::page_mutation) prev_items: Vec<InsertTextPrevItem>,
    pub(in crate::live::board::page_mutation) slice_target: TextSliceTarget,
}

pub(in crate::live::board::page_mutation) fn replacement_operations(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    replacement: &str,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let spans = visible_spans(&title.tree)?;
    let current = spans.iter().map(|span| span.content).collect::<String>();
    if current == replacement {
        return Ok(Vec::new());
    }
    let (prefix_chars, suffix_chars) = shared_char_edges(&current, replacement);
    let current_chars = current.chars().collect::<Vec<_>>();
    let replacement_chars = replacement.chars().collect::<Vec<_>>();
    let delete_start = utf16_len(&current_chars[..prefix_chars]);
    let delete_end = utf16_len(&current_chars[..current_chars.len() - suffix_chars]);
    let inserted = replacement_chars[prefix_chars..replacement_chars.len() - suffix_chars]
        .iter()
        .collect::<String>();
    let insertion = insertion_point(&title.tree, &spans, delete_start)?;
    let mut operations = deletion_operations(pointer.clone(), &spans, delete_start, delete_end);
    if !inserted.is_empty() {
        operations.push(SaveOperation::insert_text(
            pointer,
            InsertTextArgs {
                operation_type: "insertText",
                text_instance_id: insertion.text_instance_id,
                search_label: insertion.search_label,
                id: clock.reserve_after(&insertion.origin_id, inserted.encode_utf16().count())?,
                origin_id: insertion.origin_id,
                content: inserted,
                prev_items: insertion.prev_items,
            },
        ));
    }
    Ok(operations)
}

fn visible_spans(tree: &SerializedTextSliceTree) -> Result<Vec<VisibleSpan<'_>>, String> {
    let mut spans = Vec::new();
    let mut offset = 0;
    for key in tree.ordered_keys() {
        let slice = &tree.node(key).text_slice;
        for item in &slice.items {
            let SerializedTextItem::Text {
                id,
                length,
                content,
                ..
            } = item
            else {
                continue;
            };
            if *length <= 0 {
                continue;
            }
            let content = content
                .as_deref()
                .ok_or_else(|| "visible title CRDT item is missing content".to_string())?;
            let end_utf16 = offset + *length as usize;
            spans.push(VisibleSpan {
                slice,
                id,
                content,
                start_utf16: offset,
                end_utf16,
            });
            offset = end_utf16;
        }
    }
    Ok(spans)
}

pub(in crate::live::board::page_mutation) fn title_utf16_len(
    title: &CrdtTitleState,
) -> Result<usize, String> {
    Ok(visible_spans(&title.tree)?
        .last()
        .map_or(0, |span| span.end_utf16))
}

pub(in crate::live::board::page_mutation) fn title_plain_text(
    title: &CrdtTitleState,
) -> Result<String, String> {
    Ok(visible_spans(&title.tree)?
        .iter()
        .map(|span| span.content)
        .collect())
}

pub(in crate::live::board::page_mutation) fn text_offset_utf8_to_utf16(
    title: &CrdtTitleState,
    offset_utf8: usize,
) -> Result<usize, String> {
    let text = visible_spans(&title.tree)?
        .iter()
        .map(|span| span.content)
        .collect::<String>();
    if offset_utf8 > text.len() {
        return Err(format!(
            "text offset {offset_utf8} exceeds UTF-8 length {}",
            text.len()
        ));
    }
    if !text.is_char_boundary(offset_utf8) {
        return Err(format!(
            "text offset {offset_utf8} splits a UTF-8 scalar value"
        ));
    }
    Ok(text[..offset_utf8].encode_utf16().count())
}

pub(in crate::live::board::page_mutation) fn delete_title_range(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    start_utf16: usize,
    end_utf16: usize,
) -> Result<Vec<SaveOperation>, String> {
    let spans = visible_spans(&title.tree)?;
    let length = spans.last().map_or(0, |span| span.end_utf16);
    if start_utf16 > end_utf16 || end_utf16 > length {
        return Err(format!(
            "invalid title deletion range {start_utf16}..{end_utf16} for UTF-16 length {length}"
        ));
    }
    Ok(deletion_operations(pointer, &spans, start_utf16, end_utf16))
}

pub(in crate::live::board::page_mutation) fn title_insertion_point(
    title: &CrdtTitleState,
    offset_utf16: usize,
) -> Result<TextInsertionPoint, String> {
    let spans = visible_spans(&title.tree)?;
    let length = spans.last().map_or(0, |span| span.end_utf16);
    if offset_utf16 > length {
        return Err(format!(
            "insertion offset {offset_utf16} exceeds title UTF-16 length {length}"
        ));
    }
    insertion_point(&title.tree, &spans, offset_utf16)
}

fn deletion_operations(
    pointer: RecordPointer,
    spans: &[VisibleSpan<'_>],
    start: usize,
    end: usize,
) -> Vec<SaveOperation> {
    let mut grouped: Vec<DeletionGroup<'_>> = Vec::new();
    for span in spans {
        let intersection_start = start.max(span.start_utf16);
        let intersection_end = end.min(span.end_utf16);
        if intersection_start >= intersection_end {
            continue;
        }
        let id = CrdtOperationId(
            span.id.0.clone(),
            span.id.1 + (intersection_start - span.start_utf16) as u64,
        );
        let range = (id, (intersection_end - intersection_start) as u64);
        if let Some((_, ranges)) = grouped
            .last_mut()
            .filter(|(slice, _)| std::ptr::eq(*slice, span.slice))
        {
            append_range(ranges, range);
        } else {
            grouped.push((span.slice, vec![range]));
        }
    }
    grouped
        .into_iter()
        .map(|(slice, id_ranges)| {
            SaveOperation::delete_text(
                pointer.clone(),
                DeleteTextArgs {
                    operation_type: "deleteText",
                    text_instance_id: slice.text_instance_id.clone(),
                    search_label: slice.search_label.clone(),
                    id_ranges,
                },
            )
        })
        .collect()
}

fn append_range(ranges: &mut Vec<CrdtIdRange>, next: CrdtIdRange) {
    if let Some((last_id, last_length)) = ranges.last_mut() {
        if last_id.0 == (next.0).0 && last_id.1 + *last_length == (next.0).1 {
            *last_length += next.1;
            return;
        }
    }
    ranges.push(next);
}

fn insertion_point(
    tree: &SerializedTextSliceTree,
    spans: &[VisibleSpan<'_>],
    offset: usize,
) -> Result<TextInsertionPoint, String> {
    let (slice, origin_id) = if offset == 0 {
        let root = &tree.node(&tree.root_key).text_slice;
        (root, root.start_item_id()?)
    } else {
        let span = spans
            .iter()
            .find(|span| offset > span.start_utf16 && offset <= span.end_utf16)
            .ok_or_else(|| format!("cannot resolve CRDT insertion offset {offset}"))?;
        (
            span.slice,
            CrdtItemId::Operation(CrdtOperationId(
                span.id.0.clone(),
                span.id.1 + (offset - span.start_utf16 - 1) as u64,
            )),
        )
    };
    let prev_items = insert_text_prev_items(slice, &origin_id)?;
    Ok(TextInsertionPoint {
        text_instance_id: slice.text_instance_id.clone(),
        search_label: slice.search_label.clone(),
        origin_id,
        prev_items,
        slice_target: TextSliceTarget {
            text_instance_id: slice.text_instance_id.clone(),
            search_label: slice.search_label.clone(),
            end_item_id: slice.end_item_id()?,
        },
    })
}

fn shared_char_edges(current: &str, replacement: &str) -> (usize, usize) {
    let current = current.chars().collect::<Vec<_>>();
    let replacement = replacement.chars().collect::<Vec<_>>();
    let prefix = current
        .iter()
        .zip(&replacement)
        .take_while(|(left, right)| left == right)
        .count();
    let remaining = current.len().min(replacement.len()) - prefix;
    let suffix = current[current.len() - remaining..]
        .iter()
        .rev()
        .zip(replacement[replacement.len() - remaining..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    (prefix, suffix)
}

fn utf16_len(chars: &[char]) -> usize {
    chars.iter().map(|character| character.len_utf16()).sum()
}
