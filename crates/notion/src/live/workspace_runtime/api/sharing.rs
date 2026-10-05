use crate::live::{
    credentials::NotionDesktopSession, load_page_sharing, mutate_page_sharing, NotionLiveError,
};
use crate::model::{
    LoadNotionPageSharingRequest, MutateNotionPageSharingRequest, NotionPageSharingSnapshot,
};

use super::super::NotionWorkspaceRuntime;

impl NotionWorkspaceRuntime {
    pub(super) fn load_page_sharing_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadNotionPageSharingRequest,
    ) -> Result<NotionPageSharingSnapshot, NotionLiveError> {
        load_page_sharing(session, &self.workspace_context, request)
    }

    pub(super) fn mutate_page_sharing_once(
        &self,
        session: &NotionDesktopSession,
        request: MutateNotionPageSharingRequest,
    ) -> Result<NotionPageSharingSnapshot, NotionLiveError> {
        let target_id = request.target_id().clone();
        self.workspace_context
            .invalidate_navigation_bootstrap(target_id.as_str())?;
        let target_id = mutate_page_sharing(session, &self.workspace_context, request)?;
        if let Err(error) = self
            .workspace_context
            .invalidate_navigation_bootstrap(target_id.as_str())
        {
            println!(
                "notnotion navigation cache invalidation failed after sharing mutation: {error}"
            );
        }
        load_page_sharing(
            session,
            &self.workspace_context,
            LoadNotionPageSharingRequest::new(target_id),
        )
    }
}
