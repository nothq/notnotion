use serde_json::json;

use super::{load_sidebar_attribution_records_with, SyncedAttributionResponse};
use crate::live::{NotionLiveError, NotionResourceFailure, NotionSessionFailure};

#[test]
fn attribution_response_parses_user_and_bot_tables() {
    let response = serde_json::from_value::<SyncedAttributionResponse>(json!({
        "recordMap": {
            "notion_user": {
                "user-1": { "value": { "id": "user-1", "name": "Ada" } },
            },
            "bot": {
                "bot-1": { "value": { "id": "bot-1", "name": "Acme" } },
            },
        },
    }))
    .expect("deserialize typed optional attribution response");

    assert!(response.record_map.users.contains_key("user-1"));
    assert!(response.record_map.bots.contains_key("bot-1"));
}

#[test]
fn user_attribution_failure_degrades_to_absent_metadata() {
    let pointers = vec![("notion_user".to_string(), "user-1".to_string())];

    let records = load_sidebar_attribution_records_with("space-1", &pointers, |body| {
        assert_eq!(
            body,
            &json!({
                "requests": [{
                    "pointer": {
                        "table": "notion_user",
                        "id": "user-1",
                        "spaceId": "space-1",
                    },
                    "version": -1,
                }],
            })
        );
        Err(NotionLiveError::Fatal(
            "invalid optional user attribution".to_string(),
        ))
    })
    .expect("optional user attribution failure should not fail Quick Find");

    assert!(records.users.is_empty());
    assert!(records.bots.is_empty());
}

#[test]
fn bot_attribution_failure_degrades_to_absent_metadata() {
    let pointers = vec![("bot".to_string(), "bot-1".to_string())];

    let records = load_sidebar_attribution_records_with("space-1", &pointers, |body| {
        assert_eq!(body["requests"][0]["pointer"]["table"], "bot");
        Err(NotionLiveError::Unavailable(
            NotionResourceFailure::NotFound {
                diagnostic: "optional bot attribution is unavailable".to_string(),
            },
        ))
    })
    .expect("optional bot attribution failure should not fail Quick Find");

    assert!(records.users.is_empty());
    assert!(records.bots.is_empty());
}

#[test]
fn attribution_session_failure_remains_recoverable() {
    let pointers = vec![("notion_user".to_string(), "user-1".to_string())];

    let result = load_sidebar_attribution_records_with("space-1", &pointers, |_| {
        Err(NotionLiveError::Session(NotionSessionFailure::Rejected {
            status: 401,
        }))
    });

    assert!(matches!(
        result,
        Err(NotionLiveError::Session(NotionSessionFailure::Rejected {
            status: 401
        }))
    ));
}
