use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use super::{permission_is_inherited, LoadedSharePage};
use crate::model::{
    NotionPageSharingSnapshot, NotionPublicShareRole, NotionPublicShareState,
    NotionReadOnlySharePermission, NotionSharePermission, NotionShareRole, NotionShareTargetId,
    NotionShareUserPermission, NotionUserId, NotionWorkspaceUser,
};

pub(super) fn sharing_snapshot(
    target_id: NotionShareTargetId,
    page: LoadedSharePage,
    visible_users: Vec<NotionWorkspaceUser>,
) -> Result<NotionPageSharingSnapshot, String> {
    let projected = SharingPermissionProjection::new(&visible_users).project(&page.permissions)?;
    Ok(NotionPageSharingSnapshot {
        target_id,
        can_manage_sharing: page.effective_role == "editor",
        effective_role_label: effective_role_label(&page.effective_role),
        permissions: projected.permissions,
        public_link: projected.public_link,
        visible_users,
    })
}

struct ProjectedSharingPermissions {
    permissions: Vec<NotionSharePermission>,
    public_link: NotionPublicShareState,
}

struct SharingPermissionProjection<'a> {
    users_by_id: HashMap<&'a str, &'a NotionWorkspaceUser>,
    permissions: Vec<NotionSharePermission>,
    seen_user_ids: HashSet<NotionUserId>,
    public_link: NotionPublicShareState,
    found_public_link: bool,
}

impl<'a> SharingPermissionProjection<'a> {
    fn new(visible_users: &'a [NotionWorkspaceUser]) -> Self {
        Self {
            users_by_id: visible_users
                .iter()
                .map(|user| (user.user_id.as_str(), user))
                .collect(),
            permissions: Vec::new(),
            seen_user_ids: HashSet::new(),
            public_link: NotionPublicShareState::Disabled,
            found_public_link: false,
        }
    }

    fn project(mut self, raw_permissions: &[Value]) -> Result<ProjectedSharingPermissions, String> {
        for (index, permission) in raw_permissions.iter().enumerate() {
            self.project_permission(index, permission)?;
        }
        Ok(ProjectedSharingPermissions {
            permissions: self.permissions,
            public_link: self.public_link,
        })
    }

    fn project_permission(&mut self, index: usize, permission: &Value) -> Result<(), String> {
        let Some(permission) = permission.as_object() else {
            self.permissions.push(read_only_permission(
                format!("malformed-{index}"),
                "Unsupported permission",
                "Malformed permission record",
                None,
            ));
            return Ok(());
        };
        match permission.get("type").and_then(Value::as_str) {
            Some("user_permission") => self.project_user_permission(index, permission),
            Some("public_permission") if is_public_share_link(permission) => {
                self.project_public_link(permission)
            }
            permission_type => {
                self.project_read_only_permission(index, permission_type, permission);
                Ok(())
            }
        }
    }

    fn project_user_permission(
        &mut self,
        index: usize,
        permission: &Map<String, Value>,
    ) -> Result<(), String> {
        let inherited = permission_is_inherited(permission);
        let parsed_user_id = parsed_user_id(permission);
        self.register_user_id(parsed_user_id.as_ref())?;
        if !inherited && permission.get("role").and_then(Value::as_str) == Some("none") {
            return Ok(());
        }
        let role = permission
            .get("role")
            .and_then(Value::as_str)
            .and_then(|role| NotionShareRole::parse_notion(role).ok());
        let hydrated_user = parsed_user_id
            .as_ref()
            .and_then(|user_id| self.users_by_id.get(user_id.as_str()))
            .copied();
        self.permissions.push(projected_user_permission(
            index,
            permission,
            ParsedUserPermission {
                user_id: parsed_user_id,
                role,
                inherited,
                hydrated_user,
            },
        ));
        Ok(())
    }

    fn register_user_id(&mut self, user_id: Option<&NotionUserId>) -> Result<(), String> {
        let Some(user_id) = user_id else {
            return Ok(());
        };
        if self.seen_user_ids.insert(user_id.clone()) {
            return Ok(());
        }
        Err(format!(
            "Notion sharing target contains duplicate permission for user {}",
            user_id.as_str()
        ))
    }

    fn project_public_link(&mut self, permission: &Map<String, Value>) -> Result<(), String> {
        if self.found_public_link {
            return Err(
                "Notion sharing target contains duplicate public-link permissions".to_string(),
            );
        }
        self.found_public_link = true;
        self.public_link = public_share_state(permission);
        Ok(())
    }

