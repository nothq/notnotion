use serde_json::json;

use super::super::transport::{block_request, initial_sync_request};

#[test]
fn initial_sync_wire_matches_notion_desktop() {
    let request = initial_sync_request("space-1", vec![block_request("space-1", "block-1", -1)]);
    assert_eq!(
        serde_json::to_value(request).expect("serialize Initial sync request"),
        json!({
            "requests": [{
                "pointer": { "table": "block", "id": "block-1", "spaceId": "space-1" },
                "version": -1,
            }],
            "spacePointer": { "table": "space", "id": "space-1" },
        })
    );
}
