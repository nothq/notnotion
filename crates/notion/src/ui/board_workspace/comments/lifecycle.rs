use gpui::Context;

use crate::model::NotionCommentTargetId;
use crate::ui::surface::{NotionCommentsPanelState, NotionCommentsState, PageDocuments};
use crate::ui::SurfaceState;

use super::request::{CommentRequestGuard, NotionCommentRequest};

impl SurfaceState {
    pub(crate) fn open_notion_page_comments(
        &mut self,
        page_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        let mut state = match self
            .page_documents
            .prepare_notion_comments_panel(&page_id, target_id)
        {
            Ok(state) => state,
            Err(error) => {
                self.print_notion_error(error);
                return;
            }
        };
        let workspace_api = self.notion_startup.workspace_api();
        if workspace_api.is_none() {
            state.can_comment = false;
        }
        let guard = CommentRequestGuard::for_panel(&state);
        self.page_editor.reset_page_composer();
        self.page_editor.page_block_context_menu = None;
        self.notion_chrome.toolbar_dialog = None;
        self.notion_chrome.inline_toolbar_dialog = None;
        self.notion_chrome.date_undated_dialog = None;
        self.notion_chrome.inline_database_view_menu = None;
        self.notion_chrome.ai_autofill_dialog = None;
        self.notion_chrome.status_property_picker = None;
        self.notion_chrome.share_dialog = None;
        self.comments.panel = Some(state);
        cx.notify();
        if let Some(workspace_api) = workspace_api {
            self.spawn_notion_comment_request(
                NotionCommentRequest::users(workspace_api, guard),
                cx,
            );
        }
    }
}

impl PageDocuments {
    fn prepare_notion_comments_panel(
        &self,
        page_id: &str,
        target_id: String,
    ) -> Result<NotionCommentsPanelState, String> {
        let target_id = NotionCommentTargetId::try_from(target_id)?;
        let page = self
            .loaded_comment_page(page_id)
            .ok_or_else(|| format!("Notion page {page_id} is not loaded for comments"))?;
        NotionCommentsPanelState::new(page, target_id)
    }
}

impl NotionCommentsState {
    pub(crate) fn handle_notion_comments_key_down(
        &mut self,
        event: &crate::ui::KeyDownEvent,
        cx: &mut Context<SurfaceState>,
    ) -> bool {
        if event.keystroke.key != "escape" || self.panel.is_none() {
            return false;
        }
        self.dismiss_notion_comments_panel(cx);
        true
    }

    pub(crate) fn dismiss_notion_comments_panel(&mut self, cx: &mut Context<SurfaceState>) {
        if self.panel.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn dismiss_notion_comments_panel_without_notify(&mut self) {
        self.panel = None;
    }
}
