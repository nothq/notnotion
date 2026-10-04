use super::super::{
    notion_title_crdt_token,
    page_state::{
        CrdtItemId, CrdtOperationId, CrdtTitleState, SerializedTextItem, SerializedTextSlice,
        SerializedTextSliceNode, SerializedTextSliceTree,
    },
    HashMap,
};
use super::{
    wire::{
        insert_text_prev_items, InsertTextArgs, MoveTextSliceArgs, MoveTextSliceIntoBlockArgs,
        RecordPointer, SaveOperation, SplitTextArgs,
    },
    CrdtClock,
};

mod annotation;
mod mention;
mod replacement;

pub(in crate::live::board::page_mutation) use annotation::annotation_tuple;
pub(super) use annotation::{plan_selection_text_edit, plan_typing_text_edit};
pub(in crate::live::board::page_mutation) use mention::{
    build_insert_mention, build_update_mention,
};
pub(super) use replacement::{
    delete_title_range, replacement_operations, text_offset_utf8_to_utf16, title_insertion_point,
    title_plain_text, title_utf16_len, TextInsertionPoint,
};

#[derive(Clone)]
pub(super) struct TextSliceTarget {
    pub(super) text_instance_id: String,
    pub(super) search_label: String,
    pub(super) end_item_id: CrdtItemId,
}

struct SlicePosition<'a> {
    key: &'a str,
    slice: &'a SerializedTextSlice,
    start_utf16: usize,
    end_utf16: usize,
}

enum SplitPoint<'a> {
    Within {
        position: &'a SlicePosition<'a>,
        origin_id: CrdtItemId,
    },
    NodeEnd {
        position: &'a SlicePosition<'a>,
    },
}

pub(super) fn empty_title_tree(block_id: &str) -> SerializedTextSliceTree {
    let text_instance_id = notion_title_crdt_token(block_id);
    let root_key = format!("{text_instance_id},\"start\",\"end\"");
    let root = SerializedTextSliceNode {
        text_slice: SerializedTextSlice {
            text_instance_id,
            items: vec![
                SerializedTextItem::Start {
                    annotations: Default::default(),
                },
                SerializedTextItem::End {
                    annotations: Default::default(),
                },
            ],
            search_label: String::new(),
        },
        child_keys: Vec::new(),
    };
    SerializedTextSliceTree {
        root_key: root_key.clone(),
        nodes: HashMap::from([(root_key, root)]),
    }
}

pub(super) fn initial_insert_operation(
    pointer: RecordPointer,
    tree: &SerializedTextSliceTree,
    text: &str,
    clock: &mut CrdtClock,
) -> Result<Option<SaveOperation>, String> {
    if text.is_empty() {
        return Ok(None);
    }
    let root = &tree.node(&tree.root_key).text_slice;
    let origin_id = root.start_item_id()?;
    Ok(Some(SaveOperation::insert_text(
        pointer,
        InsertTextArgs {
            operation_type: "insertText",
            text_instance_id: root.text_instance_id.clone(),
            search_label: root.search_label.clone(),
            id: clock.reserve_after(&origin_id, text.encode_utf16().count())?,
            prev_items: insert_text_prev_items(root, &origin_id)?,
            origin_id,
            content: text.to_string(),
        },
    )))
}

pub(super) fn split_operations(
    pointer: RecordPointer,
    target_block_id: &str,
    title: &CrdtTitleState,
    offset_utf16: usize,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let positions = slice_positions(&title.tree)?;
    validate_split_offset(&positions, offset_utf16)?;
    let split_point = split_point(&positions, offset_utf16)?;
    let (position, start_item_id, include_children, split_operation) = match split_point {
        SplitPoint::Within {
            position,
            origin_id,
        } => {
            let split_id = match position.slice.end_item_id()? {
                CrdtItemId::Operation(clock_floor) => {
                    // Notion keeps the visible-text item as the split origin,
                    // but an evolved slice requires the new operation clock
                    // to advance past its terminal Split marker.
                    clock.reserve_after_with_clock_floor(&origin_id, &clock_floor, 1)?
                }
                CrdtItemId::Boundary(_) => clock.reserve_after(&origin_id, 1)?,
            };
            let operation = SaveOperation::split_text(
                pointer.clone(),
                SplitTextArgs {
                    operation_type: "splitText",
                    text_instance_id: position.slice.text_instance_id.clone(),
                    search_label: position.slice.search_label.clone(),
                    id: split_id.clone(),
                    origin_id,
                },
            );
            (
                position,
                CrdtItemId::Operation(split_id),
                true,
                Some(operation),
            )
        }
        SplitPoint::NodeEnd { position } => {
            (position, position.slice.start_item_id()?, false, None)
        }
    };
    let mut operations = move_following_slices(pointer.clone(), title, position, include_children)?;
    operations.extend(split_operation);
    operations.push(SaveOperation::move_text_slice_into_block(
        pointer,
        MoveTextSliceIntoBlockArgs {
            text_instance_id: position.slice.text_instance_id.clone(),
            text_slice_start_item_id: start_item_id,
            source_search_label: position.slice.search_label.clone(),
            target_block_id: target_block_id.to_string(),
        },
    ));
    Ok(operations)
}

