use serde::Serialize;
use serde_json::{json, Value};

use crate::model::{SearchWorkspaceRequest, SearchWorkspaceScope};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentPageForBoostingWire<'a> {
    visited_at: u64,
    page_id: &'a str,
}

pub(super) fn search_request_body(space_id: &str, request: &SearchWorkspaceRequest) -> Value {
    let recent_pages_for_boosting = request
        .recent_pages_for_boosting
        .iter()
        .take(SearchWorkspaceRequest::MAX_RECENT_PAGES_FOR_BOOSTING)
        .map(|visit| RecentPageForBoostingWire {
            visited_at: visit.visited_at_unix_millis,
            page_id: &visit.page_id,
        })
        .collect::<Vec<_>>();
    let excluded_block_ids = request
        .excluded_block_ids
        .iter()
        .take(request.limit as usize)
        .collect::<Vec<_>>();
    json!({
        "type": "BlocksInSpace",
        "query": request.query,
        "spaceId": space_id,
        "source": "quick_find",
        "limit": request.limit,
        "peopleBlocksToInclude": "all",
        "sort": { "field": "relevance" },
        "filters": search_filters(request.scope),
        "excludedBlockIds": excluded_block_ids,
        "searchSessionId": request.search_session_id,
        "searchSessionFlowNumber": request.flow_number,
        "recentPagesForBoosting": recent_pages_for_boosting,
        "ignoresHighlight": false,
    })
}

fn search_filters(scope: SearchWorkspaceScope) -> Value {
    json!({
        "isDeletedOnly": false,
        "excludeTemplates": false,
        "navigableBlockContentOnly": scope == SearchWorkspaceScope::TitleOnly,
        "requireEditPermissions": false,
        "includePublicPagesWithoutExplicitAccess": false,
        "ancestors": [],
        "createdBy": [],
        "editedBy": [],
        "lastEditedTime": {},
        "createdTime": {},
        "inTeams": [],
        "excludeSurrogateCollections": false,
        "excludedParentCollectionIds": [],
    })
}
