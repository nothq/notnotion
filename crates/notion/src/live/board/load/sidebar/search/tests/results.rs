use serde_json::{json, Map, Value};

use super::super::super::super::super::optional_record_map_table;
use super::super::super::records::combined_block_records;
use super::super::result::{
    search_match_snippet, shape_search_result, shape_search_results, SearchRecordMaps,
};
use super::super::test_support::{board_target, record, search_context, CURRENT_PAGE_ID};
use super::super::transport::{
    combined_search_collection_records, missing_search_collection_ids, SearchResponse,
    SearchResultWire,
};
use crate::model::{PageShellEditedAt, PageShellSearchBadge};

#[test]
fn match_snippet_strips_markers_and_only_flattens_line_breaks() {
    assert_eq!(
        search_match_snippet("Keep  meaningful <gzkNfoUU>spaces</gzkNfoUU>\r\n\tacross lines"),
        Some("Keep  meaningful spaces across lines".to_string())
    );
    assert_eq!(search_match_snippet("<gzkNfoUU></gzkNfoUU>"), None);
}

fn full_search_shape_blocks() -> Map<String, Value> {
    let mut blocks = Map::new();
    blocks.insert(
        CURRENT_PAGE_ID.to_string(),
        record(json!({
            "id": CURRENT_PAGE_ID,
            "type": "collection_view_page",
            "parent_id": "team",
            "parent_table": "block",
            "properties": { "title": [["Current database"]] },
            // 2024-01-01 12:00 UTC is unambiguously Jan 1 in Toronto and
            // remains in the formatter's absolute-date branch.
            "last_edited_time": 1_704_110_400_000_u64,
            "last_edited_by_table": "notion_user",
            "last_edited_by_id": "00000000000000000000000000000005",
        })),
    );
    blocks.insert(
        "team".to_string(),
        record(super::super::test_support::titled_block(
            "team", "Team", "page", "middle", "block",
        )),
    );
    blocks.insert(
        "middle".to_string(),
        record(super::super::test_support::titled_block(
            "middle", "Middle", "page", "general", "block",
        )),
    );
    blocks.insert(
        "general".to_string(),
        record(super::super::test_support::titled_block(
            "general", "General", "page", "space-1", "space",
        )),
    );
    blocks
}

fn full_search_shape_wire() -> SearchResultWire {
    serde_json::from_value(json!({
        "id": CURRENT_PAGE_ID,
        "spaceId": "space-1",
        "collectionId": null,
        "highlight": {
            "text": "A <gzkNfoUU>matching phrase</gzkNfoUU>\nfrom the body"
        },
        "badges": ["Untrusted API badge"]
    }))
    .expect("deserialize highlighted search result")
}

