use serde_json::{json, Map, Value};

use super::{
    load_complete_page_response_with_session, load_visible_workspace_users, record_map_table,
    reject_transaction_errors, required_string, unwrap_record_value, LiveWorkspaceContext,
    NotionPrivateApiEndpoint,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_in_space_with_session,
    NotionLiveError,
};
use crate::model::{
    LoadNotionPageSharingRequest, MutateNotionPageSharingRequest, NotionPageSharingSnapshot,
    NotionShareTargetId,
};

mod mutation;
mod snapshot;

use mutation::prepare_permission_item;
use snapshot::sharing_snapshot;

struct LoadedSharePage {
    space_id: String,
    effective_role: String,
    permissions: Vec<Value>,
}

pub(crate) fn load_page_sharing(
    session: &NotionDesktopSession,
    context: &LiveWorkspaceContext,
    request: LoadNotionPageSharingRequest,
) -> Result<NotionPageSharingSnapshot, NotionLiveError> {
    let target_id = request.into_target_id();
    let page = load_share_page(session, context, &target_id)?;
    let visible_users = load_visible_workspace_users(session, context)?;
    sharing_snapshot(target_id, page, visible_users).map_err(NotionLiveError::Fatal)
}

pub(crate) fn mutate_page_sharing(
    session: &NotionDesktopSession,
    context: &LiveWorkspaceContext,
    request: MutateNotionPageSharingRequest,
) -> Result<NotionShareTargetId, NotionLiveError> {
    let (target_id, mutation) = request.into_parts();
    let page = load_share_page(session, context, &target_id)?;
    if page.effective_role != "editor" {
        return Err(NotionLiveError::Fatal(
            "Notion sharing changes require Full access to the target page".to_string(),
        ));
    }
    let visible_users = load_visible_workspace_users(session, context)?;
    let permission_item = prepare_permission_item(&page.permissions, &visible_users, mutation)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .expect("system clock must be after the Unix epoch");
    let response = post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::SaveTransactionsFanout,
        &page.space_id,
        &json!({
            "requestId": uuid::Uuid::new_v4().to_string(),
            "transactions": [{
                "id": uuid::Uuid::new_v4().to_string(),
                "spaceId": page.space_id,
                "debug": {
                    "userAction": "ShareMenu.updatePermission",
                    "navParentClientGateState": "on",
                    "clientCommitTimeMs": now,
                },
                "operations": [{
                    "pointer": {
                        "id": target_id.as_str(),
                        "table": "block",
                        "spaceId": page.space_id,
                    },
                    "path": ["permissions"],
                    "command": "setPermissionItem",
                    "args": permission_item,
                }],
            }],
        }),
    )?;
    reject_transaction_errors(&response, "sharing").map_err(NotionLiveError::Fatal)?;
    Ok(target_id)
}

fn load_share_page(
    session: &NotionDesktopSession,
    context: &LiveWorkspaceContext,
    target_id: &NotionShareTargetId,
) -> Result<LoadedSharePage, NotionLiveError> {
    let cache = context.workspace_cache()?;
    let response = load_complete_page_response_with_session(session, target_id.as_str())?;
    let value = response.value()?;
    let blocks = record_map_table(&value, "block")?;
    let entry = blocks.get(target_id.as_str()).ok_or_else(|| {
        format!(
            "Notion sharing response omitted target block {}",
            target_id.as_str()
        )
    })?;
    let root = unwrap_record_value(entry).ok_or_else(|| {
        format!(
            "Notion sharing response did not hydrate target block {}",
            target_id.as_str()
        )
    })?;
    let hydrated_id = required_string(root, "id")?;
    if hydrated_id != target_id.as_str() {
        return Err(NotionLiveError::Fatal(format!(
            "Notion sharing response returned block {hydrated_id} for target {}",
            target_id.as_str()
        )));
    }
    let space_id = required_string(root, "space_id")?;
    if space_id != cache.space_id {
        return Err(NotionLiveError::Fatal(format!(
            "Notion sharing target {} belongs to space {space_id}, expected {}",
            target_id.as_str(),
            cache.space_id
        )));
    }
    let effective_role = record_role(entry).ok_or_else(|| {
        format!(
            "Notion sharing response omitted the effective role for target {}",
            target_id.as_str()
        )
    })?;
    let permissions = match root.get("permissions") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(permissions)) => permissions.clone(),
        Some(_) => {
            return Err(NotionLiveError::Fatal(format!(
                "Notion sharing target {} contains non-array permissions",
                target_id.as_str()
            )));
        }
    };
    Ok(LoadedSharePage {
        space_id: space_id.to_string(),
        effective_role: effective_role.to_string(),
        permissions,
    })
}

fn record_role(entry: &Value) -> Option<&str> {
    entry.get("role").and_then(Value::as_str).or_else(|| {
        entry
            .get("value")
            .and_then(|wrapper| wrapper.get("role"))
            .and_then(Value::as_str)
    })
}

fn permission_is_inherited(permission: &Map<String, Value>) -> bool {
    permission
        .get("is_inherited")
        .is_some_and(|value| value.as_bool() != Some(false))
        || permission
            .get("inherited_from")
            .is_some_and(|value| !value.is_null())
}
