use serde_json::Value;

use super::super::super::super::{
    plain_text_from_property_value, required_string, title_property, BoardTarget,
};
use super::super::records::required_bool;
use super::records::{
    event_time_ms, non_empty_text, optional_string, optional_timestamp, resolve_actor,
    validate_space, InboxRecords,
};
use crate::model::PageShellInboxItem;

pub(super) fn shape_inbox_item(
    notification_id: &str,
    space_id: &str,
    board_target: &BoardTarget,
    records: &InboxRecords,
) -> Result<PageShellInboxItem, String> {
    let notification = records.required_record("notification", notification_id)?;
    validate_space(notification, space_id, "notification")?;
    let (activity_id, activity) = resolve_activity(notification, space_id, records)?;
    let notification_type = required_string(notification, "type")?.to_string();
    let target_block_id = target_block_id(notification, activity)?;
    let block = target_block_id
        .map(|block_id| records.record("block", block_id))
        .transpose()?
        .flatten();
    let comment = resolve_comment(notification, activity, records)?;
    let actor = resolve_actor(activity, comment, records)?;
    let title = block
        .map(title_property)
        .transpose()?
        .flatten()
        .unwrap_or_else(|| {
            if target_block_id.is_some() {
                "Untitled".to_string()
            } else {
                "Notion notification".to_string()
            }
        });
    let body = comment
        .and_then(|comment| comment.get("text"))
        .map(|text| {
            plain_text_from_property_value(
                text,
                records.table("block")?,
                records.table("notion_user")?,
            )
        })
        .transpose()?
        .and_then(non_empty_text);
    let event_time_ms = event_time_ms(notification, activity, comment)?;
    let read = required_bool(notification, "read")?;
    let archived = required_bool(notification, "visited")?;
    let target_board_url = target_block_id.map(|block_id| board_target.child_url(block_id));

    Ok(PageShellInboxItem {
        notification_id: notification_id.to_string(),
        activity_id: activity_id.map(str::to_string),
        notification_type,
        actor,
        title,
        body,
        event_time_ms,
        read,
        archived,
        target_board_url,
    })
}

/// A notification's activity ID and its populated activity record.
type ResolvedActivity<'a> = (Option<&'a str>, Option<&'a Value>);

fn resolve_activity<'a>(
    notification: &'a Value,
    space_id: &str,
    records: &'a InboxRecords,
) -> Result<ResolvedActivity<'a>, String> {
    if let Some(activity_id) = optional_string(notification, "activity_id")? {
        return Ok((
            Some(activity_id),
            populated_activity(activity_id, space_id, records)?,
        ));
    }
    let Some(child_ids) = notification.get("child_notification_ids") else {
        return Ok((None, None));
    };
    let child_ids = child_ids
        .as_array()
        .ok_or_else(|| "invalid Notion inbox child_notification_ids".to_string())?;
    let mut newest = None;
    for child_id in child_ids {
        let child_id = child_id
            .as_str()
            .ok_or_else(|| "invalid Notion inbox child notification id".to_string())?;
        let child = records.required_record("notification", child_id)?;
        validate_space(child, space_id, "child notification")?;
        let Some(activity_id) = optional_string(child, "activity_id")? else {
            continue;
        };
        let Some(activity) = populated_activity(activity_id, space_id, records)? else {
            continue;
        };
        let sort_time = optional_timestamp(activity, "end_time")?
            .or(optional_timestamp(activity, "start_time")?)
            .unwrap_or_default();
        if newest
            .as_ref()
            .is_none_or(|(newest_time, _, _)| sort_time >= *newest_time)
        {
            newest = Some((sort_time, activity_id, activity));
        }
    }
    Ok(newest
        .map(|(_, activity_id, activity)| (Some(activity_id), Some(activity)))
        .unwrap_or((None, None)))
}

fn populated_activity<'a>(
    activity_id: &str,
    space_id: &str,
    records: &'a InboxRecords,
) -> Result<Option<&'a Value>, String> {
    let Some(activity) = records.record("activity", activity_id)? else {
        return Ok(None);
    };
    if activity.get("space_id").is_none() {
        return Ok(None);
    }
    validate_space(activity, space_id, "activity")?;
    Ok(Some(activity))
}

fn target_block_id<'a>(
    notification: &'a Value,
    activity: Option<&'a Value>,
) -> Result<Option<&'a str>, String> {
    if let Some(block_id) = optional_string(notification, "navigable_block_id")? {
        return Ok(Some(block_id));
    }
    let Some(activity) = activity else {
        return Ok(None);
    };
    for key in [
        "navigable_block_id",
        "top_level_block_id",
        "collection_row_id",
    ] {
        if let Some(block_id) = optional_string(activity, key)? {
            return Ok(Some(block_id));
        }
    }
    Ok(None)
}

fn resolve_comment<'a>(
    notification: &Value,
    activity: Option<&Value>,
    records: &'a InboxRecords,
) -> Result<Option<&'a Value>, String> {
    if let Some(activity) = activity {
        if let Some(comment_id) = newest_comment_id(activity)? {
            if let Some(comment) = records.record("comment", &comment_id)? {
                return Ok(Some(comment));
            }
        }
    }
    let discussion_id = optional_string(notification, "discussion_id")?.or(activity
        .map(|activity| optional_string(activity, "discussion_id"))
        .transpose()?
        .flatten());
    let Some(discussion_id) = discussion_id else {
        return Ok(None);
    };
    let Some(discussion) = records.record("discussion", discussion_id)? else {
        return Ok(None);
    };
    let Some(comment_id) = discussion
        .get("comments")
        .and_then(Value::as_array)
        .and_then(|comments| comments.last())
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    records.record("comment", comment_id)
}

fn newest_comment_id(activity: &Value) -> Result<Option<String>, String> {
    let Some(edits) = activity.get("edits") else {
        return Ok(None);
    };
    let edits = edits
        .as_array()
        .ok_or_else(|| "invalid Notion inbox activity edits".to_string())?;
    let mut latest = None;
    for edit in edits {
        let Some(comment_id) = optional_string(edit, "comment_id")? else {
            continue;
        };
        let timestamp = optional_timestamp(edit, "timestamp")?.unwrap_or_default();
        if latest
            .as_ref()
            .is_none_or(|(latest_timestamp, _)| timestamp >= *latest_timestamp)
        {
            latest = Some((timestamp, comment_id.to_string()));
        }
    }
    Ok(latest.map(|(_, comment_id)| comment_id))
}
