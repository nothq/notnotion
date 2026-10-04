use crate::model::{
    CreateCustomEmojiPageIconRequest, NotionWorkspaceApi, PageMutation, PageMutationRequest,
    PageShellIcon, SetPageIconRequest, UploadPageIconRequest,
};

pub(super) enum PageLinkIconCommitRequest {
    CreateCustomEmoji(CreateCustomEmojiPageIconRequest),
    UploadPageIcon(UploadPageIconRequest),
}

impl PageLinkIconCommitRequest {
    pub(super) fn execute(
        self,
        workspace_api: &dyn NotionWorkspaceApi,
        page_id: &str,
        block_id: &str,
    ) -> crate::model::NotionWorkspaceResult<PageShellIcon> {
        let icon = match self {
            Self::CreateCustomEmoji(request) => {
                workspace_api.create_custom_emoji_page_icon(request)
            }
            Self::UploadPageIcon(request) => workspace_api.upload_page_icon(request),
        };
        commit_page_link_icon_remotely(workspace_api, page_id, block_id, icon)
    }
}

fn commit_page_link_icon_remotely(
    workspace_api: &dyn NotionWorkspaceApi,
    page_id: &str,
    block_id: &str,
    icon: crate::model::NotionWorkspaceResult<PageShellIcon>,
) -> crate::model::NotionWorkspaceResult<PageShellIcon> {
    let icon = icon?;
    let request = SetPageIconRequest::new(block_id.to_owned(), Some(icon.clone()))?;
    workspace_api
        .apply_page_mutation(PageMutationRequest {
            page_block_id: page_id.to_owned(),
            mutation: PageMutation::SetPageIcon(request),
        })
        .map_err(|error| {
            error.with_context("the remote icon was created but its page assignment failed")
        })?;
    Ok(icon)
}
