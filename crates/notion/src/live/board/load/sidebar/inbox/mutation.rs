use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::super::super::NotionPrivateApiEndpoint;
use super::wire::{UnvisitedNotificationIdsRequest, UnvisitedNotificationIdsResponse};
use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_in_space_with_session, post_private_api_with_session},
    NotionLiveError,
};
use crate::model::{MutateSidebarInboxAction, NotionSidebarInboxFilter};

const BULK_INBOX_PAGE_SIZE: u32 = 500;
const BULK_INBOX_BATCH_LIMIT: usize = 10;

pub(super) fn mutate_sidebar_inbox(
    session: &NotionDesktopSession,
    space_id: &str,
    action: MutateSidebarInboxAction,
) -> Result<(), NotionLiveError> {
    match action {
        MutateSidebarInboxAction::SetRead {
            notification_ids,
            read,
        } => submit_notification_update(
            session,
            space_id,
            &notification_ids,
            if read {
                json!({ "read": true })
            } else {
                json!({ "read": false, "visited": false })
            },
            "InboxActionsMenu.toggleNotificationReadStatus",
        ),
        MutateSidebarInboxAction::SetArchived {
            notification_ids,
            archived,
        } => submit_notification_update(
            session,
            space_id,
            &notification_ids,
            if archived {
                json!({ "visited": true, "archived_at": mutation_now_ms(), "read": true })
            } else {
                json!({ "visited": false, "archived_at": Value::Null })
            },
            if archived {
                "InboxActionsMenu.handleArchive"
            } else {
                "Activity.handleUnarchive"
            },
        ),
        MutateSidebarInboxAction::MarkAllRead => {
            mutate_all_matching_notifications(session, space_id, "unread", true)
        }
        MutateSidebarInboxAction::ArchiveAll { filter, read_only } => {
            let read_filter = archive_all_read_filter(filter, read_only)?;
            mutate_all_matching_notifications(session, space_id, read_filter, false)
        }
    }
}

fn archive_all_read_filter(
    filter: NotionSidebarInboxFilter,
    read_only: bool,
) -> Result<&'static str, String> {
    if read_only {
        return Ok("read");
    }
    match filter {
        NotionSidebarInboxFilter::All | NotionSidebarInboxFilter::WorkspaceUpdates => Ok("none"),
        NotionSidebarInboxFilter::Unread => Ok("unread"),
        NotionSidebarInboxFilter::Archived => {
            Err("archived Notion notifications cannot be archived again".to_string())
        }
    }
}

fn mutate_all_matching_notifications(
    session: &NotionDesktopSession,
    space_id: &str,
    read_filter: &'static str,
    mark_read: bool,
) -> Result<(), NotionLiveError> {
    let timestamp = mutation_now_ms();
    for _ in 0..BULK_INBOX_BATCH_LIMIT {
        let response = load_unvisited_notifications(session, space_id, timestamp, read_filter)?;
        if response.notification_ids.is_empty() {
            return Ok(());
        }
        let count = response.notification_ids.len();
        let args = if mark_read {
            json!({ "read": true })
        } else {
            json!({ "visited": true, "archived_at": mutation_now_ms(), "read": true })
        };
        submit_notification_update(
            session,
            space_id,
            &response.notification_ids,
            args,
            if mark_read {
                "notificationActions.markNotificationIdsAsRead"
            } else {
                "notificationActions.archiveNotificationIds"
            },
        )?;
        if count < BULK_INBOX_PAGE_SIZE as usize {
            return Ok(());
        }
    }
    Err(NotionLiveError::Fatal(
        "Notion bulk inbox mutation exceeded the supported batch limit".to_string(),
    ))
}

fn load_unvisited_notifications(
    session: &NotionDesktopSession,
    space_id: &str,
    timestamp: u64,
    read_filter: &'static str,
) -> Result<UnvisitedNotificationIdsResponse, NotionLiveError> {
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::GetUnvisitedNotificationIds,
        &serde_json::to_value(UnvisitedNotificationIdsRequest {
            space_id,
            timestamp,
            notification_type: "mentions",
            size: BULK_INBOX_PAGE_SIZE,
            read_filter,
        })
        .map_err(|error| format!("failed to encode Notion bulk inbox request: {error}"))?,
    )?;
    serde_json::from_value::<UnvisitedNotificationIdsResponse>(response).map_err(|error| {
        NotionLiveError::Fatal(format!("invalid Notion bulk inbox response: {error}"))
    })
}

fn submit_notification_update(
    session: &NotionDesktopSession,
    space_id: &str,
    notification_ids: &[String],
    args: Value,
    user_action: &'static str,
) -> Result<(), NotionLiveError> {
    if notification_ids.is_empty() {
        return Ok(());
    }
    let now = mutation_now_ms();
    let operations = notification_ids
        .iter()
        .map(|notification_id| {
            json!({
                "pointer": {
                    "table": "notification",
                    "id": notification_id,
                    "spaceId": space_id,
                },
                "path": [],
                "command": "update",
                "args": args.clone(),
            })
        })
        .collect::<Vec<_>>();
    post_private_api_in_space_with_session::<_, Value>(
        session,
        NotionPrivateApiEndpoint::SaveTransactionsFanout,
        space_id,
        &json!({
            "requestId": format!("notnotion-inbox-{now}"),
            "transactions": [{
                "id": format!("notnotion-inbox-notifications-{now}"),
                "spaceId": space_id,
                "debug": { "userAction": user_action },
                "operations": operations,
            }],
            "unretryable_error_behavior": "continue",
        }),
    )?;
    Ok(())
}

fn mutation_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_millis() as u64
}
