use crate::model::{LoadCalendarItemsRequest, LoadCalendarItemsResult};

use super::super::NotionWorkspaceRuntime;
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

impl NotionWorkspaceRuntime {
    pub(super) fn load_calendar_items_once(
        &self,
        session: &NotionDesktopSession,
        request: LoadCalendarItemsRequest,
    ) -> Result<LoadCalendarItemsResult, NotionLiveError> {
        let state = self.collection_query_state.as_ref().ok_or_else(|| {
            "Notion Calendar item loading requires an active date view".to_string()
        })?;
        crate::live::query_calendar_items(session, state, request)
    }
}
