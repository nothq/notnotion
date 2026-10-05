use serde_json::{Map, Value};

use super::permission_is_inherited;
use crate::live::NotionLiveError;
use crate::model::{
    NotionPageSharingMutation, NotionPublicShareRole, NotionShareRole, NotionUserId,
    NotionWorkspaceUser,
};

pub(super) fn prepare_permission_item(
    permissions: &[Value],
    visible_users: &[NotionWorkspaceUser],
    mutation: NotionPageSharingMutation,
) -> Result<Value, NotionLiveError> {
    match mutation {
        NotionPageSharingMutation::SetUserRole {
            user_id,
            expected_role,
            role,
        } => prepare_user_permission_item(
            permissions,
            visible_users,
            &user_id,
            expected_role,
            Some(role),
        ),
        NotionPageSharingMutation::RemoveUser {
            user_id,
            expected_role,
        } => prepare_user_permission_item(
            permissions,
            visible_users,
            &user_id,
            Some(expected_role),
            None,
        ),
        NotionPageSharingMutation::SetPublicLink {
            expected_role,
            role,
        } => prepare_public_permission_item(permissions, expected_role, role),
    }
    .map_err(NotionLiveError::Fatal)
}

fn prepare_user_permission_item(
    permissions: &[Value],
    visible_users: &[NotionWorkspaceUser],
    user_id: &NotionUserId,
    expected_role: Option<NotionShareRole>,
    next_role: Option<NotionShareRole>,
) -> Result<Value, String> {
    let existing = unique_user_permission(permissions, user_id)?;
    if existing.is_some_and(permission_is_inherited) {
        return Err("inherited Notion permissions cannot be changed in notnotion".to_string());
    }
    let active_existing = active_permission(existing);
    validate_expected_user_permission(user_id, expected_role, active_existing)?;
    if expected_role.is_none() {
        validate_addable_user(visible_users, user_id)?;
    }
    validate_user_role_transition(
        permissions,
        user_id,
        expected_role,
        next_role,
        active_existing,
    )?;
    Ok(updated_user_permission_item(existing, user_id, next_role))
}

type PermissionItem = Map<String, Value>;

fn unique_user_permission<'a>(
    permissions: &'a [Value],
    user_id: &NotionUserId,
) -> Result<Option<&'a PermissionItem>, String> {
    let matches = permissions
        .iter()
        .filter_map(Value::as_object)
        .filter(|permission| {
            permission.get("type").and_then(Value::as_str) == Some("user_permission")
                && permission.get("user_id").and_then(Value::as_str) == Some(user_id.as_str())
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!(
            "Notion sharing target contains duplicate permission items for user {}",
            user_id.as_str()
        ));
    }
    Ok(matches.first().copied())
}

fn active_permission(permission: Option<&Map<String, Value>>) -> Option<&Map<String, Value>> {
    permission.filter(|permission| permission.get("role").and_then(Value::as_str) != Some("none"))
}

fn validate_expected_user_permission(
    user_id: &NotionUserId,
    expected_role: Option<NotionShareRole>,
    active_existing: Option<&Map<String, Value>>,
) -> Result<(), String> {
    match (expected_role, active_existing) {
        (None, Some(_)) => Err(format!(
            "Notion user {} already has an explicit permission",
            user_id.as_str()
        )),
        (Some(_), None) => Err(format!(
            "Notion user {} no longer has the expected permission",
            user_id.as_str()
        )),
        _ => Ok(()),
    }
}

fn validate_addable_user(
    visible_users: &[NotionWorkspaceUser],
    user_id: &NotionUserId,
) -> Result<(), String> {
    if !visible_users.iter().any(|user| user.user_id == *user_id) {
        return Err(format!(
            "Notion user {} is not in the visible workspace-user set",
            user_id.as_str()
        ));
    }
    if visible_users
        .iter()
        .any(|user| user.user_id == *user_id && user.is_current_user)
    {
        return Err(
            "cannot add an implicit current-user permission with a different role".to_string(),
        );
    }
    Ok(())
}

