use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NotionShareTargetId(String);

impl NotionShareTargetId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NotionShareTargetId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("Notion share target ID must not be empty or whitespace");
        }
        Ok(Self(value))
    }
}

impl std::str::FromStr for NotionShareTargetId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl From<NotionShareTargetId> for String {
    fn from(value: NotionShareTargetId) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NotionUserId(String);

impl NotionUserId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NotionUserId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err("Notion user ID must not be empty or whitespace");
        }
        Ok(Self(value))
    }
}

impl std::str::FromStr for NotionUserId {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl From<NotionUserId> for String {
    fn from(value: NotionUserId) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotionShareRole {
    Reader,
    Commenter,
    Editor,
    FullAccess,
}

impl NotionShareRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reader => "Can view",
            Self::Commenter => "Can comment",
            Self::Editor => "Can edit",
            Self::FullAccess => "Full access",
        }
    }

    pub(crate) const fn as_notion_str(self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Commenter => "comment_only",
            Self::Editor => "read_and_write",
            Self::FullAccess => "editor",
        }
    }

    pub(crate) fn parse_notion(value: &str) -> Result<Self, &'static str> {
        match value {
            "reader" => Ok(Self::Reader),
            "comment_only" => Ok(Self::Commenter),
            "read_and_write" => Ok(Self::Editor),
            "editor" => Ok(Self::FullAccess),
            _ => Err("unsupported Notion share role"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotionPublicShareRole {
    Reader,
    Commenter,
    Editor,
}

impl NotionPublicShareRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reader => "Can view",
            Self::Commenter => "Can comment",
            Self::Editor => "Can edit",
        }
    }

    pub(crate) const fn as_notion_str(self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Commenter => "comment_only",
            Self::Editor => "read_and_write",
        }
    }

    pub(crate) fn parse_notion(value: &str) -> Result<Self, &'static str> {
        match value {
            "reader" => Ok(Self::Reader),
            "comment_only" => Ok(Self::Commenter),
            "read_and_write" => Ok(Self::Editor),
            _ => Err("unsupported Notion public-link role"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotionWorkspaceUser {
    pub user_id: NotionUserId,
    pub name: String,
    pub profile_photo: Option<String>,
    pub is_current_user: bool,
}

#[derive(Clone, Debug)]
pub struct NotionShareUserPermission {
    pub user: NotionWorkspaceUser,
    pub role: NotionShareRole,
}

#[derive(Clone, Debug)]
pub struct NotionReadOnlySharePermission {
    pub stable_id: String,
    pub label: String,
    pub detail: String,
    pub principal_user_id: Option<NotionUserId>,
}

#[derive(Clone, Debug)]
pub enum NotionSharePermission {
    User(NotionShareUserPermission),
    ReadOnly(NotionReadOnlySharePermission),
}

#[derive(Clone, Debug)]
pub enum NotionPublicShareState {
    Disabled,
    Enabled { role: NotionPublicShareRole },
    ReadOnly { detail: String },
}

#[derive(Clone, Debug)]
pub struct NotionPageSharingSnapshot {
    pub target_id: NotionShareTargetId,
    pub can_manage_sharing: bool,
    pub effective_role_label: String,
    pub permissions: Vec<NotionSharePermission>,
    pub public_link: NotionPublicShareState,
    pub visible_users: Vec<NotionWorkspaceUser>,
}

#[derive(Clone, Debug)]
pub struct LoadNotionPageSharingRequest {
    target_id: NotionShareTargetId,
}

impl LoadNotionPageSharingRequest {
    pub fn new(target_id: NotionShareTargetId) -> Self {
        Self { target_id }
    }

    pub(crate) fn into_target_id(self) -> NotionShareTargetId {
        self.target_id
    }
}

#[derive(Clone, Debug)]
pub struct MutateNotionPageSharingRequest {
    target_id: NotionShareTargetId,
    mutation: NotionPageSharingMutation,
}

impl MutateNotionPageSharingRequest {
    pub(crate) fn target_id(&self) -> &NotionShareTargetId {
        &self.target_id
    }

    pub fn add_user(
        target_id: NotionShareTargetId,
        user_id: NotionUserId,
        role: NotionShareRole,
    ) -> Self {
        Self {
            target_id,
            mutation: NotionPageSharingMutation::SetUserRole {
                user_id,
                expected_role: None,
                role,
            },
        }
    }

    pub fn set_user_role(
        target_id: NotionShareTargetId,
        user_id: NotionUserId,
        expected_role: NotionShareRole,
        role: NotionShareRole,
    ) -> Result<Self, String> {
        if expected_role == role {
            return Err("Notion share role update must change the role".to_string());
        }
        Ok(Self {
            target_id,
            mutation: NotionPageSharingMutation::SetUserRole {
                user_id,
                expected_role: Some(expected_role),
                role,
            },
        })
    }

    pub fn remove_user(
        target_id: NotionShareTargetId,
        user_id: NotionUserId,
        expected_role: NotionShareRole,
    ) -> Self {
        Self {
            target_id,
            mutation: NotionPageSharingMutation::RemoveUser {
                user_id,
                expected_role,
            },
        }
    }

    pub fn enable_public_link(target_id: NotionShareTargetId, role: NotionPublicShareRole) -> Self {
        Self {
            target_id,
            mutation: NotionPageSharingMutation::SetPublicLink {
                expected_role: None,
                role: Some(role),
            },
        }
    }

    pub fn set_public_link_role(
        target_id: NotionShareTargetId,
        expected_role: NotionPublicShareRole,
        role: NotionPublicShareRole,
    ) -> Result<Self, String> {
        if expected_role == role {
            return Err("Notion public-link update must change the role".to_string());
        }
        Ok(Self {
            target_id,
            mutation: NotionPageSharingMutation::SetPublicLink {
                expected_role: Some(expected_role),
                role: Some(role),
            },
        })
    }

    pub fn disable_public_link(
        target_id: NotionShareTargetId,
        expected_role: NotionPublicShareRole,
    ) -> Self {
        Self {
            target_id,
            mutation: NotionPageSharingMutation::SetPublicLink {
                expected_role: Some(expected_role),
                role: None,
            },
        }
    }

    pub(crate) fn into_parts(self) -> (NotionShareTargetId, NotionPageSharingMutation) {
        (self.target_id, self.mutation)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum NotionPageSharingMutation {
    SetUserRole {
        user_id: NotionUserId,
        expected_role: Option<NotionShareRole>,
        role: NotionShareRole,
    },
    RemoveUser {
        user_id: NotionUserId,
        expected_role: NotionShareRole,
    },
    SetPublicLink {
        expected_role: Option<NotionPublicShareRole>,
        role: Option<NotionPublicShareRole>,
    },
}
