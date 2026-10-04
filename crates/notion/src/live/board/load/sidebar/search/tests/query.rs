use serde_json::{json, Value};

use super::super::query::search_request_body;
use crate::model::{RecentPageVisit, SearchWorkspaceRequest, SearchWorkspaceScope};

#[test]
fn quick_find_request_serializes_fifty_typed_recent_page_boosts() {
    let request = SearchWorkspaceRequest {
        current_board_url:
            "https://www.notion.so/acme/Current-Page-00000000000000000000000000000001".to_string(),
        query: "payroll".to_string(),
        scope: SearchWorkspaceScope::TitleOnly,
        limit: 20,
        search_session_id: "session-1".to_string(),
        flow_number: 7,
        recent_pages_for_boosting: (0..52)
            .map(|index| RecentPageVisit {
                page_id: format!("page-{index:02}"),
                visited_at_unix_millis: 10_000 - index,
            })
            .collect(),
        excluded_block_ids: vec!["local-page-1".to_string(), "local-page-2".to_string()],
    };

    let body = search_request_body("space-1", &request);
    let expected = (0..50)
        .map(|index| {
            json!({
                "visitedAt": 10_000 - index,
                "pageId": format!("page-{index:02}"),
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(
        body.get("recentPagesForBoosting"),
        Some(&Value::Array(expected))
    );
    assert_eq!(
        body.get("excludedBlockIds"),
        Some(&json!(["local-page-1", "local-page-2"]))
    );
    assert_eq!(body.get("searchSessionId"), Some(&json!("session-1")));
    assert_eq!(body.get("searchSessionFlowNumber"), Some(&json!(7)));
}
