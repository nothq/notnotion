use std::{cell::RefCell, collections::HashSet, sync::Arc};

use gpui::{Entity, SharedString, UniformListScrollHandle};
use gpui_components::text_input::TextInput;

use crate::model::{
    NotionPageSharingSnapshot, NotionPublicShareRole, NotionPublicShareState,
    NotionSharePermission, NotionShareRole, NotionShareTargetId, NotionUserId,
};

#[derive(Clone)]
pub(crate) enum NotionShareDialogRow {
    PublicLink {
        state: NotionShareDialogPublicLink,
    },
    UserPermission {
        user_id: NotionUserId,
        label: SharedString,
        initial: SharedString,
        detail: SharedString,
        role: NotionShareRole,
    },
    ReadOnlyPermission {
        stable_id: SharedString,
        label: SharedString,
        detail: SharedString,
    },
    AddPeople,
    CandidateUser {
        user_id: NotionUserId,
        label: SharedString,
        initial: SharedString,
    },
    NoCandidateUsers {
        query_active: bool,
    },
}

#[derive(Clone)]
pub(crate) enum NotionShareDialogPublicLink {
    Disabled,
    Enabled(NotionPublicShareRole),
    ReadOnly(SharedString),
}

#[derive(Clone)]
pub(crate) enum NotionShareRolePickerTarget {
    User {
        user_id: NotionUserId,
        current_role: NotionShareRole,
    },
    PublicLink {
        current_role: NotionPublicShareRole,
    },
}

pub(crate) struct NotionShareDialogState {
    pub(crate) session: Arc<()>,
    pub(crate) target_id: NotionShareTargetId,
    pub(crate) loaded: bool,
    pub(crate) can_manage_sharing: bool,
    pub(crate) effective_role_label: SharedString,
    pub(crate) rows: Arc<[NotionShareDialogRow]>,
    pub(crate) user_picker_open: bool,
    pub(crate) user_picker_session: Arc<()>,
    pub(crate) user_query: String,
    pub(crate) user_query_input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) role_picker: Option<NotionShareRolePickerTarget>,
    pub(crate) mutation_in_flight: bool,
    pub(crate) scroll_handle: UniformListScrollHandle,
    snapshot: Option<Arc<NotionPageSharingSnapshot>>,
}

impl NotionShareDialogState {
    pub(crate) fn loading(target_id: NotionShareTargetId) -> Self {
        Self {
            session: Arc::new(()),
            target_id,
            loaded: false,
            can_manage_sharing: false,
            effective_role_label: SharedString::default(),
            rows: Arc::new([]),
            user_picker_open: false,
            user_picker_session: Arc::new(()),
            user_query: String::new(),
            user_query_input: RefCell::new(None),
            role_picker: None,
            mutation_in_flight: false,
            scroll_handle: UniformListScrollHandle::new(),
            snapshot: None,
        }
    }

    pub(crate) fn replace_snapshot(&mut self, snapshot: NotionPageSharingSnapshot) {
        self.loaded = true;
        self.can_manage_sharing = snapshot.can_manage_sharing;
        self.effective_role_label = SharedString::from(snapshot.effective_role_label.clone());
        self.user_picker_open = false;
        self.user_picker_session = Arc::new(());
        self.user_query.clear();
        self.user_query_input.borrow_mut().take();
        self.role_picker = None;
        self.mutation_in_flight = false;
        self.snapshot = Some(Arc::new(snapshot));
        self.rebuild_rows();
    }

    pub(crate) fn toggle_user_picker(&mut self) {
        self.user_picker_open = !self.user_picker_open;
        self.user_picker_session = Arc::new(());
        self.user_query.clear();
        self.user_query_input.borrow_mut().take();
        self.role_picker = None;
        self.rebuild_rows();
    }

    pub(crate) fn set_user_query(&mut self, query: String) {
        if !self.user_picker_open || self.user_query == query {
            return;
        }
        self.user_query = query;
        self.rebuild_rows();
    }

    pub(crate) fn open_role_picker(&mut self, target: NotionShareRolePickerTarget) {
        let rebuild_rows = self.user_picker_open;
        self.user_picker_open = false;
        self.user_picker_session = Arc::new(());
        self.user_query.clear();
        self.user_query_input.borrow_mut().take();
        self.role_picker = Some(target);
        if rebuild_rows {
            self.rebuild_rows();
        }
    }

