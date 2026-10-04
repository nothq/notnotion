mod activity;
mod mutation;
mod notification;
mod records;
mod wire;

use super::super::super::{BoardTarget, NotionPrivateApiEndpoint};
use super::context::load_sidebar_workspace_context;
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use crate::model::{LoadSidebarInboxResult, MutateSidebarInboxAction, NotionSidebarInboxFilter};
use records::InboxRecords;
use wire::{NotificationLogRequest, NotificationLogResponse};

pub(crate) fn load_sidebar_inbox(
    session: &NotionDesktopSession,
    current_board_url: &str,
    filter: NotionSidebarInboxFilter,
    size: u32,
) -> Result<LoadSidebarInboxResult, NotionLiveError> {
    let context = load_sidebar_workspace_context(session, current_board_url)?;
    let board_target = BoardTarget::parse(current_board_url)?;
    if filter == NotionSidebarInboxFilter::WorkspaceUpdates {
        return activity::load_workspace_updates(session, &context.space_id, &board_target, size);
    }
    let request = NotificationLogRequest {
        space_id: &context.space_id,
        size,
        notification_type: inbox_notification_log_type(filter),
    };
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::GetNotificationLog,
        &serde_json::to_value(request)
            .map_err(|error| format!("failed to encode Notion inbox request: {error}"))?,
    )?;
    let response = serde_json::from_value::<NotificationLogResponse>(response)
        .map_err(|error| format!("invalid Notion private API inbox response: {error}"))?;
    let has_more = response.notification_ids.len() >= size as usize;
    let records = InboxRecords::new(response.record_map);
    let items = response
        .notification_ids
        .iter()
        .map(|notification_id| {
            notification::shape_inbox_item(
                notification_id,
                &context.space_id,
                &board_target,
                &records,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadSidebarInboxResult { items, has_more })
}

pub(crate) fn mutate_sidebar_inbox(
    session: &NotionDesktopSession,
    current_board_url: &str,
    action: MutateSidebarInboxAction,
) -> Result<(), NotionLiveError> {
    let context = load_sidebar_workspace_context(session, current_board_url)?;
    mutation::mutate_sidebar_inbox(session, &context.space_id, action)
}

fn inbox_notification_log_type(filter: NotionSidebarInboxFilter) -> &'static str {
    match filter {
        NotionSidebarInboxFilter::All => "unread_and_read",
        NotionSidebarInboxFilter::Unread => "unread_only",
        NotionSidebarInboxFilter::Archived => "archived",
        NotionSidebarInboxFilter::WorkspaceUpdates => {
            panic!("workspace updates use the Notion activity-log endpoint")
        }
    }
}
