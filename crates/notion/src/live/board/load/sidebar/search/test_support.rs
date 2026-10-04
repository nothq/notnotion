use std::{collections::HashMap, sync::Arc};

use serde_json::{json, Value};

use super::super::super::super::{BoardTarget, LiveWorkspaceSearchContext};
use crate::model::PagePresenceProfile;

pub(super) const CURRENT_PAGE_ID: &str = "00000000-0000-0000-0000-000000000004";

pub(super) fn search_context() -> LiveWorkspaceSearchContext {
    LiveWorkspaceSearchContext {
        space_id: "space-1".to_string(),
        active_user_id: "00000000-0000-0000-0000-000000000005".to_string(),
        active_user_profile: PagePresenceProfile {
            user_id: "00000000-0000-0000-0000-000000000005".to_string(),
            name: "Ada Lovelace".to_string(),
            profile_photo: None,
            avatar: None,
        },
        time_zone: "America/Toronto".to_string(),
        sidebar_breadcrumbs_by_block_id: Arc::new(HashMap::new()),
        quick_find_records:
            super::super::super::super::quick_find_records::QuickFindRecords::for_scope(
                "00000000-0000-0000-0000-000000000005",
                "space-1",
            ),
    }
}

pub(super) fn board_target() -> BoardTarget {
    BoardTarget::parse(&format!(
        "https://www.notion.so/acme/Current-{}?v=00000000000000000000000000000009",
        CURRENT_PAGE_ID.replace('-', "")
    ))
    .expect("parse current page URL")
}

pub(super) fn record(value: Value) -> Value {
    json!({ "value": value })
}

pub(super) fn titled_block(
    id: &str,
    title: &str,
    block_type: &str,
    parent_id: &str,
    parent_table: &str,
) -> Value {
    json!({
        "id": id,
        "type": block_type,
        "parent_id": parent_id,
        "parent_table": parent_table,
        "properties": { "title": [[title]] },
    })
}
