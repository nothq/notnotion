use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use super::{
    notion_block_id, notion_title_crdt_token, CalendarDateMutation, CalendarPageCreation,
    DatabaseMutationContext, FavoriteMutationContext, MoveCardMutationRequest,
};

pub(super) struct NewPageMutation {
    pub(super) block_id: String,
    pub(super) title_crdt_token: String,
    pub(super) title_crdt_root: String,
    pub(super) now: u64,
}

pub(super) fn new_page_mutation(
    context: &DatabaseMutationContext,
) -> Result<NewPageMutation, String> {
    let now = mutation_now_ms();
    let block_id = notion_block_id(context.favorite.space_short_id, now)?;
    let title_crdt_token = notion_title_crdt_token(&block_id);
    Ok(NewPageMutation {
        title_crdt_root: format!("{title_crdt_token},\"start\",\"end\""),
        block_id,
        title_crdt_token,
        now,
    })
}

pub(super) fn mutation_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .expect("system clock must be after the Unix epoch")
}

pub(super) fn create_page_operations(
    context: &DatabaseMutationContext,
    mutation: &NewPageMutation,
    status_property_id: &str,
    target_column_title: &str,
) -> Vec<Value> {
    vec![
        create_page_block_operation(context, mutation),
        create_page_status_operation(context, mutation, status_property_id, target_column_title),
        create_page_parent_operation(context, mutation),
        page_sort_append_operation(context, &mutation.block_id),
        block_edit_operation(context, &mutation.block_id, mutation.now),
        collection_view_edit_operation(context, mutation.now),
        block_edit_operation(context, &context.favorite.page_block_id, mutation.now),
    ]
}

pub(super) fn create_calendar_page_operations(
    context: &DatabaseMutationContext,
    mutation: &NewPageMutation,
    creation: &CalendarPageCreation,
) -> Vec<Value> {
    vec![
        create_page_block_operation(context, mutation),
        create_page_parent_operation(context, mutation),
        page_sort_append_operation(context, &mutation.block_id),
        calendar_page_date_operation(context, &mutation.block_id, creation),
        block_edit_operation(context, &mutation.block_id, mutation.now),
        collection_view_edit_operation(context, mutation.now),
        block_edit_operation(context, &context.favorite.page_block_id, mutation.now),
    ]
}

fn create_page_block_operation(
    context: &DatabaseMutationContext,
    mutation: &NewPageMutation,
) -> Value {
    let title_crdt_root = mutation.title_crdt_root.clone();
    json!({
        "pointer": { "table": "block", "id": mutation.block_id, "spaceId": context.favorite.space_id },
        "path": [],
        "command": "set",
        "args": {
            "id": mutation.block_id,
            "type": "page",
            "space_id": context.favorite.space_id,
            "created_time": mutation.now,
            "created_by_table": "notion_user",
            "created_by_id": context.favorite.user_id,
            "last_edited_time": mutation.now,
            "crdt_data": {
                "title": {
                    "r": title_crdt_root,
                    "n": {
                        title_crdt_root.clone(): {
                            "s": { "x": mutation.title_crdt_token, "i": [{"t": "s"}, {"t": "e"}], "l": "" },
                            "c": [],
                        }
                    }
                }
            },
            "crdt_format_version": 1,
        },
    })
}

fn create_page_status_operation(
    context: &DatabaseMutationContext,
    mutation: &NewPageMutation,
    status_property_id: &str,
    target_column_title: &str,
) -> Value {
    json!({
        "pointer": { "table": "block", "id": mutation.block_id, "spaceId": context.favorite.space_id },
        "path": ["properties"],
        "command": "update",
        "args": { status_property_id: [[target_column_title]] },
    })
}

pub(super) fn status_property_update_operation(
    context: &FavoriteMutationContext,
    page_id: &str,
    property_id: &str,
    option_label: &str,
) -> Value {
    json!({
        "pointer": { "id": page_id, "table": "block", "spaceId": context.space_id },
        "path": ["properties"],
        "command": "update",
        "args": { property_id: [[option_label]] },
    })
}

fn calendar_page_date_operation(
    context: &DatabaseMutationContext,
    block_id: &str,
    creation: &CalendarPageCreation,
) -> Value {
    let pointer = json!({
        "id": block_id,
        "table": "block",
        "spaceId": context.favorite.space_id,
    });
    json!({
        "pointer": pointer.clone(),
        "path": ["properties", creation.date_property_id.as_str()],
        "command": "updateBlockPropertyValue",
        "args": {
            "primitiveOp": {
                "command": "set",
                "args": [["‣", [["d", creation.date_value.clone()]]]],
            },
        },
        "blockPropertyValueExpectedVersions": {},
        "additionalUpdatedPointers": [pointer],
    })
}

