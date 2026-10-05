use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::model::{
    CardPageComment, CardPageCommentAuthor, CardPageCommentRichSegment, CardPageDiscussion,
    CardPageDiscussionAccess, NotionCommentId, NotionCommentTargetId, NotionDiscussionId,
    NotionUserId,
};

use super::super::{loaded_record_value, required_array, required_string, required_u64};

mod content;

use content::{parse_attachment_content, parse_comment_text};

pub(super) fn page_discussions(
    page_id: &str,
    page_block_ids: impl IntoIterator<Item = String>,
    records: PageCommentRecords<'_>,
) -> Result<Vec<CardPageDiscussion>, String> {
    let mut state = PageCommentParseState::default();
    let mut targets = Vec::new();
    targets.push(page_id.to_string());
    targets.extend(page_block_ids);
    for target_id in targets {
        parse_target_discussions(&target_id, &records, &mut state)?;
    }
    Ok(state.discussions)
}

pub(super) struct PageCommentRecords<'a> {
    blocks: &'a Map<String, Value>,
    discussions: Option<&'a Map<String, Value>>,
    comments: Option<&'a Map<String, Value>>,
    users: Option<&'a Map<String, Value>>,
}

#[derive(Default)]
struct PageCommentParseState {
    discussion_ids: HashSet<NotionDiscussionId>,
    comment_ids: HashSet<NotionCommentId>,
    discussions: Vec<CardPageDiscussion>,
}

fn parse_target_discussions(
    target_id: &str,
    records: &PageCommentRecords<'_>,
    state: &mut PageCommentParseState,
) -> Result<(), String> {
    let target = records
        .blocks
        .get(target_id)
        .and_then(loaded_record_value)
        .ok_or_else(|| format!("missing hydrated Notion comment target block {target_id}"))?;
    let Some(raw_discussions) = target.get("discussions") else {
        return Ok(());
    };
    let raw_discussions = raw_discussions
        .as_array()
        .ok_or_else(|| format!("Notion block {target_id} contains non-array discussions"))?;
    let target_space_id = required_string(target, "space_id")?;
    for raw_discussion_id in raw_discussions {
        let discussion = parse_discussion(
            raw_discussion_id,
            target_id,
            target_space_id,
            records,
            state,
        )?;
        state.discussions.push(discussion);
    }
    Ok(())
}

fn parse_discussion(
    raw_discussion_id: &Value,
    target_id: &str,
    target_space_id: &str,
    records: &PageCommentRecords<'_>,
    state: &mut PageCommentParseState,
) -> Result<CardPageDiscussion, String> {
    let discussion_id = parse_discussion_id(raw_discussion_id, target_id)?;
    if !state.discussion_ids.insert(discussion_id.clone()) {
        return Err(format!(
            "Notion discussion {} is referenced more than once",
            discussion_id.as_str()
        ));
    }
    let discussion = referenced_record(records.discussions, "discussion", discussion_id.as_str())?;
    validate_parent(
        discussion,
        "discussion",
        discussion_id.as_str(),
        ExpectedParent {
            table: "block",
            id: target_id,
            space_id: target_space_id,
        },
    )?;
    let discussion_type = required_string(discussion, "type")?;
    let property_id = optional_property_id(discussion, discussion_id.as_str())?;
    let resolved = discussion_resolved(discussion, &discussion_id)?;
    let comments =
        parse_discussion_comments(discussion, &discussion_id, target_space_id, records, state)?;
    Ok(CardPageDiscussion::new(
        discussion_id.clone(),
        NotionCommentTargetId::try_from(target_id.to_string()).map_err(str::to_string)?,
        resolved,
        CardPageDiscussionAccess::for_discussion(resolved, discussion_type, property_id),
        comments,
    ))
}

fn parse_discussion_id(
    raw_discussion_id: &Value,
    target_id: &str,
) -> Result<NotionDiscussionId, String> {
    let discussion_id = raw_discussion_id
        .as_str()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion block {target_id} contains an invalid discussion ID"))?;
    NotionDiscussionId::try_from(discussion_id.to_string()).map_err(str::to_string)
}

fn discussion_resolved(
    discussion: &Value,
    discussion_id: &NotionDiscussionId,
) -> Result<bool, String> {
    discussion
        .get("resolved")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            format!(
                "Notion discussion {} is missing boolean resolved",
                discussion_id.as_str()
            )
        })
}

fn parse_discussion_comments(
    discussion: &Value,
    discussion_id: &NotionDiscussionId,
    target_space_id: &str,
    records: &PageCommentRecords<'_>,
    state: &mut PageCommentParseState,
) -> Result<Vec<CardPageComment>, String> {
    let mut comments = Vec::new();
    for raw_comment_id in required_array(discussion, "comments")? {
        let comment_id = parse_comment_id(raw_comment_id, discussion_id)?;
        if !state.comment_ids.insert(comment_id.clone()) {
            return Err(format!(
                "Notion comment {} is referenced more than once",
                comment_id.as_str()
            ));
        }
        if let Some(comment) =
            parse_referenced_comment(comment_id, discussion_id, target_space_id, records)?
        {
            comments.push(comment);
        }
    }
    Ok(comments)
}

fn parse_comment_id(
    raw_comment_id: &Value,
    discussion_id: &NotionDiscussionId,
) -> Result<NotionCommentId, String> {
    let comment_id = raw_comment_id
        .as_str()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "Notion discussion {} contains an invalid comment ID",
                discussion_id.as_str()
            )
        })?;
    NotionCommentId::try_from(comment_id.to_string()).map_err(str::to_string)
}

