use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use super::{
    notion_block_id, record_map_table, reject_transaction_errors, FavoriteMutationContext,
    LiveBoardMutator, MutationContext, NotionPrivateApiEndpoint,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_in_space_with_session,
    NotionLiveError,
};
use crate::model::{
    NotionCommentDraftSegment, NotionCommentMutation, NotionCommentMutationRequest,
    NotionWorkspaceUser,
};

impl LiveBoardMutator {
    pub(crate) fn apply_comment_mutation(
        &self,
        session: &NotionDesktopSession,
        request: &NotionCommentMutationRequest,
        visible_users: &[NotionWorkspaceUser],
    ) -> Result<(), NotionLiveError> {
        let context = match &self.context {
            MutationContext::Page(context) => context,
            MutationContext::Database(context) => &context.favorite,
        };
        let page_id = request.page_id().as_str();
        let target_id = request.target_id().as_str();
        let state = self.page_states.get(page_id).ok_or_else(|| {
            format!("Notion page {page_id} has not been loaded for comment mutation")
        })?;
        if state.space_id != context.space_id {
            return Err(NotionLiveError::Fatal(format!(
                "Notion page {page_id} is loaded from a different space"
            )));
        }
        let target = state.block(target_id)?;
        if !target.alive {
            return Err(NotionLiveError::Fatal(format!(
                "Notion comment target block {target_id} is not alive"
            )));
        }
        let response = self
            .page_responses
            .get(page_id)
            .ok_or_else(|| format!("Notion page {page_id} is missing its cached response"))?;
        validate_comment_role(response.as_value(), page_id)?;
        let page = super::page::card_page_from_response(page_id, response)?;
        validate_expected_comment_state(&page, request)?;
        validate_mentions(request, visible_users)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?;
        let comment_id = notion_block_id(context.space_short_id, now)?;
        let operations = comment_operations(context, request, &comment_id, now)?;
        let response = post_private_api_in_space_with_session(
            session,
            NotionPrivateApiEndpoint::SaveTransactionsFanout,
            &context.space_id,
            &json!({
                "requestId": uuid::Uuid::new_v4().to_string(),
                "transactions": [{
                    "id": uuid::Uuid::new_v4().to_string(),
                    "spaceId": context.space_id,
                    "operations": operations,
                }],
            }),
        )?;
        reject_transaction_errors(&response, "comment").map_err(NotionLiveError::Fatal)
    }
}

fn validate_comment_role(response: &Value, page_id: &str) -> Result<(), NotionLiveError> {
    let entry = record_map_table(response, "block")?
        .get(page_id)
        .ok_or_else(|| format!("Notion page response omitted root block {page_id}"))?;
    let role = entry
        .get("role")
        .and_then(Value::as_str)
        .or_else(|| {
            entry
                .get("value")
                .and_then(Value::as_object)
                .and_then(|value| value.get("role"))
                .and_then(Value::as_str)
        })
        .ok_or_else(|| format!("Notion page {page_id} omitted its effective role"))?;
    if matches!(
        role,
        "comment_only" | "content_only_editor" | "read_and_write" | "membership_admin" | "editor"
    ) {
        return Ok(());
    }
    Err(NotionLiveError::Fatal(format!(
        "Notion page {page_id} role {role} cannot create comments"
    )))
}

fn validate_expected_comment_state(
    page: &crate::model::CardPage,
    request: &NotionCommentMutationRequest,
) -> Result<(), String> {
    let current_discussion_ids = page
        .discussions
        .iter()
        .filter(|discussion| discussion.target_block_id == *request.target_id())
        .map(|discussion| discussion.discussion_id.clone())
        .collect::<Vec<_>>();
    if current_discussion_ids != request.expected_discussion_ids() {
        return Err(format!(
            "Notion comment target {} changed since the composer opened",
            request.target_id().as_str()
        ));
    }
    let NotionCommentMutation::Reply {
        discussion_id,
        expected_comment_ids,
    } = request.mutation()
    else {
        return Ok(());
    };
    let discussion = page
        .discussions
        .iter()
        .find(|discussion| discussion.discussion_id == *discussion_id)
        .ok_or_else(|| {
            format!(
                "Notion discussion {} is no longer loaded",
                discussion_id.as_str()
            )
        })?;
    if discussion.target_block_id != *request.target_id() || !discussion.access.is_writable() {
        return Err(format!(
            "Notion discussion {} is no longer writable for target {}",
            discussion_id.as_str(),
            request.target_id().as_str()
        ));
    }
    let current_comment_ids = discussion
        .comments
        .iter()
        .map(|comment| comment.comment_id.clone())
        .collect::<Vec<_>>();
    if current_comment_ids != *expected_comment_ids {
        return Err(format!(
            "Notion discussion {} changed since the reply composer opened",
            discussion_id.as_str()
        ));
    }
    Ok(())
}

