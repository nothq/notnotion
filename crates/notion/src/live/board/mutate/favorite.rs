use serde_json::json;

use crate::live::{
    credentials::{current_notion_desktop_session, NotionDesktopSession},
    http::post_private_api_in_space_with_session,
    NotionLiveError,
};

use super::super::{
    FavoriteMutationContext, LiveBoardMutator, NotionPrivateApiEndpoint, SystemTime, UNIX_EPOCH,
};

impl LiveBoardMutator {
    pub(crate) fn favorite_context(&self) -> FavoriteMutationContext {
        self.context.favorite().clone()
    }

    pub fn set_favorited(&self, is_favorited: bool) -> Result<(), String> {
        let session = current_notion_desktop_session().map_err(|error| error.to_string())?;
        self.set_favorited_with_session(&session, is_favorited)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn set_favorited_with_session(
        &self,
        session: &NotionDesktopSession,
        is_favorited: bool,
    ) -> Result<(), NotionLiveError> {
        Self::set_favorited_with_context(session, self.context.favorite(), is_favorited)
    }

    pub(crate) fn set_favorited_with_context(
        session: &NotionDesktopSession,
        context: &FavoriteMutationContext,
        is_favorited: bool,
    ) -> Result<(), NotionLiveError> {
        let Some(space_view_id) = context.favorite_space_view_id.as_deref() else {
            return Err(format!(
                "missing favorite space_view context for space {}",
                context.space_id
            )
            .into());
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
        let (command, user_action) = if is_favorited {
            ("listBefore", "TopbarFavoriteButton.handleBookmarkClick")
        } else {
            ("listRemove", "TopbarFavoriteButton.handleUnbookmarkClick")
        };

        post_private_api_in_space_with_session::<_, serde_json::Value>(
            session,
            NotionPrivateApiEndpoint::SaveTransactionsFanout,
            &context.space_id,
            &json!({
                "requestId": uuid::Uuid::new_v4().to_string(),
                "transactions": [{
                    "id": uuid::Uuid::new_v4().to_string(),
                    "spaceId": context.space_id,
                    "debug": {
                        "userAction": user_action,
                        "navParentClientGateState": "on",
                        "clientCommitTimeMs": now,
                    },
                    "operations": [{
                        "pointer": {
                            "id": space_view_id,
                            "table": "space_view",
                            "spaceId": context.space_id,
                        },
                        "path": ["bookmarked_pages"],
                        "command": command,
                        "args": {
                            "id": context.page_block_id,
                        },
                    }],
                }],
            }),
        )?;
        Ok(())
    }
}
