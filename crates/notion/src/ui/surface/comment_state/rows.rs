use std::sync::Arc;

use chrono::{DateTime, Local, Utc};
use gpui::SharedString;

use super::{
    NotionCommentRow, NotionCommentSegmentRow, NotionCommentThreadRow, NotionCommentsListRow,
};
use crate::model::{
    CardPage, CardPageComment, CardPageCommentRichSegment, CardPageDiscussion,
    CardPageDiscussionAccess, NotionCommentTargetId,
};

pub(super) fn comment_thread_rows(
    page: &CardPage,
    target_id: &NotionCommentTargetId,
) -> Arc<[NotionCommentThreadRow]> {
    page.discussions
        .iter()
        .filter(|discussion| discussion.target_block_id == *target_id)
        .map(|discussion| comment_thread_row(page, discussion))
        .collect::<Vec<_>>()
        .into()
}

fn comment_thread_row(page: &CardPage, discussion: &CardPageDiscussion) -> NotionCommentThreadRow {
    NotionCommentThreadRow {
        stable_id: SharedString::from(format!(
            "notion-comment-thread-{}",
            discussion.discussion_id.as_str()
        )),
        discussion_id: discussion.discussion_id.clone(),
        comments: discussion
            .comments
            .iter()
            .map(comment_row)
            .collect::<Vec<_>>()
            .into(),
        replyable: discussion.access.is_writable() && page.comments_writable,
        read_only_reason: discussion_read_only_reason(&discussion.access),
    }
}

fn discussion_read_only_reason(access: &CardPageDiscussionAccess) -> Option<SharedString> {
    match access {
        CardPageDiscussionAccess::Writable => None,
        CardPageDiscussionAccess::ReadOnly { reason } => Some(SharedString::from(reason.clone())),
    }
}

fn comment_row(comment: &CardPageComment) -> NotionCommentRow {
    NotionCommentRow {
        stable_id: SharedString::from(format!("notion-comment-{}", comment.comment_id.as_str())),
        author: SharedString::from(comment.author.name.clone()),
        author_initial: author_initial(&comment.author.name),
        timestamp: SharedString::from(comment_time_label(comment.created_time_ms)),
        segments: comment
            .rich_text
            .iter()
            .map(comment_segment_row)
            .collect::<Vec<_>>()
            .into(),
    }
}

fn author_initial(author_name: &str) -> SharedString {
    SharedString::from(
        author_name
            .chars()
            .find(|character| !character.is_whitespace())
            .map(|character| character.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string()),
    )
}

fn comment_segment_row(segment: &CardPageCommentRichSegment) -> NotionCommentSegmentRow {
    match segment {
        CardPageCommentRichSegment::Plain { text } => {
            NotionCommentSegmentRow::Plain(SharedString::from(text.clone()))
        }
        CardPageCommentRichSegment::Mention { display_name, .. } => {
            NotionCommentSegmentRow::Mention(SharedString::from(format!("@{display_name}")))
        }
        CardPageCommentRichSegment::Unsupported { display } => {
            NotionCommentSegmentRow::Unsupported(SharedString::from(display.clone()))
        }
    }
}

pub(super) fn comment_list_rows(
    threads: &[NotionCommentThreadRow],
) -> Arc<[NotionCommentsListRow]> {
    let mut rows = Vec::new();
    for thread in threads {
        rows.push(thread_header_row(thread));
        rows.extend(
            thread
                .comments
                .iter()
                .cloned()
                .map(|comment| NotionCommentsListRow::Comment { comment }),
        );
        rows.push(thread_reply_row(thread));
    }
    rows.into()
}

fn thread_header_row(thread: &NotionCommentThreadRow) -> NotionCommentsListRow {
    NotionCommentsListRow::ThreadHeader {
        stable_id: SharedString::from(format!("{}-header", thread.stable_id)),
        read_only_reason: thread.read_only_reason.clone(),
    }
}

fn thread_reply_row(thread: &NotionCommentThreadRow) -> NotionCommentsListRow {
    NotionCommentsListRow::Reply {
        stable_id: SharedString::from(format!("{}-reply", thread.stable_id)),
        discussion_id: thread.discussion_id.clone(),
        enabled: thread.replyable,
    }
}

fn comment_time_label(timestamp_ms: u64) -> String {
    let timestamp_ms = i64::try_from(timestamp_ms).unwrap_or(i64::MAX);
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .map(|timestamp| {
            timestamp
                .with_timezone(&Local)
                .format("%b %-d, %Y %-I:%M %p")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown time".to_string())
}