fn validate_mentions(
    request: &NotionCommentMutationRequest,
    visible_users: &[NotionWorkspaceUser],
) -> Result<(), String> {
    let visible_user_ids = visible_users
        .iter()
        .map(|user| user.user_id.as_str())
        .collect::<HashSet<_>>();
    for user_id in request.draft().mentioned_user_ids() {
        if !visible_user_ids.contains(user_id.as_str()) {
            return Err(format!(
                "Notion mention user {} is not in the authoritative visible workspace set",
                user_id.as_str()
            ));
        }
    }
    Ok(())
}

fn comment_operations(
    context: &FavoriteMutationContext,
    request: &NotionCommentMutationRequest,
    comment_id: &str,
    now: u64,
) -> Result<Vec<Value>, String> {
    let text = Value::Array(
        request
            .draft()
            .segments()
            .iter()
            .map(|segment| match segment {
                NotionCommentDraftSegment::Plain(text) => json!([text]),
                NotionCommentDraftSegment::Mention(user_id) => {
                    json!(["‣", [["u", user_id.as_str()]]])
                }
            })
            .collect(),
    );
    match request.mutation() {
        NotionCommentMutation::NewThread => {
            let discussion_id = notion_block_id(context.space_short_id, now)?;
            Ok(vec![
                json!({
                    "pointer": { "table": "discussion", "id": discussion_id, "spaceId": context.space_id },
                    "path": [],
                    "command": "update",
                    "args": {
                        "type": "default",
                        "parent_id": request.target_id().as_str(),
                        "parent_table": "block",
                        "resolved": false,
                        "space_id": context.space_id,
                    },
                }),
                new_comment_operation(context, &discussion_id, comment_id, now),
                append_comment_operation(context, &discussion_id, comment_id),
                set_comment_text_operation(context, comment_id, text),
                json!({
                    "pointer": { "table": "block", "id": request.target_id().as_str(), "spaceId": context.space_id },
                    "path": ["discussions"],
                    "command": "listAfter",
                    "args": { "id": discussion_id },
                }),
            ])
        }
        NotionCommentMutation::Reply { discussion_id, .. } => Ok(vec![
            new_comment_operation(context, discussion_id.as_str(), comment_id, now),
            append_comment_operation(context, discussion_id.as_str(), comment_id),
            set_comment_text_operation(context, comment_id, text),
        ]),
    }
}

fn new_comment_operation(
    context: &FavoriteMutationContext,
    discussion_id: &str,
    comment_id: &str,
    now: u64,
) -> Value {
    json!({
        "pointer": { "table": "comment", "id": comment_id, "spaceId": context.space_id },
        "path": [],
        "command": "set",
        "args": {
            "id": comment_id,
            "parent_id": discussion_id,
            "parent_table": "discussion",
            "alive": true,
            "space_id": context.space_id,
            "created_time": now,
            "last_edited_time": now,
            "version": 1,
        },
    })
}

fn append_comment_operation(
    context: &FavoriteMutationContext,
    discussion_id: &str,
    comment_id: &str,
) -> Value {
    json!({
        "pointer": { "table": "discussion", "id": discussion_id, "spaceId": context.space_id },
        "path": ["comments"],
        "command": "listAfter",
        "args": { "id": comment_id },
    })
}

fn set_comment_text_operation(
    context: &FavoriteMutationContext,
    comment_id: &str,
    text: Value,
) -> Value {
    json!({
        "pointer": { "table": "comment", "id": comment_id, "spaceId": context.space_id },
        "path": ["text"],
        "command": "set",
        "args": text,
    })
}