fn validate_split_offset(
    positions: &[SlicePosition<'_>],
    offset_utf16: usize,
) -> Result<(), String> {
    let total_utf16 = positions.last().map_or(0, |position| position.end_utf16);
    if offset_utf16 <= total_utf16 {
        return Ok(());
    }
    Err(format!(
        "split offset {offset_utf16} exceeds title length {total_utf16}"
    ))
}

pub(super) fn merge_operations(
    source_pointer: RecordPointer,
    source_title: &CrdtTitleState,
    target_block_id: &str,
    target_title: &CrdtTitleState,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let target = last_slice_target(target_title)?;
    merge_operations_to_target(
        source_pointer,
        source_title,
        target_block_id,
        &target,
        clock,
    )
}

pub(super) fn last_slice_target(title: &CrdtTitleState) -> Result<TextSliceTarget, String> {
    let target_key = title
        .tree
        .ordered_keys()
        .into_iter()
        .last()
        .ok_or_else(|| "target title CRDT is empty".to_string())?;
    let target = &title.tree.node(target_key).text_slice;
    Ok(TextSliceTarget {
        text_instance_id: target.text_instance_id.clone(),
        search_label: target.search_label.clone(),
        end_item_id: target.end_item_id()?,
    })
}

pub(super) fn merge_operations_to_target(
    source_pointer: RecordPointer,
    source_title: &CrdtTitleState,
    target_block_id: &str,
    target: &TextSliceTarget,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let source = &source_title
        .tree
        .node(&source_title.tree.root_key)
        .text_slice;
    clock.skip(20)?;
    let origin_id = source.start_item_id()?;
    let split_id = clock.reserve_after(&origin_id, 1)?;
    Ok(vec![
        SaveOperation::split_text(
            source_pointer.clone(),
            SplitTextArgs {
                operation_type: "splitText",
                text_instance_id: source.text_instance_id.clone(),
                search_label: source.search_label.clone(),
                id: split_id.clone(),
                origin_id,
            },
        ),
        SaveOperation::move_text_slice(
            source_pointer,
            MoveTextSliceArgs {
                text_instance_id: source.text_instance_id.clone(),
                text_slice_start_item_id: CrdtItemId::Operation(split_id),
                source_search_label: source.search_label.clone(),
                target_block_id: target_block_id.to_string(),
                target_text_instance_id: target.text_instance_id.clone(),
                target_search_label: target.search_label.clone(),
                target_text_slice_end_item_id: target.end_item_id.clone(),
                is_merge: Some(true),
            },
        ),
    ])
}

fn slice_positions(tree: &SerializedTextSliceTree) -> Result<Vec<SlicePosition<'_>>, String> {
    let mut positions = Vec::new();
    let mut offset = 0;
    for key in tree.ordered_keys() {
        let slice = &tree.node(key).text_slice;
        let length = slice
            .items
            .iter()
            .filter_map(|item| match item {
                SerializedTextItem::Text { length, .. } if *length > 0 => Some(*length as usize),
                _ => None,
            })
            .sum::<usize>();
        positions.push(SlicePosition {
            key,
            slice,
            start_utf16: offset,
            end_utf16: offset + length,
        });
        offset += length;
    }
    if positions.is_empty() {
        return Err("title CRDT contains no slices".to_string());
    }
    Ok(positions)
}

fn split_point<'a>(
    positions: &'a [SlicePosition<'a>],
    offset: usize,
) -> Result<SplitPoint<'a>, String> {
    for (index, position) in positions.iter().enumerate() {
        if offset == position.end_utf16 {
            if let Some(next) = positions.get(index + 1) {
                return Ok(SplitPoint::NodeEnd { position: next });
            }
        }
        if offset >= position.start_utf16 && offset <= position.end_utf16 {
            return Ok(SplitPoint::Within {
                position,
                origin_id: origin_in_slice(position, offset)?,
            });
        }
    }
    Err(format!("cannot resolve CRDT split offset {offset}"))
}

fn origin_in_slice(position: &SlicePosition<'_>, offset: usize) -> Result<CrdtItemId, String> {
    let local_offset = offset - position.start_utf16;
    if local_offset == 0 {
        return position.slice.start_item_id();
    }
    let mut cursor = 0;
    for item in &position.slice.items {
        let SerializedTextItem::Text { id, length, .. } = item else {
            continue;
        };
        if *length <= 0 {
            continue;
        }
        let end = cursor + *length as usize;
        if local_offset <= end {
            return Ok(CrdtItemId::Operation(CrdtOperationId(
                id.0.clone(),
                id.1 + (local_offset - cursor - 1) as u64,
            )));
        }
        cursor = end;
    }
    Err(format!("cannot resolve CRDT split offset {offset}"))
}

fn move_following_slices(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    target: &SlicePosition<'_>,
    include_target_children: bool,
) -> Result<Vec<SaveOperation>, String> {
    let target_end = target.slice.end_item_id()?;
    title
        .tree
        .following_top_level_keys(target.key, include_target_children)?
        .into_iter()
        .map(|key| {
            let source = &title.tree.node(key).text_slice;
            Ok(SaveOperation::move_text_slice(
                pointer.clone(),
                MoveTextSliceArgs {
                    text_instance_id: source.text_instance_id.clone(),
                    text_slice_start_item_id: source.start_item_id()?,
                    source_search_label: source.search_label.clone(),
                    target_block_id: pointer.id.clone(),
                    target_text_instance_id: target.slice.text_instance_id.clone(),
                    target_search_label: target.slice.search_label.clone(),
                    target_text_slice_end_item_id: target_end.clone(),
                    is_merge: None,
                },
            ))
        })
        .collect()
}
