use std::sync::Arc;

use gpui::Context;

use crate::model::{
    CardPage, NotionCommentMutationRequest, NotionCommentTargetId, NotionDiscussionId,
    NotionWorkspaceApi,
};
use crate::ui::surface::{NotionCommentsPanelState, NotionCommentsState, PageDocuments};
use crate::ui::SurfaceState;

use super::request::{CommentRequestGuard, NotionCommentRequest};

impl SurfaceState {
    pub(super) fn submit_notion_comment(&mut self, cx: &mut Context<Self>) {
        let workspace_api = self.notion_startup.workspace_api();
        match self
            .comments
            .prepare_comment_request(&self.page_documents, workspace_api)
        {
            Ok(Some(request)) => {
                cx.notify();
                self.spawn_notion_comment_request(request, cx);
            }
            Ok(None) => {}
            Err(error) => self.print_notion_error(error),
        }
    }
}

impl NotionCommentsState {
    fn prepare_comment_request(
        &mut self,
        documents: &PageDocuments,
        workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    ) -> Result<Option<NotionCommentRequest>, String> {
        let Some(panel) = self.panel.as_mut() else {
            return Ok(None);
        };
        if panel.mutation_in_flight || !panel.can_comment {
            return Ok(None);
        }
        let request = panel.prepare_mutation(documents)?;
        let workspace_api = workspace_api
            .ok_or_else(|| "comment mutation requires a live Notion workspace API".to_string())?;
        panel.mutation_in_flight = true;
        let guard = CommentRequestGuard::for_panel(panel);
        Ok(Some(NotionCommentRequest::mutation(
            workspace_api,
            request,
            guard,
        )))
    }
}

impl NotionCommentsPanelState {
    fn prepare_mutation(
        &self,
        documents: &PageDocuments,
    ) -> Result<NotionCommentMutationRequest, String> {
        let page = documents
            .loaded_comment_page(self.page_id.as_str())
            .ok_or_else(|| "comment page is no longer loaded".to_string())?;
        let draft = self.draft()?;
        match self.reply_to.clone() {
            Some(discussion_id) => NotionCommentMutationRequest::reply(
                page,
                self.target_id.clone(),
                discussion_id,
                draft,
            ),
            None => NotionCommentMutationRequest::new_thread(page, self.target_id.clone(), draft),
        }
    }
}

impl NotionCommentsState {
    pub(super) fn begin_notion_comment_reply(
        &mut self,
        discussion_id: NotionDiscussionId,
        cx: &mut Context<SurfaceState>,
    ) {
        let Some(panel) = self.panel.as_mut() else {
            return;
        };
        if !panel
            .threads
            .iter()
            .any(|thread| thread.discussion_id == discussion_id && thread.replyable)
        {
            return;
        }
        panel.reply_to = Some(discussion_id);
        panel.focus_requested.set(true);
        cx.notify();
    }

    pub(super) fn enable_notion_comment_composer(&mut self) {
        if let Some(panel) = self.panel.as_mut() {
            panel.mutation_in_flight = false;
        }
    }

    pub(super) fn comments_panel_matches(
        &self,
        session: &Arc<()>,
        page_id: &NotionCommentTargetId,
        target_id: &NotionCommentTargetId,
        loaded_page: Option<&CardPage>,
    ) -> bool {
        self.panel.as_ref().is_some_and(|panel| {
            Arc::ptr_eq(&panel.session, session)
                && panel.page_id == *page_id
                && panel.target_id == *target_id
                && loaded_page.is_some_and(|page| {
                    page.block_id == target_id.as_str()
                        || page
                            .blocks
                            .iter()
                            .any(|block| block.block_id == target_id.as_str())
                })
        })
    }
}
