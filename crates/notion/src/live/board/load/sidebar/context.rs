use super::super::super::{
    block_value, load_complete_page_response_with_session, load_user_context_with_session,
    record_map_table, required_string, BoardTarget, UserContext,
};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

pub(super) struct SidebarWorkspaceContext {
    pub(super) space_id: String,
    pub(super) user: UserContext,
}

pub(super) fn load_sidebar_workspace_context(
    session: &NotionDesktopSession,
    current_board_url: &str,
) -> Result<SidebarWorkspaceContext, NotionLiveError> {
    let board_target = BoardTarget::parse(current_board_url)?;
    let bootstrap =
        load_complete_page_response_with_session(session, &board_target.collection_view_block_id)?;
    let root = block_value(
        record_map_table(bootstrap.as_value(), "block")?,
        &board_target.collection_view_block_id,
    )?;
    let space_id = required_string(root, "space_id")?.to_string();
    let user = load_user_context_with_session(session, &space_id)?;
    Ok(SidebarWorkspaceContext { space_id, user })
}