fn parse_referenced_comment(
    comment_id: NotionCommentId,
    discussion_id: &NotionDiscussionId,
    target_space_id: &str,
    records: &PageCommentRecords<'_>,
) -> Result<Option<CardPageComment>, String> {
    let comment = referenced_record(records.comments, "comment", comment_id.as_str())?;
    validate_parent(
        comment,
        "comment",
        comment_id.as_str(),
        ExpectedParent {
            table: "discussion",
            id: discussion_id.as_str(),
            space_id: target_space_id,
        },
    )?;
    let alive = comment
        .get("alive")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            format!(
                "Notion comment {} is missing boolean alive",
                comment_id.as_str()
            )
        })?;
    if !alive {
        return Ok(None);
    }
    parse_comment(comment_id, comment, records.users).map(Some)
}

fn optional_property_id<'a>(
    discussion: &'a Value,
    discussion_id: &str,
) -> Result<Option<&'a str>, String> {
    match discussion.get("property_id") {
        None => Ok(None),
        Some(Value::String(property_id)) if !property_id.trim().is_empty() => Ok(Some(property_id)),
        Some(_) => Err(format!(
            "Notion discussion {discussion_id} contains an invalid property ID"
        )),
    }
}

fn parse_comment(
    comment_id: NotionCommentId,
    comment: &Value,
    users: Option<&Map<String, Value>>,
) -> Result<CardPageComment, String> {
    if comment.get("created_by_table").and_then(Value::as_str) != Some("notion_user") {
        return Err(format!(
            "Notion comment {} has an unsupported author table",
            comment_id.as_str()
        ));
    }
    let author_id = NotionUserId::try_from(required_string(comment, "created_by_id")?.to_string())
        .map_err(str::to_string)?;
    let author = comment_user(users, &author_id)?;
    let mut rich_text = parse_comment_text(comment.get("text"), users, comment_id.as_str())?;
    let attachment_count = parse_attachment_content(comment, comment_id.as_str())?;
    if attachment_count > 0 {
        rich_text.push(CardPageCommentRichSegment::unsupported(
            if attachment_count == 1 {
                "Attachment".to_string()
            } else {
                format!("{attachment_count} attachments")
            },
        ));
    }
    if rich_text.is_empty() {
        return Err(format!(
            "Notion comment {} has neither text nor attachment content",
            comment_id.as_str()
        ));
    }
    CardPageComment::new(
        comment_id,
        author,
        required_u64(comment, "created_time")?,
        required_u64(comment, "last_edited_time")?,
        rich_text,
    )
}

fn comment_user(
    users: Option<&Map<String, Value>>,
    user_id: &NotionUserId,
) -> Result<CardPageCommentAuthor, String> {
    let user = users
        .and_then(|users| users.get(user_id.as_str()))
        .and_then(loaded_record_value)
        .ok_or_else(|| format!("missing hydrated Notion comment user {}", user_id.as_str()))?;
    if required_string(user, "id")? != user_id.as_str() {
        return Err(format!(
            "Notion comment user key {} contains a different user",
            user_id.as_str()
        ));
    }
    let profile_photo = match user.get("profile_photo") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        Some(_) => {
            return Err(format!(
                "Notion comment user {} has a non-string profile photo",
                user_id.as_str()
            ))
        }
    };
    CardPageCommentAuthor::new(
        user_id.clone(),
        required_string(user, "name")?.to_string(),
        profile_photo,
    )
}

struct ExpectedParent<'a> {
    table: &'a str,
    id: &'a str,
    space_id: &'a str,
}

fn validate_parent(
    value: &Value,
    table: &str,
    record_id: &str,
    expected: ExpectedParent<'_>,
) -> Result<(), String> {
    let ExpectedParent {
        table: expected_parent_table,
        id: expected_parent_id,
        space_id: expected_space_id,
    } = expected;
    if required_string(value, "parent_table")? != expected_parent_table
        || required_string(value, "parent_id")? != expected_parent_id
    {
        return Err(format!(
            "Notion {table} {record_id} does not belong to {expected_parent_table} {expected_parent_id}"
        ));
    }
    if required_string(value, "space_id")? != expected_space_id {
        return Err(format!(
            "Notion {table} {record_id} belongs to a different space"
        ));
    }
    if let Some(value_id) = value.get("id") {
        if value_id.as_str() != Some(record_id) {
            return Err(format!(
                "Notion {table} key {record_id} contains a different record ID"
            ));
        }
    }
    Ok(())
}

fn referenced_record<'a>(
    records: Option<&'a Map<String, Value>>,
    table: &str,
    record_id: &str,
) -> Result<&'a Value, String> {
    records
        .and_then(|records| records.get(record_id))
        .and_then(loaded_record_value)
        .ok_or_else(|| format!("missing hydrated Notion {table} record {record_id}"))
}

pub(super) fn page_discussions_from_response(
    page_id: &str,
    page_block_ids: impl IntoIterator<Item = String>,
    response: &Value,
) -> Result<Vec<CardPageDiscussion>, String> {
    let raw_blocks = super::super::record_map_table(response, "block")?;
    page_discussions(
        page_id,
        page_block_ids,
        PageCommentRecords {
            blocks: raw_blocks,
            discussions: super::super::optional_record_map_table(response, "discussion"),
            comments: super::super::optional_record_map_table(response, "comment"),
            users: super::super::optional_record_map_table(response, "notion_user"),
        },
    )
}