    fn project_read_only_permission(
        &mut self,
        index: usize,
        permission_type: Option<&str>,
        permission: &Map<String, Value>,
    ) {
        let permission_type = permission_type.unwrap_or("unknown");
        self.permissions.push(read_only_permission(
            format!("{permission_type}-{index}"),
            permission_type_label(permission_type),
            read_only_permission_detail(permission),
            None,
        ));
    }
}

fn is_public_share_link(permission: &Map<String, Value>) -> bool {
    permission
        .get("is_public_share_link")
        .and_then(Value::as_bool)
        == Some(true)
}

fn parsed_user_id(permission: &Map<String, Value>) -> Option<NotionUserId> {
    permission
        .get("user_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .map(NotionUserId::try_from)
        .transpose()
        .ok()
        .flatten()
}

struct ParsedUserPermission<'a> {
    user_id: Option<NotionUserId>,
    role: Option<NotionShareRole>,
    inherited: bool,
    hydrated_user: Option<&'a NotionWorkspaceUser>,
}

fn projected_user_permission(
    index: usize,
    permission: &Map<String, Value>,
    parsed: ParsedUserPermission<'_>,
) -> NotionSharePermission {
    let ParsedUserPermission {
        user_id,
        role,
        inherited,
        hydrated_user,
    } = parsed;
    match (user_id, role, inherited, hydrated_user) {
        (Some(_), Some(role), false, Some(user)) => {
            NotionSharePermission::User(NotionShareUserPermission {
                user: user.clone(),
                role,
            })
        }
        (user_id, _, _, hydrated_user) => read_only_permission(
            format!("user-{index}"),
            hydrated_user
                .map(|user| user.name.clone())
                .unwrap_or_else(|| "Unavailable user".to_string()),
            read_only_user_permission_detail(permission, inherited),
            user_id,
        ),
    }
}

fn public_share_state(permission: &Map<String, Value>) -> NotionPublicShareState {
    let role = permission.get("role").and_then(Value::as_str);
    if permission_is_inherited(permission) {
        return NotionPublicShareState::ReadOnly {
            detail: "Inherited public link".to_string(),
        };
    }
    if role == Some("none") {
        return NotionPublicShareState::Disabled;
    }
    match role.and_then(|role| NotionPublicShareRole::parse_notion(role).ok()) {
        Some(role) => NotionPublicShareState::Enabled { role },
        None => NotionPublicShareState::ReadOnly {
            detail: role
                .map(|role| format!("Unsupported access: {role}"))
                .unwrap_or_else(|| "Missing public-link role".to_string()),
        },
    }
}

fn read_only_permission(
    stable_id: String,
    label: impl Into<String>,
    detail: impl Into<String>,
    principal_user_id: Option<NotionUserId>,
) -> NotionSharePermission {
    NotionSharePermission::ReadOnly(NotionReadOnlySharePermission {
        stable_id,
        label: label.into(),
        detail: detail.into(),
        principal_user_id,
    })
}

fn read_only_user_permission_detail(permission: &Map<String, Value>, inherited: bool) -> String {
    if inherited {
        return "Inherited permission".to_string();
    }
    permission
        .get("role")
        .and_then(Value::as_str)
        .map(|role| format!("Unsupported access: {role}"))
        .unwrap_or_else(|| "Missing permission role".to_string())
}

fn read_only_permission_detail(permission: &Map<String, Value>) -> String {
    if permission_is_inherited(permission) {
        return "Inherited permission".to_string();
    }
    permission
        .get("role")
        .and_then(Value::as_str)
        .map(|role| format!("Read-only access: {role}"))
        .unwrap_or_else(|| "Read-only permission".to_string())
}

fn permission_type_label(permission_type: &str) -> String {
    match permission_type {
        "group_permission" => "Workspace group".to_string(),
        "space_permission" => "Workspace access".to_string(),
        "team_permission" => "Teamspace access".to_string(),
        "public_permission" => "Public access".to_string(),
        _ => "Unsupported permission".to_string(),
    }
}

fn effective_role_label(role: &str) -> String {
    match role {
        "none" => "No access".to_string(),
        "reader" => "Can view".to_string(),
        "comment_only" => "Can comment".to_string(),
        "content_only_editor" => "Can edit content".to_string(),
        "read_and_write" => "Can edit".to_string(),
        "membership_admin" => "Membership admin".to_string(),
        "editor" => "Full access".to_string(),
        _ => format!("Unsupported access: {role}"),
    }
}
