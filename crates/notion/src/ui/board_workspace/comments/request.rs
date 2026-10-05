use crate::ui::board_workspace::PageDocumentAction;
use std::sync::Arc;

use gpui::Context;

use crate::model::{
    CardPage, NotionCommentMutationRequest, NotionCommentTargetId, NotionCommentWorkspaceUsers,
    NotionWorkspaceApi, NotionWorkspaceOperationFailure, NotionWorkspaceResult,
};
use crate::ui::surface::{NotionCommentsPanelState, NotionCommentsState, PageDocuments};
use crate::ui::SurfaceState;

pub(super) struct CommentRequestGuard {
    session: Arc<()>,
    page_id: NotionCommentTargetId,
    target_id: NotionCommentTargetId,
}

impl CommentRequestGuard {
    pub(super) fn for_panel(panel: &NotionCommentsPanelState) -> Self {
        Self {
            session: panel.session.clone(),
            page_id: panel.page_id.clone(),
            target_id: panel.target_id.clone(),
        }
    }

    fn authoritative_target_matches(&self, page: &CardPage) -> bool {
        page.block_id == self.page_id.as_str()
            && (page.block_id == self.target_id.as_str()
                || page
                    .blocks
                    .iter()
                    .any(|block| block.block_id == self.target_id.as_str()))
    }
}

pub(super) struct NotionCommentRequest {
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    guard: CommentRequestGuard,
    operation: CommentRequestOperation,
}

enum CommentRequestOperation {
    Users,
    Mutation(Box<NotionCommentMutationRequest>),
}

enum CommentRequestResult {
    Users(NotionWorkspaceResult<NotionCommentWorkspaceUsers>),
    Mutation(NotionWorkspaceResult<Box<CardPage>>),
}

struct NotionCommentCompletion {
    guard: CommentRequestGuard,
    result: CommentRequestResult,
}

enum CommentHostUpdate {
    Unchanged,
    Notify,
    InvalidTarget,
    ReplacePage(Box<CardPage>),
    Failure {
        operation: &'static str,
        error: NotionWorkspaceOperationFailure,
        notify: bool,
    },
}

impl NotionCommentRequest {
    pub(super) fn users(
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        guard: CommentRequestGuard,
    ) -> Self {
        Self {
            workspace_api,
            guard,
            operation: CommentRequestOperation::Users,
        }
    }

    pub(super) fn mutation(
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        request: NotionCommentMutationRequest,
        guard: CommentRequestGuard,
    ) -> Self {
        Self {
            workspace_api,
            guard,
            operation: CommentRequestOperation::Mutation(Box::new(request)),
        }
    }

    fn execute(self) -> NotionCommentCompletion {
        let result = match self.operation {
            CommentRequestOperation::Users => {
                CommentRequestResult::Users(self.workspace_api.load_comment_users())
            }
            CommentRequestOperation::Mutation(request) => CommentRequestResult::Mutation(
                self.workspace_api
                    .mutate_page_comment(*request)
                    .map(Box::new),
            ),
        };
        NotionCommentCompletion {
            guard: self.guard,
            result,
        }
    }
}

impl NotionCommentsState {
    fn finish_comment_request(
        &mut self,
        completion: NotionCommentCompletion,
        documents: &PageDocuments,
    ) -> CommentHostUpdate {
        let guard = completion.guard;
        if !self.comments_panel_matches(
            &guard.session,
            &guard.page_id,
            &guard.target_id,
            documents.loaded_comment_page(guard.page_id.as_str()),
        ) {
            return CommentHostUpdate::Unchanged;
        }
        match completion.result {
            CommentRequestResult::Users(result) => self.finish_comment_users(result),
            CommentRequestResult::Mutation(result) => self.finish_comment_mutation(guard, result),
        }
    }

    fn finish_comment_users(
        &mut self,
        result: NotionWorkspaceResult<NotionCommentWorkspaceUsers>,
    ) -> CommentHostUpdate {
        match result {
            Ok(users) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.replace_users(users.users);
                }
                CommentHostUpdate::Notify
            }
            Err(error) => CommentHostUpdate::Failure {
                operation: "comment mention users failed to load",
                error,
                notify: false,
            },
        }
    }

    fn finish_comment_mutation(
        &mut self,
        guard: CommentRequestGuard,
        result: NotionWorkspaceResult<Box<CardPage>>,
    ) -> CommentHostUpdate {
        match result {
            Ok(page) if guard.authoritative_target_matches(&page) => {
                CommentHostUpdate::ReplacePage(page)
            }
            Ok(_) => CommentHostUpdate::InvalidTarget,
            Err(error) => {
                self.enable_notion_comment_composer();
                CommentHostUpdate::Failure {
                    operation: "comment mutation failed",
                    error,
                    notify: true,
                }
            }
        }
    }
}

impl SurfaceState {
    pub(super) fn spawn_notion_comment_request(
        &mut self,
        request: NotionCommentRequest,
        cx: &mut Context<Self>,
    ) {
        self.spawn_background_task(
            request,
            cx,
            NotionCommentRequest::execute,
            |this, completion, cx| match this
                .comments
                .finish_comment_request(completion, &this.page_documents)
            {
                CommentHostUpdate::Unchanged => {}
                CommentHostUpdate::Notify => cx.notify(),
                CommentHostUpdate::InvalidTarget => {
                    this.print_notion_error(
                        "authoritative comment reload returned a different target",
                    );
                    this.comments.enable_notion_comment_composer();
                    cx.notify();
                }
                CommentHostUpdate::ReplacePage(page) => {
                    let page = *page;
                    this.dispatch_page_document_action(
                        PageDocumentAction::replace_loaded_and_authority(
                            page.clone(),
                            Arc::new(page),
                        ),
                        cx,
                    );
                    if let Some(panel) = this.comments.panel.as_mut() {
                        panel.reset_composer();
                    }
                    cx.notify();
                }
                CommentHostUpdate::Failure {
                    operation,
                    error,
                    notify,
                } => {
                    if !this.handle_notion_workspace_failure(operation, error, cx) && notify {
                        cx.notify();
                    }
                }
            },
        );
    }
}
