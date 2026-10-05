use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::desktop_restoration::{restored_notion_space_id, RestoredNotionSpace};
use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_with_session, NotionPrivateApiEndpoint},
    NotionLiveError, NotionSessionFailure,
};

pub(crate) struct ValidatedNotionSession {
    user_records: SpacesInitialUserRecords,
    restored_space: RestoredNotionSpace,
}

pub(crate) fn validate_notion_desktop_session(
    session: &NotionDesktopSession,
) -> Result<ValidatedNotionSession, NotionLiveError> {
    let mut response = post_private_api_with_session::<_, SpacesInitialResponse>(
        session,
        NotionPrivateApiEndpoint::GetSpacesInitial,
        &EmptyRequest {},
    )?;
    let user_records = response
        .users
        .remove(session.user_id())
        .ok_or(NotionLiveError::Session(
            NotionSessionFailure::ActiveUserMismatch,
        ))?;
    let user_records = parse_user_records(user_records)?;
    let restored_space =
        restored_notion_space_id(session.user_id()).map_err(NotionLiveError::Fatal)?;
    if restored_space.is_different_user() {
        return Err(NotionLiveError::Session(
            NotionSessionFailure::ActiveUserMismatch,
        ));
    }
    Ok(ValidatedNotionSession {
        user_records,
        restored_space,
    })
}

fn parse_user_records(
    records: serde_json::Value,
) -> Result<SpacesInitialUserRecords, NotionLiveError> {
    serde_json::from_value(records).map_err(|error| {
        NotionLiveError::Fatal(format!(
            "failed to decode the active user's Notion getSpacesInitial records: {error}"
        ))
    })
}

pub(super) struct ActiveNotionWorkspace {
    pub(super) space_id: NonEmptyNotionId,
    pub(super) space_view_id: NonEmptyNotionId,
}

pub(super) fn select_active_workspace(
    session: &NotionDesktopSession,
    validated: &ValidatedNotionSession,
) -> Result<ActiveNotionWorkspace, NotionLiveError> {
    let user_root = validated
        .user_records
        .user_roots
        .get(session.user_id())
        .ok_or_else(|| {
            NotionLiveError::Fatal(
                "Notion getSpacesInitial did not include the active user's user_root".to_string(),
            )
        })?
        .value();
    validate_user_root(session, user_root)?;
    select_workspace_pointer(user_root, &validated.restored_space)
}

fn validate_user_root(
    session: &NotionDesktopSession,
    user_root: &UserRootValue,
) -> Result<(), NotionLiveError> {
    if user_root
        .id
        .as_ref()
        .is_some_and(|id| id.0 != session.user_id())
    {
        return Err(NotionLiveError::Session(
            NotionSessionFailure::ActiveUserMismatch,
        ));
    }
    validate_space_view_pointers(&user_root.space_view_pointers)
}

fn select_workspace_pointer(
    user_root: &UserRootValue,
    restored_space: &RestoredNotionSpace,
) -> Result<ActiveNotionWorkspace, NotionLiveError> {
    let restored_space_id = restored_space.space_id()?.cloned();
    if let Some(pointer) = restored_space_id.as_ref().and_then(|space_id| {
        user_root
            .space_view_pointers
            .iter()
            .find(|pointer| pointer.space_id == *space_id)
    }) {
        return Ok(ActiveNotionWorkspace::from(pointer));
    }
    select_unambiguous_workspace(&user_root.space_view_pointers, restored_space_id)
}

fn select_unambiguous_workspace(
    pointers: &[SpaceViewPointer],
    restored_space_id: Option<NonEmptyNotionId>,
) -> Result<ActiveNotionWorkspace, NotionLiveError> {
    match pointers {
        [] => Err(NotionLiveError::Fatal(
            "the active Notion user has no workspace space_view pointers".to_string(),
        )),
        [pointer] => Ok(ActiveNotionWorkspace::from(pointer)),
        pointers => Err(NotionLiveError::Fatal(ambiguous_workspace_message(
            restored_space_id.as_ref(),
            pointers.len(),
        ))),
    }
}

fn ambiguous_workspace_message(
    restored_space_id: Option<&NonEmptyNotionId>,
    workspace_count: usize,
) -> String {
    let context = restored_space_id.map_or_else(
        || "Notion Desktop restoration state is unavailable".to_string(),
        |space_id| {
            format!(
                "Notion Desktop restored workspace {} is not present in getSpacesInitial",
                space_id.0
            )
        },
    );
    format!(
        "{context}; the active user has {workspace_count} workspaces, so notnotion cannot choose one safely"
    )
}

fn validate_space_view_pointers(pointers: &[SpaceViewPointer]) -> Result<(), NotionLiveError> {
    let mut space_ids = HashSet::with_capacity(pointers.len());
    let mut space_view_ids = HashSet::with_capacity(pointers.len());
    for pointer in pointers {
        validate_space_view_pointer(pointer, &mut space_ids, &mut space_view_ids)?;
    }
    Ok(())
}

fn validate_space_view_pointer<'a>(
    pointer: &'a SpaceViewPointer,
    space_ids: &mut HashSet<&'a str>,
    space_view_ids: &mut HashSet<&'a str>,
) -> Result<(), NotionLiveError> {
    if pointer
        .table
        .as_deref()
        .is_some_and(|table| table != "space_view")
    {
        return Err(NotionLiveError::Fatal(format!(
            "Notion user_root pointer {} has table other than space_view",
            pointer.id.0
        )));
    }
    if !space_ids.insert(pointer.space_id.0.as_str()) {
        return Err(NotionLiveError::Fatal(format!(
            "Notion user_root contains duplicate workspace pointer {}",
            pointer.space_id.0
        )));
    }
    if !space_view_ids.insert(pointer.id.0.as_str()) {
        return Err(NotionLiveError::Fatal(format!(
            "Notion user_root contains duplicate space_view pointer {}",
            pointer.id.0
        )));
    }
    Ok(())
}

impl From<&SpaceViewPointer> for ActiveNotionWorkspace {
    fn from(pointer: &SpaceViewPointer) -> Self {
        Self {
            space_id: pointer.space_id.clone(),
            space_view_id: pointer.id.clone(),
        }
    }
}

#[derive(serde::Serialize)]
struct EmptyRequest {}

#[derive(Deserialize)]
struct SpacesInitialResponse {
    users: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct SpacesInitialUserRecords {
    #[serde(default, rename = "user_root")]
    user_roots: HashMap<String, NotionRecordEntry<UserRootValue>>,
}

#[derive(Deserialize)]
struct NotionRecordEntry<T> {
    value: NotionRecordPayload<T>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NotionRecordPayload<T> {
    Wrapped { value: T },
    Direct(T),
}

impl<T> NotionRecordEntry<T> {
    fn value(&self) -> &T {
        match &self.value {
            NotionRecordPayload::Wrapped { value } | NotionRecordPayload::Direct(value) => value,
        }
    }
}

#[derive(Deserialize)]
struct UserRootValue {
    #[serde(default)]
    id: Option<NonEmptyNotionId>,
    space_view_pointers: Vec<SpaceViewPointer>,
}

#[derive(Clone, Deserialize)]
struct SpaceViewPointer {
    id: NonEmptyNotionId,
    #[serde(default)]
    table: Option<String>,
    #[serde(rename = "spaceId")]
    space_id: NonEmptyNotionId,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(super) struct NonEmptyNotionId(pub(super) String);

impl TryFrom<String> for NonEmptyNotionId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.trim().to_string();
        if value.is_empty() {
            return Err("Notion identifier must not be empty or whitespace".to_string());
        }
        Ok(Self(value))
    }
}