#[test]
fn search_result_shapes_full_breadcrumb_cached_editor_and_derived_badges() {
    let blocks = full_search_shape_blocks();
    let collections = Map::new();
    let shaped = shape_search_result(
        &full_search_shape_wire(),
        &search_context(),
        &board_target(),
        SearchRecordMaps {
            blocks: &blocks,
            collections: &collections,
            teams: None,
            users: None,
            bots: None,
        },
    )
    .expect("shape search result")
    .expect("search result should be available");

    assert_eq!(shaped.highlight.as_deref(), Some("General / … / Team"));
    assert_eq!(
        shaped.match_snippet.as_deref(),
        Some("A matching phrase from the body")
    );
    assert_eq!(shaped.editor_display_name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(shaped.edited_label, None);
    assert_eq!(
        shaped.edited_at,
        Some(PageShellEditedAt {
            unix_millis: 1_704_110_400_000,
            date_label: "Jan 1".to_string(),
        })
    );
    assert_eq!(
        shaped.target_board_url,
        format!(
            "https://www.notion.so/acme/Current-{}?v=00000000000000000000000000000009",
            CURRENT_PAGE_ID.replace('-', "")
        )
    );
    assert_eq!(
        shaped.badges,
        vec![
            PageShellSearchBadge::CurrentPage,
            PageShellSearchBadge::Database,
        ]
    );
}

#[test]
fn compact_search_result_id_uses_the_matching_record_and_shapes_a_canonical_id() {
    let compact_id = CURRENT_PAGE_ID.replace('-', "");
    let mut blocks = Map::new();
    blocks.insert(
        compact_id.clone(),
        record(json!({
            "id": compact_id,
            "type": "page",
            "properties": { "title": [["Compact page"]] },
        })),
    );
    let wire = serde_json::from_value::<SearchResultWire>(json!({
        "id": compact_id,
        "spaceId": "space-1",
        "collectionId": null,
    }))
    .expect("deserialize compact search result");
    let collections = Map::new();

    let shaped = shape_search_result(
        &wire,
        &search_context(),
        &board_target(),
        SearchRecordMaps {
            blocks: &blocks,
            collections: &collections,
            teams: None,
            users: None,
            bots: None,
        },
    )
    .expect("shape compact search result")
    .expect("compact search result should be available");

    assert_eq!(shaped.block_id, CURRENT_PAGE_ID);
    assert_eq!(shaped.title, "Compact page");
}

#[test]
fn collection_view_search_result_uses_its_collection_name_when_the_wire_omits_collection_id() {
    let database_page_id = "00000000-0000-0000-0000-000000000006";
    let mut blocks = Map::new();
    blocks.insert(
        database_page_id.to_string(),
        record(json!({
            "id": database_page_id,
            "space_id": "space-1",
            "type": "collection_view",
            "properties": { "title": [["Symphony"]] },
            "collection_id": "collection-1",
        })),
    );
    let mut collections = Map::new();
    collections.insert(
        "collection-1".to_string(),
        record(json!({
            "id": "collection-1",
            "name": [["Acme Playground"]],
        })),
    );
    let wire = serde_json::from_value::<SearchResultWire>(json!({
        "id": database_page_id,
        "spaceId": "space-1"
    }))
    .expect("deserialize collection view search result without collectionId");

    assert_eq!(
        shape_search_result(
            &wire,
            &search_context(),
            &board_target(),
            SearchRecordMaps {
                blocks: &blocks,
                collections: &collections,
                teams: None,
                users: None,
                bots: None,
            },
        )
        .expect("shape collection view search result")
        .expect("collection view search result should be available")
        .title,
        "Acme Playground"
    );
}

fn full_search_envelope(database_page_id: &str) -> Value {
    json!({
        "total": 3,
        "results": [
            {
                "id": database_page_id,
                "spaceId": "space-1",
                "highlight": { "text": "A <gzkNfoUU>database</gzkNfoUU> match" },
            },
            {
                "id": "00000000-0000-0000-0000-000000000007",
                "spaceId": "space-1",
            },
            {
                "id": "00000000-0000-0000-0000-000000000008",
                "spaceId": "space-1",
            },
        ],
        "recordMap": {
            "block": {
                "00000000-0000-0000-0000-000000000007": { "role": "none" },
                "00000000-0000-0000-0000-000000000008": record(json!({
                    "id": "00000000-0000-0000-0000-000000000008",
                    "type": "collection_view",
                    "collection_id": "revoked-collection",
                    "properties": { "title": [["Stale block title"]] },
                })),
            },
            "collection": {
                "revoked-collection": { "role": "none" },
            },
        },
    })
}

fn hydrated_database_blocks(database_page_id: &str) -> Map<String, Value> {
    Map::from_iter([(
        database_page_id.to_string(),
        record(json!({
            "id": database_page_id,
            "space_id": "space-1",
            "type": "collection_view",
            "properties": { "title": [["Wrong block title"]] },
            "collection_id": "collection-1",
        })),
    )])
}

#[test]
fn full_search_envelope_hydrates_missing_database_records_and_skips_unavailable_rows() {
    let database_page_id = "00000000-0000-0000-0000-000000000006";
    let response = full_search_envelope(database_page_id);
    let decoded = serde_json::from_value::<SearchResponse>(response.clone())
        .expect("deserialize complete Quick Find search response");
    let response_blocks = optional_record_map_table(&response, "block")
        .expect("full search response should contain its block table");
    let hydrated_blocks = hydrated_database_blocks(database_page_id);
    let blocks = combined_block_records([response_blocks, &hydrated_blocks])
        .expect("combine response and hydrated blocks");
    let response_collections = optional_record_map_table(&response, "collection");
    let missing = missing_search_collection_ids(&decoded.results, &blocks, response_collections)
        .expect("identify collections revealed by hydrated blocks");
    assert_eq!(missing, vec!["collection-1".to_string()]);

    let hydrated_collections = Map::from_iter([(
        "collection-1".to_string(),
        record(json!({ "id": "collection-1", "name": [["Hydrated database title"]] })),
    )]);
    let collections =
        combined_search_collection_records(response_collections, hydrated_collections)
            .expect("combine response and hydrated collections");
    let shaped = shape_search_results(
        &decoded.results,
        &search_context(),
        &board_target(),
        SearchRecordMaps {
            blocks: &blocks,
            collections: &collections,
            teams: None,
            users: None,
            bots: None,
        },
    )
    .expect("shape available rows from a full search envelope");

    assert_eq!(shaped.len(), 1);
    assert_eq!(shaped[0].block_id, database_page_id);
    assert_eq!(shaped[0].title, "Hydrated database title");
    assert_eq!(shaped[0].match_snippet.as_deref(), Some("A database match"));
}