fn validate_user_role_transition(
    permissions: &[Value],
    user_id: &NotionUserId,
    expected_role: Option<NotionShareRole>,
    next_role: Option<NotionShareRole>,
    active_existing: Option<&Map<String, Value>>,
) -> Result<(), String> {
    let Some(permission) = active_existing else {
        return Ok(());
    };
    let current_role = permission
        .get("role")
        .and_then(Value::as_str)
        .and_then(|role| NotionShareRole::parse_notion(role).ok())
        .ok_or_else(|| "unsupported Notion permission roles are read-only".to_string())?;
    if Some(current_role) != expected_role {
        return Err(format!(
            "Notion user {} permission changed before this update",
            user_id.as_str()
        ));
    }
    if current_role == NotionShareRole::FullAccess
        && next_role != Some(NotionShareRole::FullAccess)
        && known_full_access_principal_count(permissions) <= 1
    {
        return Err("cannot remove or demote the last known Full access principal".to_string());
    }
    Ok(())
}

fn updated_user_permission_item(
    existing: Option<&Map<String, Value>>,
    user_id: &NotionUserId,
    next_role: Option<NotionShareRole>,
) -> Value {
    let mut item = existing.cloned().unwrap_or_else(|| {
        Map::from_iter([
            (
                "type".to_string(),
                Value::String("user_permission".to_string()),
            ),
            (
                "user_id".to_string(),
                Value::String(user_id.as_str().to_string()),
            ),
        ])
    });
    item.insert(
        "role".to_string(),
        Value::String(
            next_role
                .map(NotionShareRole::as_notion_str)
                .unwrap_or("none")
                .to_string(),
        ),
    );
    Value::Object(item)
}

fn prepare_public_permission_item(
    permissions: &[Value],
    expected_role: Option<NotionPublicShareRole>,
    next_role: Option<NotionPublicShareRole>,
) -> Result<Value, String> {
    let existing = unique_public_permission(permissions)?;
    if existing.is_some_and(permission_is_inherited) {
        return Err("inherited Notion public links cannot be changed in notnotion".to_string());
    }
    let active_existing = active_permission(existing);
    validate_expected_public_permission(expected_role, active_existing)?;
    validate_public_role_transition(expected_role, active_existing)?;
    Ok(updated_public_permission_item(existing, next_role))
}

fn unique_public_permission(permissions: &[Value]) -> Result<Option<&PermissionItem>, String> {
    let matches = permissions
        .iter()
        .filter_map(Value::as_object)
        .filter(|permission| {
            permission.get("type").and_then(Value::as_str) == Some("public_permission")
                && permission
                    .get("is_public_share_link")
                    .and_then(Value::as_bool)
                    == Some(true)
        })
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err("Notion sharing target contains duplicate public-link permissions".to_string());
    }
    Ok(matches.first().copied())
}

fn validate_expected_public_permission(
    expected_role: Option<NotionPublicShareRole>,
    active_existing: Option<&Map<String, Value>>,
) -> Result<(), String> {
    match (expected_role, active_existing) {
        (None, Some(_)) => Err("Notion public link is already enabled".to_string()),
        (Some(_), None) => Err("Notion public link no longer has the expected access".to_string()),
        _ => Ok(()),
    }
}

fn validate_public_role_transition(
    expected_role: Option<NotionPublicShareRole>,
    active_existing: Option<&Map<String, Value>>,
) -> Result<(), String> {
    let Some(permission) = active_existing else {
        return Ok(());
    };
    let current_role = permission
        .get("role")
        .and_then(Value::as_str)
        .and_then(|role| NotionPublicShareRole::parse_notion(role).ok())
        .ok_or_else(|| "unsupported Notion public-link roles are read-only".to_string())?;
    if Some(current_role) != expected_role {
        return Err("Notion public-link access changed before this update".to_string());
    }
    Ok(())
}

fn updated_public_permission_item(
    existing: Option<&Map<String, Value>>,
    next_role: Option<NotionPublicShareRole>,
) -> Value {
    let mut item = existing.cloned().unwrap_or_else(|| {
        Map::from_iter([
            (
                "type".to_string(),
                Value::String("public_permission".to_string()),
            ),
            ("is_public_share_link".to_string(), Value::Bool(true)),
        ])
    });
    item.insert(
        "role".to_string(),
        Value::String(
            next_role
                .map(NotionPublicShareRole::as_notion_str)
                .unwrap_or("none")
                .to_string(),
        ),
    );
    Value::Object(item)
}

fn known_full_access_principal_count(permissions: &[Value]) -> usize {
    permissions
        .iter()
        .filter_map(Value::as_object)
        .filter(|permission| permission.get("role").and_then(Value::as_str) == Some("editor"))
        .count()
}
