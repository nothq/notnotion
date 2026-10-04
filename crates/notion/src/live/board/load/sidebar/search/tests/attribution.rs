use serde_json::{json, Map};

use super::super::result::{
    search_result_attribution_display_name, shape_search_result, SearchRecordMaps,
};
use super::super::test_support::{board_target, record, search_context};
use super::super::transport::{combined_search_attribution_records, SearchResultWire};

#[test]
fn attribution_name_is_omitted_when_the_cached_user_does_not_match() {
    let block = json!({
        "last_edited_by_table": "notion_user",
        "last_edited_by_id": "00000000-0000-0000-0000-000000000006",
    });

    assert_eq!(
        search_result_attribution_display_name(&block, &search_context(), None, None),
        None
    );
}

#[test]
fn attribution_name_uses_a_user_record_returned_with_search_results() {
    let editor_id = "00000000-0000-0000-0000-000000000006";
    let block = json!({
        "last_edited_by_table": "notion_user",
        "last_edited_by_id": editor_id,
    });
    let mut users = Map::new();
    users.insert(
        editor_id.replace('-', ""),
        record(json!({
            "id": editor_id,
            "name": "Grace Hopper",
        })),
    );

    assert_eq!(
        search_result_attribution_display_name(&block, &search_context(), Some(&users), None,)
            .as_deref(),
        Some("Grace Hopper")
    );
}

#[test]
fn attribution_name_uses_a_bot_record_returned_with_search_results() {
    let editor_id = "00000000-0000-0000-0000-000000000007";
    let block = json!({
        "last_edited_by_table": "bot",
        "last_edited_by_id": editor_id,
    });
    let mut bots = Map::new();
    bots.insert(
        editor_id.to_string(),
        record(json!({
            "id": editor_id,
            "name": "Tiny Bot",
        })),
    );

    assert_eq!(
        search_result_attribution_display_name(&block, &search_context(), None, Some(&bots))
            .as_deref(),
        Some("Tiny Bot")
    );
}

#[test]
fn conflicting_optional_attribution_keeps_the_valid_response_record() {
    let editor_id = "00000000-0000-0000-0000-000000000006";
    let mut response_users = Map::new();
    response_users.insert(
        editor_id.to_string(),
        record(json!({
            "id": editor_id,
            "name": "Response name",
        })),
    );
    let mut hydrated_users = Map::new();
    hydrated_users.insert(
        editor_id.to_string(),
        record(json!({
            "id": editor_id,
            "name": "Conflicting hydrated name",
        })),
    );

    let users =
        combined_search_attribution_records("notion_user", Some(&response_users), &hydrated_users);
    let block = json!({
        "last_edited_by_table": "notion_user",
        "last_edited_by_id": editor_id,
    });

    assert_eq!(
        search_result_attribution_display_name(&block, &search_context(), Some(&users), None,)
            .as_deref(),
        Some("Response name")
    );
}

#[test]
fn absent_user_and_bot_attribution_metadata_preserves_valid_search_rows() {
    let attribution_cases = [
        ("notion_user", "00000000-0000-0000-0000-000000000006"),
        ("bot", "00000000-0000-0000-0000-000000000007"),
    ];
    let empty_attribution = Map::new();
    let collections = Map::new();

    for (index, (table, attribution_id)) in attribution_cases.into_iter().enumerate() {
        let block_id = format!("00000000-0000-0000-0000-{index:012}");
        let mut blocks = Map::new();
        blocks.insert(
            block_id.clone(),
            record(json!({
                "id": block_id.clone(),
                "space_id": "space-1",
                "type": "page",
                "properties": { "title": [["Still available"]] },
                "created_by_table": table,
                "created_by_id": attribution_id,
            })),
        );
        let wire = serde_json::from_value::<SearchResultWire>(json!({
            "id": block_id,
            "spaceId": "space-1",
        }))
        .expect("deserialize search result with optional attribution");

        let shaped = shape_search_result(
            &wire,
            &search_context(),
            &board_target(),
            SearchRecordMaps {
                blocks: &blocks,
                collections: &collections,
                teams: None,
                users: Some(&empty_attribution),
                bots: Some(&empty_attribution),
            },
        )
        .expect("shape row without optional attribution metadata")
        .expect("optional attribution must not remove a valid row");

        assert_eq!(shaped.title, "Still available");
        assert_eq!(shaped.editor_display_name, None);
    }
}

#[test]
fn creator_attribution_precedes_the_last_editor_like_notion_quick_find() {
    let creator_id = "00000000-0000-0000-0000-000000000007";
    let block = json!({
        "created_by_table": "bot",
        "created_by_id": creator_id,
        "last_edited_by_table": "notion_user",
        "last_edited_by_id": "00000000-0000-0000-0000-000000000005",
    });
    let mut bots = Map::new();
    bots.insert(
        creator_id.to_string(),
        record(json!({
            "id": creator_id,
            "name": "Tiny Bot",
        })),
    );

    assert_eq!(
        search_result_attribution_display_name(&block, &search_context(), None, Some(&bots),)
            .as_deref(),
        Some("Tiny Bot")
    );
}
