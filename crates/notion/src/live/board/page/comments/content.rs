use serde_json::{Map, Value};

use super::comment_user;
use crate::model::{CardPageCommentRichSegment, NotionUserId};

pub(super) fn parse_comment_text(
    text: Option<&Value>,
    users: Option<&Map<String, Value>>,
    comment_id: &str,
) -> Result<Vec<CardPageCommentRichSegment>, String> {
    let Some(text) = text else {
        return Ok(Vec::new());
    };
    let segments = text
        .as_array()
        .ok_or_else(|| format!("Notion comment {comment_id} contains non-array rich text"))?;
    segments
        .iter()
        .map(|segment| parse_comment_text_segment(segment, users, comment_id))
        .collect()
}

fn parse_comment_text_segment(
    segment: &Value,
    users: Option<&Map<String, Value>>,
    comment_id: &str,
) -> Result<CardPageCommentRichSegment, String> {
    let Some(segment_array) = segment.as_array() else {
        return Ok(CardPageCommentRichSegment::unsupported(
            "Unsupported comment content".to_string(),
        ));
    };
    let Some(token) = segment_array.first().and_then(Value::as_str) else {
        return Ok(CardPageCommentRichSegment::unsupported(
            "Unsupported comment content".to_string(),
        ));
    };
    if token != "‣" {
        return visible_plain_token(token);
    }
    let Some(user_id) = mention_user_id(segment_array) else {
        return Ok(CardPageCommentRichSegment::unsupported(
            "Unsupported mention".to_string(),
        ));
    };
    let user_id = NotionUserId::try_from(user_id.to_string()).map_err(|error| {
        format!("Notion comment {comment_id} contains an invalid mention: {error}")
    })?;
    let user = comment_user(users, &user_id)?;
    CardPageCommentRichSegment::mention(user_id, user.name)
}

fn visible_plain_token(token: &str) -> Result<CardPageCommentRichSegment, String> {
    if token.is_empty() {
        return Ok(CardPageCommentRichSegment::unsupported(
            "Unsupported comment content".to_string(),
        ));
    }
    CardPageCommentRichSegment::plain(token.to_string())
}

fn mention_user_id(segment: &[Value]) -> Option<&str> {
    segment
        .get(1)?
        .as_array()?
        .iter()
        .filter_map(Value::as_array)
        .find(|annotation| annotation.first().and_then(Value::as_str) == Some("u"))?
        .get(1)?
        .as_str()
        .filter(|id| !id.trim().is_empty())
}

pub(super) fn parse_attachment_content(comment: &Value, comment_id: &str) -> Result<usize, String> {
    let Some(content) = comment.get("content") else {
        return Ok(0);
    };
    let content = content
        .as_array()
        .ok_or_else(|| format!("Notion comment {comment_id} contains non-array content"))?;
    for attachment_id in content {
        if attachment_id.as_str().is_none_or(|id| id.trim().is_empty()) {
            return Err(format!(
                "Notion comment {comment_id} contains an invalid attachment reference"
            ));
        }
    }
    Ok(content.len())
}
