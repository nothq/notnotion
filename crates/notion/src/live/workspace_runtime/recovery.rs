use crate::live::{NotionLiveError, NotionSessionFailure};
use crate::model::{NotionWorkspaceOperationFailure, NotionWorkspaceResult};

use super::NotionWorkspaceRuntime;

impl NotionWorkspaceRuntime {
    pub(super) fn with_session_recovery<T>(
        &self,
        mut operation: impl FnMut(
            &crate::live::credentials::NotionDesktopSession,
        ) -> Result<T, NotionLiveError>,
    ) -> NotionWorkspaceResult<T> {
        crate::live::credentials::with_notion_session_recovery(|session| {
            if session.user_id() != self.active_user_id {
                return Err(NotionLiveError::Session(
                    NotionSessionFailure::ActiveUserMismatch,
                ));
            }
            operation(session)
        })
        .map(|recovery| recovery.into_value())
        .map_err(|error| {
            if error.recovery_disposition().discards_previous_state() {
                NotionWorkspaceOperationFailure::requiring_rebootstrap(
                    error.to_string(),
                    self.rebootstrap_api.clone(),
                )
            } else {
                NotionWorkspaceOperationFailure::new(error.to_string())
            }
        })
    }
}