fn create_page_parent_operation(
    context: &DatabaseMutationContext,
    mutation: &NewPageMutation,
) -> Value {
    let pointer = json!({
        "table": "block",
        "id": mutation.block_id,
        "spaceId": context.favorite.space_id,
    });
    json!({
        "pointer": pointer.clone(),
        "path": [],
        "command": "setParent",
        "args": { "parentId": context.collection_id, "parentTable": "collection" },
        "additionalUpdatedPointers": [pointer],
    })
}

fn page_sort_append_operation(context: &DatabaseMutationContext, block_id: &str) -> Value {
    json!({
        "pointer": { "table": "collection_view", "id": context.collection_view_id, "spaceId": context.favorite.space_id },
        "path": ["page_sort"],
        "command": "listAfter",
        "args": { "id": block_id },
    })
}

pub(super) fn move_card_operations(
    context: &DatabaseMutationContext,
    request: MoveCardMutationRequest<'_>,
    now: u64,
) -> Result<Vec<Value>, String> {
    let mut operations = Vec::new();
    if let Some(status_operation) = move_card_status_operation(context, request)? {
        operations.push(status_operation);
    }
    operations.push(page_sort_move_operation(context, request)?);
    operations.push(block_edit_operation(context, request.block_id, now));
    operations.push(collection_view_edit_operation(context, now));
    operations.push(block_edit_operation(
        context,
        &context.favorite.page_block_id,
        now,
    ));
    Ok(operations)
}

pub(super) fn calendar_date_operations(
    context: &DatabaseMutationContext,
    mutation: &CalendarDateMutation,
    now: u64,
) -> Vec<Value> {
    vec![
        json!({
            "pointer": {
                "id": mutation.block_id.as_str(),
                "table": "block",
                "spaceId": context.favorite.space_id,
            },
            "path": ["properties", mutation.date_property_id.as_str()],
            "command": "set",
            "args": [["‣", [["d", mutation.date_value.clone()]]]],
        }),
        block_edit_operation(context, &mutation.block_id, now),
        collection_view_edit_operation(context, now),
        block_edit_operation(context, &context.favorite.page_block_id, now),
    ]
}

fn move_card_status_operation(
    context: &DatabaseMutationContext,
    request: MoveCardMutationRequest<'_>,
) -> Result<Option<Value>, String> {
    if request.source_column_title == request.target_column_title {
        return Ok(None);
    }
    let status_property_id = context
        .board_group()?
        .property_for_moved_card(request.target_column_title)?;
    Ok(Some(json!({
        "pointer": { "id": request.block_id, "table": "block", "spaceId": context.favorite.space_id },
        "path": ["properties"],
        "command": "update",
        "args": { status_property_id: [[request.target_column_title]] },
    })))
}

fn page_sort_move_operation(
    context: &DatabaseMutationContext,
    request: MoveCardMutationRequest<'_>,
) -> Result<Value, String> {
    let pointer = json!({
        "table": "collection_view",
        "id": context.collection_view_id,
        "spaceId": context.favorite.space_id,
    });
    if let Some(before_block_id) = request.before_block_id {
        return Ok(json!({
            "pointer": pointer,
            "path": ["page_sort"],
            "command": "listBefore",
            "args": { "before": before_block_id, "id": request.block_id },
        }));
    }
    if let Some(after_block_id) = request.after_block_id {
        return Ok(json!({
            "pointer": pointer,
            "path": ["page_sort"],
            "command": "listAfter",
            "args": { "after": after_block_id, "id": request.block_id },
        }));
    }
    Err("missing Notion page_sort anchor for drag mutation".to_string())
}

fn block_edit_operation(context: &DatabaseMutationContext, block_id: &str, now: u64) -> Value {
    json!({
        "pointer": { "id": block_id, "table": "block", "spaceId": context.favorite.space_id },
        "path": [],
        "command": "update",
        "args": {
            "last_edited_time": now,
            "last_edited_by_id": context.favorite.user_id,
            "last_edited_by_table": "notion_user",
        },
    })
}

fn collection_view_edit_operation(context: &DatabaseMutationContext, now: u64) -> Value {
    json!({
        "pointer": { "table": "collection_view", "id": context.collection_view_id, "spaceId": context.favorite.space_id },
        "path": [],
        "command": "update",
        "args": { "last_edited_time": now },
    })
}
