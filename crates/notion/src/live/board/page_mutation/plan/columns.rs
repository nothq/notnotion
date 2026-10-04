use super::{
    metadata_operation, BuiltPageMutation, MutationIdentity, PageMutationEffect, PageMutationState,
    RecordPointer, SaveOperation,
};
use crate::model::{NotionPageBlockKind, ResizePageColumnsRequest};

pub(super) fn build_resize_columns(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ResizePageColumnsRequest,
) -> Result<BuiltPageMutation, String> {
    validate_resize_source(state, identity, request)?;
    let target = request.target();
    let destination = request.destination_weights();
    let left_pointer = RecordPointer::block(target.left_column_block_id(), identity.space_id);
    let right_pointer = RecordPointer::block(target.right_column_block_id(), identity.space_id);
    Ok(BuiltPageMutation {
        operations: vec![
            SaveOperation::update_column_ratio(left_pointer, destination.left()),
            SaveOperation::update_column_ratio(right_pointer, destination.right()),
            metadata_operation(identity, target.left_column_block_id()),
            metadata_operation(identity, identity.page_block_id),
            metadata_operation(identity, target.right_column_block_id()),
        ],
        effect: PageMutationEffect::NoAdditionalContext,
        user_action: "ColumnDivider.handleDragEnd",
    })
}

fn validate_resize_source(
    state: &PageMutationState,
    identity: &MutationIdentity<'_>,
    request: &ResizePageColumnsRequest,
) -> Result<(), String> {
    if request.page_block_id() != identity.page_block_id
        || request.page_block_id() != state.page_block_id
    {
        return Err(format!(
            "column resize belongs to page {}, active page is {}",
            request.page_block_id(),
            identity.page_block_id
        ));
    }
    let target = request.target();
    let list = state.block(target.column_list_block_id())?;
    if !list.alive || !is_layout_kind(&list.kind, "column_list") {
        return Err(format!(
            "block {} is not a live Notion column list",
            target.column_list_block_id()
        ));
    }
    if list.content_ids.iter().map(String::as_str).ne(request
        .source_columns()
        .iter()
        .map(|column| column.column_block_id()))
    {
        return Err(format!(
            "column list {} changed since the resize began",
            target.column_list_block_id()
        ));
    }
    for source in request.source_columns() {
        let column = state.block(source.column_block_id())?;
        if !column.alive
            || column.parent_table != "block"
            || column.parent_id != target.column_list_block_id()
            || !is_layout_kind(&column.kind, "column")
        {
            return Err(format!(
                "block {} is not a live child of column list {}",
                source.column_block_id(),
                target.column_list_block_id()
            ));
        }
        if column.column_ratio != source.raw_weight() {
            return Err(format!(
                "column {} ratio changed since the resize began",
                source.column_block_id()
            ));
        }
    }
    Ok(())
}

fn is_layout_kind(kind: &NotionPageBlockKind, expected: &str) -> bool {
    matches!(kind, NotionPageBlockKind::Other(value) if value == expected)
}
