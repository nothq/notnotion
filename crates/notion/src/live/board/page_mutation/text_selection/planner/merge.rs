use super::super::super::{
    blocks::MutationIdentity,
    text::{
        delete_title_range, last_slice_target, merge_operations_to_target, title_insertion_point,
        title_utf16_len, TextInsertionPoint,
    },
    wire::{RecordPointer, SaveOperation},
    CrdtClock,
};
use super::ResolvedSelection;

pub(super) struct TextMergeOperations {
    pub(super) survivor_pointer: RecordPointer,
    pub(super) survivor_insertion: TextInsertionPoint,
    pub(super) operations: Vec<SaveOperation>,
}

pub(super) fn build_text_merge_operations(
    selection: &ResolvedSelection<'_>,
    identity: &MutationIdentity<'_>,
    clock: &mut CrdtClock,
) -> Result<TextMergeOperations, String> {
    let survivor_pointer = RecordPointer::block(&selection.first.id, identity.space_id);
    let survivor_insertion =
        title_insertion_point(selection.first.title()?, selection.first_offset_utf16)?;
    let mut operations = Vec::new();
    append_intermediate_merges(
        selection,
        identity,
        &survivor_insertion,
        clock,
        &mut operations,
    )?;
    append_endpoint_merge(
        selection,
        identity,
        &survivor_pointer,
        clock,
        &mut operations,
    )?;
    Ok(TextMergeOperations {
        survivor_pointer,
        survivor_insertion,
        operations,
    })
}

fn append_intermediate_merges(
    selection: &ResolvedSelection<'_>,
    identity: &MutationIdentity<'_>,
    survivor_insertion: &TextInsertionPoint,
    clock: &mut CrdtClock,
    operations: &mut Vec<SaveOperation>,
) -> Result<(), String> {
    for block in &selection.intermediate {
        let pointer = RecordPointer::block(&block.id, identity.space_id);
        operations.extend(delete_title_range(
            pointer.clone(),
            block.title()?,
            0,
            title_utf16_len(block.title()?)?,
        )?);
        operations.extend(merge_operations_to_target(
            pointer,
            block.title()?,
            &selection.first.id,
            &survivor_insertion.slice_target,
            clock,
        )?);
    }
    Ok(())
}

fn append_endpoint_merge(
    selection: &ResolvedSelection<'_>,
    identity: &MutationIdentity<'_>,
    survivor_pointer: &RecordPointer,
    clock: &mut CrdtClock,
    operations: &mut Vec<SaveOperation>,
) -> Result<(), String> {
    operations.extend(delete_title_range(
        survivor_pointer.clone(),
        selection.first.title()?,
        selection.first_offset_utf16,
        title_utf16_len(selection.first.title()?)?,
    )?);
    let terminal_pointer = RecordPointer::block(&selection.last.id, identity.space_id);
    operations.extend(delete_title_range(
        terminal_pointer.clone(),
        selection.last.title()?,
        0,
        selection.last_offset_utf16,
    )?);
    operations.extend(merge_operations_to_target(
        terminal_pointer,
        selection.last.title()?,
        &selection.first.id,
        &last_slice_target(selection.first.title()?)?,
        clock,
    )?);
    Ok(())
}
