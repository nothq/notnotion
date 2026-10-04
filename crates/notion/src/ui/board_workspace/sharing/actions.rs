use std::sync::Arc;

use crate::model::{
    MutateNotionPageSharingRequest, NotionPublicShareRole, NotionShareRole, NotionShareTargetId,
    NotionUserId,
};
use crate::ui::surface::{NotionShareDialogState, NotionShareRolePickerTarget};

use super::operations::ShareOperationResult;

pub(crate) enum ShareEvent {
    Open,
    Dismiss,
    Update(ShareUpdate),
    Mutate(ShareMutation),
    Completion(Box<ShareOperationResult>),
}

pub(crate) enum ShareUpdate {
    ToggleUserPicker,
    OpenRolePicker(NotionShareRolePickerTarget),
    ChangeQuery { session: Arc<()>, value: String },
    CloseUserPicker { session: Arc<()> },
}

pub(crate) enum ShareMutation {
    AddUser(NotionUserId),
    ChangeUserRole {
        user_id: NotionUserId,
        current: NotionShareRole,
        role: NotionShareRole,
    },
    RemoveUser {
        user_id: NotionUserId,
        current: NotionShareRole,
    },
    EnablePublic,
    ChangePublicRole {
        current: NotionPublicShareRole,
        role: NotionPublicShareRole,
    },
    DisablePublic(NotionPublicShareRole),
}

impl ShareMutation {
    pub(super) fn request(
        self,
        target: NotionShareTargetId,
    ) -> Result<MutateNotionPageSharingRequest, String> {
        match self {
            Self::AddUser(user) => Ok(MutateNotionPageSharingRequest::add_user(
                target,
                user,
                NotionShareRole::Reader,
            )),
            Self::ChangeUserRole {
                user_id,
                current,
                role,
            } => MutateNotionPageSharingRequest::set_user_role(target, user_id, current, role),
            Self::RemoveUser { user_id, current } => Ok(
                MutateNotionPageSharingRequest::remove_user(target, user_id, current),
            ),
            Self::EnablePublic => Ok(MutateNotionPageSharingRequest::enable_public_link(
                target,
                NotionPublicShareRole::Reader,
            )),
            Self::ChangePublicRole { current, role } => {
                MutateNotionPageSharingRequest::set_public_link_role(target, current, role)
            }
            Self::DisablePublic(current) => Ok(
                MutateNotionPageSharingRequest::disable_public_link(target, current),
            ),
        }
    }
}

impl NotionShareDialogState {
    pub(super) fn can_mutate(&self) -> bool {
        self.loaded && self.can_manage_sharing && !self.mutation_in_flight
    }

    pub(super) fn apply_view_action(&mut self, action: ShareUpdate) -> bool {
        match action {
            ShareUpdate::ToggleUserPicker => self.toggle_user_picker(),
            ShareUpdate::OpenRolePicker(target) => {
                if !self.can_mutate() {
                    return false;
                }
                self.open_role_picker(target);
            }
            ShareUpdate::ChangeQuery { session, value } => {
                if !Arc::ptr_eq(&self.user_picker_session, &session) {
                    return false;
                }
                self.set_user_query(value);
            }
            ShareUpdate::CloseUserPicker { session } => {
                if !self.user_picker_open || !Arc::ptr_eq(&self.user_picker_session, &session) {
                    return false;
                }
                self.toggle_user_picker();
            }
        }
        true
    }
}
