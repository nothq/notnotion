use crate::live::{
    credentials::NotionDesktopSession, load_visible_workspace_users, NotionLiveError,
};
use crate::model::{CardPage, NotionCommentMutationRequest, NotionCommentWorkspaceUsers};

use super::super::NotionWorkspaceRuntime;

impl NotionWorkspaceRuntime {
    pub(super) fn load_comment_users_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<NotionCommentWorkspaceUsers, NotionLiveError> {
        Ok(NotionCommentWorkspaceUsers {
            users: load_visible_workspace_users(session, &self.workspace_context)?,
        })
    }

    pub(super) fn mutate_page_comment_once(
        &self,
        session: &NotionDesktopSession,
        request: NotionCommentMutationRequest,
    ) -> Result<CardPage, NotionLiveError> {
        let page_id = request.page_id().clone();
        self.workspace_context
            .invalidate_navigation_bootstrap(page_id.as_str())?;
        let visible_users = load_visible_workspace_users(session, &self.workspace_context)?;
        self.mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .apply_comment_mutation(session, &request, &visible_users)?;
        if let Err(error) = self
            .workspace_context
            .invalidate_navigation_bootstrap(page_id.as_str())
        {
            println!(
                "notnotion navigation cache invalidation failed after comment mutation: {error}"
            );
        }
        self.load_card_page_once(session, page_id.as_str())
    }
}
