use std::sync::Mutex;

use serde_json::{json, Map, Value};

use super::super::preview::cached_record;
use super::super::test_support::{block_table, entry};
use super::super::{QuickFindRecordState, QuickFindRecords, QUICK_FIND_RECORD_LIMIT};

#[test]
fn quick_find_user_cache_accepts_a_new_unversioned_profile_snapshot() {
    let records = QuickFindRecords {
        space_id: "space-1".to_string(),
        state: Mutex::new(QuickFindRecordState::default()),
    };
    records
        .merge(Map::from_iter([(
            "notion_user".to_string(),
            json!({
                "user-1": {
                    "role": "reader",
                    "value": { "id": "user-1", "email": "old@example.com" },
                },
            }),
        )]))
        .expect("cache the original profile");
    records
        .merge(Map::from_iter([(
            "notion_user".to_string(),
            json!({
                "user-1": {
                    "role": "reader",
                    "value": { "id": "user-1", "email": "new@example.com" },
                },
            }),
        )]))
        .expect("replace the unversioned profile snapshot");

    let snapshot = records.record_map_snapshot().expect("snapshot user cache");
    assert_eq!(
        cached_record(&snapshot, "notion_user", "user-1")
            .and_then(|user| user.get("email"))
            .and_then(Value::as_str),
        Some("new@example.com")
    );
}

#[test]
fn preview_root_hydration_is_explicit_and_invalidated() {
    let records = QuickFindRecords {
        space_id: "space-1".to_string(),
        state: Mutex::new(QuickFindRecordState::default()),
    };
    records
        .merge(block_table([("root", entry("root", 1, Some(Vec::new())))]))
        .expect("cache a shallow search result");
    assert!(!records
        .preview_root_is_hydrated("root")
        .expect("read hydration state"));

    records
        .mark_preview_root_hydrated("root")
        .expect("mark the Initial root response");
    assert!(records
        .preview_root_is_hydrated("root")
        .expect("read hydration state"));

    records
        .merge(block_table([("root", entry("root", 2, Some(Vec::new())))]))
        .expect("replace the hydrated root with a newer search shell");
    assert!(!records
        .preview_root_is_hydrated("root")
        .expect("read newer root hydration state"));

    records.invalidate_root("root").expect("invalidate root");
    assert!(!records
        .preview_root_is_hydrated("root")
        .expect("read invalidated hydration state"));
}

#[test]
fn invalidation_rejects_an_in_flight_preview_root_response() {
    let records = QuickFindRecords {
        space_id: "space-1".to_string(),
        state: Mutex::new(QuickFindRecordState::default()),
    };
    let generation = records
        .preview_root_generation("root")
        .expect("capture preview generation");

    records
        .invalidate_root("root")
        .expect("invalidate preview while its request is in flight");
    let error = records
        .merge_hydrated_root_at_generation(
            block_table([("root", entry("root", 1, Some(Vec::new())))]),
            "root",
            generation,
        )
        .expect_err("reject the stale response");

    assert!(error.contains("invalidated while loading"));
    assert!(cached_record(
        &records.record_map_snapshot().expect("snapshot cache"),
        "block",
        "root",
    )
    .is_none());
    assert!(!records
        .preview_root_is_hydrated("root")
        .expect("read stale response hydration state"));
}

#[test]
fn older_navigation_enrichment_keeps_a_newer_hydrated_root() {
    let records = QuickFindRecords {
        space_id: "space-1".to_string(),
        state: Mutex::new(QuickFindRecordState::default()),
    };
    records
        .merge_hydrated_root(
            block_table([("root", entry("root", 2, Some(Vec::new())))]),
            "root",
        )
        .expect("cache the newer hydrated root");
    let generation = records
        .preview_root_generation("root")
        .expect("capture preview generation");

    records
        .merge_hydrated_root_at_generation(
            block_table([("root", entry("root", 1, Some(Vec::new())))]),
            "root",
            generation,
        )
        .expect("ignore an older navigation enrichment root");

    let snapshot = records.record_map_snapshot().expect("snapshot cache");
    assert_eq!(
        cached_record(&snapshot, "block", "root")
            .and_then(|root| root.get("version"))
            .and_then(Value::as_u64),
        Some(2)
    );
    assert!(records
        .preview_root_is_hydrated("root")
        .expect("retain newer hydration authority"));
}

#[test]
fn record_cache_is_bounded_and_root_invalidation_is_exact() {
    let records = QuickFindRecords {
        space_id: "space-1".to_string(),
        state: Mutex::new(QuickFindRecordState::default()),
    };
    let blocks = (0..=QUICK_FIND_RECORD_LIMIT)
        .map(|index| {
            let id = format!("block-{index}");
            (id.clone(), entry(&id, 1, None))
        })
        .collect();
    records
        .merge(Map::from_iter([(
            "block".to_string(),
            Value::Object(blocks),
        )]))
        .expect("cache records");
    let snapshot = records.record_map_snapshot().expect("snapshot cache");
    assert_eq!(
        snapshot
            .get("block")
            .and_then(Value::as_object)
            .expect("block table")
            .len(),
        QUICK_FIND_RECORD_LIMIT
    );

    records.invalidate_root("block-1").expect("invalidate root");
    assert!(cached_record(
        &records.record_map_snapshot().expect("snapshot cache"),
        "block",
        "block-1"
    )
    .is_none());
}

#[test]
fn preview_root_survives_descendant_merges_at_cache_capacity() {
    let records = QuickFindRecords {
        space_id: "space-1".to_string(),
        state: Mutex::new(QuickFindRecordState::default()),
    };
    let mut blocks = Map::new();
    blocks.insert("root".to_string(), entry("root", 1, None));
    for index in 1..QUICK_FIND_RECORD_LIMIT {
        let id = format!("cached-{index}");
        blocks.insert(id.clone(), entry(&id, 1, None));
    }
    records
        .merge(Map::from_iter([(
            "block".to_string(),
            Value::Object(blocks),
        )]))
        .expect("fill the Quick Find record cache");
    records
        .touch_preview_records("root", &[])
        .expect("touch the selected preview root");
    records
        .mark_preview_root_hydrated("root")
        .expect("mark the selected preview root hydrated");

    let incoming = (0..super::super::QUICK_FIND_PREVIEW_REQUEST_LIMIT)
        .map(|index| {
            let id = format!("incoming-{index}");
            (id.clone(), entry(&id, 1, None))
        })
        .collect();
    records
        .merge(Map::from_iter([(
            "block".to_string(),
            Value::Object(incoming),
        )]))
        .expect("merge one full descendant request batch");

    assert!(cached_record(
        &records.record_map_snapshot().expect("snapshot cache"),
        "block",
        "root",
    )
    .is_some());
    assert!(records
        .preview_root_is_hydrated("root")
        .expect("read retained root hydration"));
}
