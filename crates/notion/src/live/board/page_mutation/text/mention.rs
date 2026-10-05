use super::super::super::page_state::PageMutationState;
use super::super::{
    blocks::{editable_block, MutationIdentity},
    wire::{AddAnnotationArgs, InsertTextArgs, RecordPointer, RemoveAnnotationArgs, SaveOperation},
    with_text_metadata, BuiltPageMutation, CrdtClock,
};
use super::annotation::{mention_token_boundaries, MentionTokenBoundaries};
use super::{
    delete_title_range, text_offset_utf8_to_utf16, title_insertion_point, title_plain_text,
    CrdtItemId, CrdtTitleState,
};
use crate::model::{
    InsertPageMentionRequest, PageMutationEffect, UpdatePageMentionRequest, PAGE_MENTION_TOKEN,
};

const INSERT_MENTION_USER_ACTION: &str = "MentionMenu.menuItem";
const UPDATE_MENTION_USER_ACTION: &str = "DateMentionEditor.updateDate";

pub(in crate::live::board::page_mutation) fn build_insert_mention(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &InsertPageMentionRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, request.block_id())?;
    let operations = insert_mention_operations(
        RecordPointer::block(&block.id, identity.space_id),
        block.title()?,
        request,
        clock,
    )?;
    Ok(BuiltPageMutation {
        operations: with_text_metadata(operations, identity, &block.id),
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: INSERT_MENTION_USER_ACTION,
    })
}

pub(in crate::live::board::page_mutation) fn build_update_mention(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &UpdatePageMentionRequest,
    clock: &mut CrdtClock,
) -> Result<BuiltPageMutation, String> {
    let block = editable_block(state, request.block_id())?;
    let operations = update_mention_operations(
        RecordPointer::block(&block.id, identity.space_id),
        block.title()?,
        request,
        clock,
    )?;
    Ok(BuiltPageMutation {
        operations: with_text_metadata(operations, identity, &block.id),
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: UPDATE_MENTION_USER_ACTION,
    })
}

/// Delete the typed trigger text and insert one annotated `‣` item, the
/// operation pair Notion's own mention menu sends.
pub(in crate::live::board::page_mutation) fn insert_mention_operations(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    request: &InsertPageMentionRequest,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let start_utf16 = text_offset_utf8_to_utf16(title, request.start_utf8())?;
    let end_utf16 = text_offset_utf8_to_utf16(title, request.end_utf8())?;
    let mut operations = if end_utf16 > start_utf16 {
        delete_title_range(pointer.clone(), title, start_utf16, end_utf16)?
    } else {
        Vec::new()
    };
    let insertion = title_insertion_point(title, start_utf16)?;
    let content = request.inserted_text();
    let first_id = clock.reserve_after(&insertion.origin_id, content.encode_utf16().count())?;
    let token_id = CrdtItemId::Operation(first_id.clone());
    operations.push(SaveOperation::insert_text(
        pointer.clone(),
        InsertTextArgs {
            operation_type: "insertText",
            text_instance_id: insertion.text_instance_id.clone(),
            search_label: insertion.search_label.clone(),
            id: first_id,
            origin_id: insertion.origin_id,
            content,
            prev_items: insertion.prev_items,
        },
    ));
    operations.push(SaveOperation::add_annotation(
        pointer,
        AddAnnotationArgs {
            operation_type: "addAnnotation",
            text_instance_id: insertion.text_instance_id,
            search_label: insertion.search_label,
            id: clock.reserve(1)?,
            start: super::annotation::token_boundary_before(token_id.clone()),
            end: super::annotation::token_boundary_after(token_id),
            annotation: request.mention().annotation_tuple(),
        },
    ));
    Ok(operations)
}

/// Replace the mention annotation of the `‣` item at the request offset.
pub(in crate::live::board::page_mutation) fn update_mention_operations(
    pointer: RecordPointer,
    title: &CrdtTitleState,
    request: &UpdatePageMentionRequest,
    clock: &mut CrdtClock,
) -> Result<Vec<SaveOperation>, String> {
    let text = title_plain_text(title)?;
    let token = text
        .get(request.offset_utf8()..)
        .and_then(|rest| rest.chars().next());
    if token != Some(PAGE_MENTION_TOKEN) {
        return Err(format!(
            "block text offset {} is not a mention token",
            request.offset_utf8()
        ));
    }
    let offset_utf16 = text_offset_utf8_to_utf16(title, request.offset_utf8())?;
    let MentionTokenBoundaries {
        text_instance_id,
        search_label,
        start,
        end,
    } = mention_token_boundaries(title, offset_utf16)?;
    Ok(vec![
        SaveOperation::remove_annotation(
            pointer.clone(),
            RemoveAnnotationArgs {
                operation_type: "removeAnnotation",
                text_instance_id: text_instance_id.clone(),
                search_label: search_label.clone(),
                id: clock.reserve(1)?,
                start: start.clone(),
                end: end.clone(),
                annotation_key: request.mention().kind().annotation_key().to_string(),
            },
        ),
        SaveOperation::add_annotation(
            pointer,
            AddAnnotationArgs {
                operation_type: "addAnnotation",
                text_instance_id,
                search_label,
                id: clock.reserve(1)?,
                start,
                end,
                annotation: request.mention().annotation_tuple(),
            },
        ),
    ])
}
