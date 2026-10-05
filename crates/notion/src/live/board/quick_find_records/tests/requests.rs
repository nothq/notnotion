use std::collections::HashSet;

use serde_json::{json, Map, Value};

use super::super::merge::preview_root_hydration;
use super::super::preview::{preview_record_closure, preview_record_key, preview_requests};
use super::super::test_support::{block_entry, block_table, entry};
use super::super::QUICK_FIND_PREVIEW_REQUEST_LIMIT;

#[test]
fn preview_omits_root_and_uses_missing_then_concrete_child_versions() {
    let mut record_map = Map::from_iter([(
        "block".to_string(),
        Value::Object(Map::from_iter([
            (
                "root".to_string(),
                entry("root", 8, Some(vec!["known", "missing"])),
            ),
            ("known".to_string(), entry("known", 3, None)),
        ])),
    )]);
    let preview_records =
        preview_record_closure(&record_map, "space-1", "root").expect("cached topology");
    let first = preview_requests(&record_map, &preview_records, &HashSet::new());
    assert_eq!(
        first
            .iter()
            .map(|request| (request.pointer.id.as_str(), request.version))
            .collect::<Vec<_>>(),
        vec![("known", 3), ("missing", -1)]
    );
    assert!(first.iter().all(|request| request.pointer.id != "root"));

    record_map
        .get_mut("block")
        .and_then(Value::as_object_mut)
        .expect("block table")
        .insert("missing".to_string(), entry("missing", 2, None));
    let repeat = preview_requests(&record_map, &preview_records, &HashSet::new());
    assert_eq!(
        repeat
            .iter()
            .map(|request| (request.pointer.id.as_str(), request.version))
            .collect::<Vec<_>>(),
        vec![("known", 3), ("missing", 2)]
    );
}

#[test]
fn preview_root_hydration_rejects_missing_deleted_and_unavailable_records() {
    assert_eq!(
        preview_root_hydration(&Map::new(), "root").expect("inspect missing root"),
        None
    );

    let unavailable = block_table([(
        "root",
        json!({
            "role": "none",
            "value": {
                "role": "none",
                "value": {
                    "id": "root",
                    "version": 1,
                    "space_id": "space-1",
                    "type": "page",
                },
            },
        }),
    )]);
    assert_eq!(
        preview_root_hydration(&unavailable, "root").expect("inspect unavailable root"),
        None
    );

    let deleted = block_table([(
        "root",
        block_entry(
            "root",
            1,
            "page",
            Some(Vec::new()),
            Map::from_iter([("alive".to_string(), Value::Bool(false))]),
        ),
    )]);
    assert_eq!(
        preview_root_hydration(&deleted, "root").expect("inspect deleted root"),
        None
    );
}

fn preview_request_root() -> Value {
    block_entry(
        "root",
        1,
        "page",
        Some(vec!["database"]),
        Map::from_iter([
            (
                "parent_table".to_string(),
                Value::String("collection".to_string()),
            ),
            (
                "parent_id".to_string(),
                Value::String("root-collection".to_string()),
            ),
            (
                "properties".to_string(),
                json!({ "people": [["‣", [["u", "property-user"]]]] }),
            ),
        ]),
    )
}

fn preview_request_database() -> Value {
    block_entry(
        "database",
        2,
        "collection_view",
        None,
        Map::from_iter([
            (
                "collection_id".to_string(),
                Value::String("child-collection".to_string()),
            ),
            ("last_edited_time".to_string(), Value::from(100)),
            (
                "last_edited_by_table".to_string(),
                Value::String("notion_user".to_string()),
            ),
            (
                "last_edited_by_id".to_string(),
                Value::String("editor-user".to_string()),
            ),
        ]),
    )
}

#[test]
fn preview_requests_match_quick_find_initial_sync_block_table() {
    let record_map = block_table([
        ("root", preview_request_root()),
        ("database", preview_request_database()),
    ]);
    let pointers = preview_record_closure(&record_map, "space-1", "root").expect("preview closure");
    assert_eq!(
        pointers
            .iter()
            .map(|pointer| (pointer.table.as_str(), pointer.id.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("block", "database"),
            ("collection", "child-collection"),
            ("collection", "root-collection"),
        ]
    );
    assert_eq!(
        preview_requests(&record_map, &pointers, &HashSet::new())
            .into_iter()
            .map(|request| {
                (
                    request.pointer.table,
                    request.pointer.id,
                    request.pointer.space_id,
                    request.version,
                )
            })
            .collect::<Vec<_>>(),
        vec![(
            "block".to_string(),
            "database".to_string(),
            "space-1".to_string(),
            2,
        )]
    );
}

#[test]
fn preview_closure_is_complete_and_requests_are_batched() {
    let child_ids = (0..=QUICK_FIND_PREVIEW_REQUEST_LIMIT)
        .map(|index| format!("child-{index}"))
        .collect::<Vec<_>>();
    let mut blocks = Map::new();
    blocks.insert(
        "root".to_string(),
        entry(
            "root",
            1,
            Some(child_ids.iter().map(String::as_str).collect()),
        ),
    );
    for (version, child_id) in child_ids.iter().enumerate() {
        blocks.insert(
            child_id.clone(),
            block_entry(
                child_id,
                u64::try_from(version + 2).expect("test version"),
                "text",
                None,
                Map::new(),
            ),
        );
    }
    let record_map = Map::from_iter([("block".to_string(), Value::Object(blocks))]);
    let pointers = preview_record_closure(&record_map, "space-1", "root").expect("preview closure");

    assert_eq!(pointers.len(), QUICK_FIND_PREVIEW_REQUEST_LIMIT + 1);
    assert_eq!(
        pointers.first().map(|pointer| pointer.id.as_str()),
        Some("child-0")
    );
    assert_eq!(
        pointers.last().map(|pointer| pointer.id.as_str()),
        Some("child-512")
    );
    let first = preview_requests(&record_map, &pointers, &HashSet::new());
    assert_eq!(first.len(), QUICK_FIND_PREVIEW_REQUEST_LIMIT);
    assert_eq!(
        first.last().map(|request| request.pointer.id.as_str()),
        Some("child-511")
    );
    let requested = first
        .iter()
        .map(|request| preview_record_key(&request.pointer))
        .collect();
    let second = preview_requests(&record_map, &pointers, &requested);
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].pointer.id, "child-512");
}
