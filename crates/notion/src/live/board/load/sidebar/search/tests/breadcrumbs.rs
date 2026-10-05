use std::{collections::HashMap, sync::Arc};

use serde_json::{json, Map};

use super::super::breadcrumb::{resolve_search_breadcrumb, search_result_breadcrumb};
use super::super::test_support::{record, search_context, titled_block, CURRENT_PAGE_ID};

#[test]
fn breadcrumb_supports_collection_parents_and_preserves_two_levels() {
    let result = json!({
        "parent_id": "collection-1",
        "parent_table": "collection",
    });
    let mut collections = Map::new();
    collections.insert(
        "collection-1".to_string(),
        record(json!({
            "id": "collection-1",
            "name": [["Symphony"]],
            "parent_id": "database-view",
            "parent_table": "block",
        })),
    );
    let mut blocks = Map::new();
    blocks.insert(
        "database-view".to_string(),
        record(json!({
            "id": "database-view",
            "type": "collection_view",
            "collection_id": "collection-1",
            "parent_id": "symphony-root",
            "parent_table": "block",
        })),
    );
    blocks.insert(
        "symphony-root".to_string(),
        record(json!({
            "id": "symphony-root",
            "type": "page",
            "properties": { "title": [["Symphony"]] },
            "parent_id": "space-1",
            "parent_table": "space",
        })),
    );

    assert_eq!(
        search_result_breadcrumb(&result, &blocks, Some(&collections), None)
            .expect("shape collection breadcrumb"),
        Some("Symphony / Symphony".to_string())
    );
}

#[test]
fn breadcrumb_uses_the_team_root_and_omits_collection_view_wrappers() {
    let result = json!({
        "parent_id": "fast-road",
        "parent_table": "block",
    });
    let mut blocks = Map::new();
    blocks.insert(
        "fast-road".to_string(),
        record(titled_block(
            "fast-road",
            "Fast Road Capable Robots",
            "page",
            "documents",
            "collection",
        )),
    );
    blocks.insert(
        "docs-wrapper".to_string(),
        record(titled_block(
            "docs-wrapper",
            "Docs",
            "collection_view_page",
            "team-general",
            "team",
        )),
    );
    let mut collections = Map::new();
    collections.insert(
        "documents".to_string(),
        record(json!({
            "id": "documents",
            "name": [["Our Documents"]],
            "parent_id": "docs-wrapper",
            "parent_table": "block",
        })),
    );
    let mut teams = Map::new();
    teams.insert(
        "team-general".to_string(),
        record(json!({
            "id": "team-general",
            "name": "General",
            "parent_id": "space-1",
            "parent_table": "space",
        })),
    );

    assert_eq!(
        search_result_breadcrumb(&result, &blocks, Some(&collections), Some(&teams))
            .expect("shape team-rooted breadcrumb"),
        Some("General / … / Fast Road Capable Robots".to_string())
    );
}

#[test]
fn authoritative_record_breadcrumb_precedes_the_cached_sidebar_fallback() {
    let mut context = search_context();
    context.sidebar_breadcrumbs_by_block_id = Arc::new(HashMap::from([(
        CURRENT_PAGE_ID.to_string(),
        "Cached / Old parent".to_string(),
    )]));

    assert_eq!(
        resolve_search_breadcrumb(
            Some("Fresh / New parent".to_string()),
            &context,
            CURRENT_PAGE_ID,
        )
        .as_deref(),
        Some("Fresh / New parent")
    );
    assert_eq!(
        resolve_search_breadcrumb(None, &context, CURRENT_PAGE_ID).as_deref(),
        Some("Cached / Old parent")
    );
}

#[test]
fn breadcrumb_detects_cycles_across_collection_and_block_parents() {
    let result = json!({
        "parent_id": "shared-id",
        "parent_table": "collection",
    });
    let mut collections = Map::new();
    collections.insert(
        "shared-id".to_string(),
        record(json!({
            "id": "shared-id",
            "name": [["Symphony"]],
            "parent_id": "shared-id",
            "parent_table": "block",
        })),
    );
    let mut blocks = Map::new();
    blocks.insert(
        "shared-id".to_string(),
        record(titled_block(
            "shared-id",
            "Database view",
            "collection_view",
            "shared-id",
            "collection",
        )),
    );

    let error = search_result_breadcrumb(&result, &blocks, Some(&collections), None)
        .expect_err("reject cyclic collection/block ancestry");

    assert!(error.contains("cyclic Notion search result parent chain"));
    assert!(error.contains("collection shared-id"));
}