    fn rebuild_rows(&mut self) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            self.rows = Arc::new([]);
            return;
        };
        self.rows = share_dialog_rows(snapshot, self.user_picker_open, &self.user_query);
        self.scroll_handle = UniformListScrollHandle::new();
    }
}

fn share_dialog_rows(
    snapshot: &NotionPageSharingSnapshot,
    user_picker_open: bool,
    user_query: &str,
) -> Arc<[NotionShareDialogRow]> {
    let mut rows = Vec::with_capacity(
        snapshot.permissions.len()
            + snapshot.visible_users.len() * usize::from(user_picker_open)
            + 3,
    );
    let mut permission_rows = Vec::with_capacity(snapshot.permissions.len());
    let assigned_user_ids = append_permission_rows(&mut permission_rows, &snapshot.permissions);
    rows.push(public_link_row(&snapshot.public_link));
    if user_picker_open {
        rows.push(NotionShareDialogRow::AddPeople);
        append_candidate_rows(&mut rows, snapshot, &assigned_user_ids, user_query);
        rows.extend(permission_rows);
    } else {
        rows.extend(permission_rows);
        rows.push(NotionShareDialogRow::AddPeople);
    }
    rows.into()
}

fn public_link_row(state: &NotionPublicShareState) -> NotionShareDialogRow {
    NotionShareDialogRow::PublicLink {
        state: match state {
            NotionPublicShareState::Disabled => NotionShareDialogPublicLink::Disabled,
            NotionPublicShareState::Enabled { role } => NotionShareDialogPublicLink::Enabled(*role),
            NotionPublicShareState::ReadOnly { detail } => {
                NotionShareDialogPublicLink::ReadOnly(SharedString::from(detail.clone()))
            }
        },
    }
}

fn append_permission_rows(
    rows: &mut Vec<NotionShareDialogRow>,
    permissions: &[NotionSharePermission],
) -> HashSet<NotionUserId> {
    let mut assigned_user_ids = HashSet::new();
    for permission in permissions {
        rows.push(permission_row(permission, &mut assigned_user_ids));
    }
    assigned_user_ids
}

fn permission_row(
    permission: &NotionSharePermission,
    assigned_user_ids: &mut HashSet<NotionUserId>,
) -> NotionShareDialogRow {
    match permission {
        NotionSharePermission::User(permission) => {
            assigned_user_ids.insert(permission.user.user_id.clone());
            NotionShareDialogRow::UserPermission {
                user_id: permission.user.user_id.clone(),
                label: share_user_label(&permission.user.name, permission.user.is_current_user),
                initial: share_user_initial(&permission.user.name),
                detail: SharedString::from(permission.role.label()),
                role: permission.role,
            }
        }
        NotionSharePermission::ReadOnly(permission) => {
            if let Some(user_id) = permission.principal_user_id.as_ref() {
                assigned_user_ids.insert(user_id.clone());
            }
            NotionShareDialogRow::ReadOnlyPermission {
                stable_id: SharedString::from(permission.stable_id.clone()),
                label: SharedString::from(permission.label.clone()),
                detail: SharedString::from(permission.detail.clone()),
            }
        }
    }
}

fn append_candidate_rows(
    rows: &mut Vec<NotionShareDialogRow>,
    snapshot: &NotionPageSharingSnapshot,
    assigned_user_ids: &HashSet<NotionUserId>,
    user_query: &str,
) {
    let normalized_query = user_query.trim().to_lowercase();
    let candidate_count_before = rows.len();
    rows.extend(
        snapshot
            .visible_users
            .iter()
            .filter(|user| !user.is_current_user && !assigned_user_ids.contains(&user.user_id))
            .filter(|user| {
                normalized_query.is_empty() || user.name.to_lowercase().contains(&normalized_query)
            })
            .map(|user| NotionShareDialogRow::CandidateUser {
                user_id: user.user_id.clone(),
                label: share_user_label(&user.name, user.is_current_user),
                initial: share_user_initial(&user.name),
            }),
    );
    if rows.len() == candidate_count_before {
        rows.push(NotionShareDialogRow::NoCandidateUsers {
            query_active: !normalized_query.is_empty(),
        });
    }
}

fn share_user_label(name: &str, is_current_user: bool) -> SharedString {
    if is_current_user {
        SharedString::from(format!("{name} (you)"))
    } else {
        SharedString::from(name.to_string())
    }
}

fn share_user_initial(name: &str) -> SharedString {
    SharedString::from(
        name.chars()
            .find(|character| !character.is_whitespace())
            .map(|character| character.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string()),
    )
}
