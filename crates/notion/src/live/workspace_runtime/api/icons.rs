use crate::model::{
    CreateCustomEmojiPageIconRequest, NotionCustomEmojiLibrary, PageShellIcon,
    UploadPageIconRequest,
};

use super::super::NotionWorkspaceRuntime;
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

impl NotionWorkspaceRuntime {
    pub(super) fn upload_page_icon_once(
        &self,
        session: &NotionDesktopSession,
        request: UploadPageIconRequest,
    ) -> Result<PageShellIcon, NotionLiveError> {
        let (page_block_id, block_id, name, content_type, bytes) = request.into_parts();
        let (block_id, space_id) = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .page_icon_upload_target(&page_block_id, &block_id)?;
        crate::live::upload_notion_page_icon(
            session,
            &block_id,
            &space_id,
            crate::live::NotionIconFile {
                name: &name,
                content_type: &content_type,
                bytes: bytes.as_ref(),
            },
        )
    }

    pub(super) fn create_custom_emoji_page_icon_once(
        &self,
        session: &NotionDesktopSession,
        request: CreateCustomEmojiPageIconRequest,
    ) -> Result<PageShellIcon, NotionLiveError> {
        self.workspace_context
            .ensure_custom_emoji_creation_allowed()?;
        let (page_block_id, block_id, emoji_name, source) = request.into_parts();
        let (_, space_id) = self
            .mutator
            .lock()
            .map_err(|_| "Notion mutation state lock is poisoned".to_string())?
            .page_icon_upload_target(&page_block_id, &block_id)?;
        let icon = crate::live::create_notion_custom_emoji_page_icon(
            session,
            &space_id,
            &emoji_name,
            source,
        )?;
        self.workspace_context.invalidate_custom_emoji_library()?;
        Ok(icon)
    }

    pub(super) fn load_custom_emoji_library_once(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<NotionCustomEmojiLibrary, NotionLiveError> {
        self.workspace_context.load_custom_emoji_library(session)
    }
}
