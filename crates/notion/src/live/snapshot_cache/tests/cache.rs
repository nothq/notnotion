use super::{
    quick_find_cache_lock, recent_page, NotionQuickFindCache, NotionWorkspaceSnapshotCacheStore,
    QuickFindLocalSearchCache, QuickFindQueryCacheUpdate, QuickFindQueryCacheWrite,
    QuickFindQueryCacheWriter, NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION,
};

#[test]
fn quick_find_cache_decodes_spaces_written_before_local_search_timestamp() {
    let cache: NotionQuickFindCache = serde_json::from_value(serde_json::json!({
        "schema_version": NOTION_QUICK_FIND_CACHE_SCHEMA_VERSION,
        "active_user_id": "user-1",
        "spaces": [{
            "space_id": "space-1",
            "refreshed_at_unix_millis": 100,
            "local_visits": [],
            "results": [],
        }],
    }))
    .expect("decode the previous Quick Find cache shape");

    assert_eq!(cache.spaces[0].local_search_updated_at_unix_millis, 0);
}

#[test]
fn quick_find_cache_is_encrypted_and_partitioned_by_space() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-cache-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x5a; 32],
        active_user_id: "user-1".to_string(),
    };
    let first = recent_page("page-1", "Sensitive Quick Find title", 100);
    let second = recent_page("page-2", "Second page", 200);
    let mut local_search = QuickFindLocalSearchCache::default();
    local_search.record_complete_query_results(
        "all_content|sensitive-index-query",
        [recent_page("indexed-page", "Sensitive indexed title", 300).page],
    );

    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|sensitive-index-query",
            results: &local_search.pages,
            response_is_complete: true,
            query_authority_version: 100,
            accepted_at_unix_millis: 110,
        },
    );
    store.persist_quick_find_recents("space-a", std::slice::from_ref(&first));
    store.persist_quick_find_recents("space-b", std::slice::from_ref(&second));

    assert_eq!(
        store
            .load_quick_find_recents("space-a")
            .expect("read space A"),
        Some(vec![first])
    );
    assert_eq!(
        store
            .load_quick_find_recents("space-b")
            .expect("read space B"),
        Some(vec![second])
    );
    assert_eq!(
        store
            .load_quick_find_local_search("space-a")
            .expect("read space A local index"),
        Some(local_search)
    );
    let ciphertext = std::fs::read(store.quick_find_path()).expect("read encrypted cache");
    assert!(!ciphertext
        .windows("Sensitive Quick Find title".len())
        .any(|window| window == b"Sensitive Quick Find title"));
    assert!(!ciphertext
        .windows("Sensitive indexed title".len())
        .any(|window| window == b"Sensitive indexed title"));
    assert!(!ciphertext
        .windows("sensitive-index-query".len())
        .any(|window| window == b"sensitive-index-query"));

    std::fs::remove_dir_all(&cache_dir).expect("remove Quick Find cache fixture");
}

#[test]
fn query_cache_writer_never_blocks_result_delivery_on_durable_io() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-writer-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x3d; 32],
        active_user_id: "user-1".to_string(),
    };
    let writer = QuickFindQueryCacheWriter::start(store.clone(), "space-a".to_string())
        .expect("start query-cache writer");
    let payroll = recent_page("payroll", "Payroll", 100).page;
    let durable_io_guard = quick_find_cache_lock()
        .lock()
        .expect("hold durable Quick Find cache lock");
    let (staged, stage_completed) = std::sync::mpsc::sync_channel(1);
    let stage_thread = std::thread::spawn(move || {
        writer.stage(QuickFindQueryCacheWrite {
            query_key: "all_content|payroll".to_string(),
            results: vec![payroll],
            response_is_complete: false,
            authority_version: 100,
            accepted_at_unix_millis: 110,
        });
        staged.send(()).expect("report non-blocking stage");
    });
    stage_completed
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("staging must return while durable I/O is blocked");
    drop(durable_io_guard);
    stage_thread.join().expect("join query-cache stage thread");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let persisted = loop {
        if let Some(persisted) = store
            .load_quick_find_local_search("space-a")
            .expect("load asynchronously persisted query cache")
        {
            break persisted;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "background query-cache write should complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert!(persisted.query_is_indexed("all_content|payroll"));

    std::fs::remove_dir_all(&cache_dir).expect("remove Quick Find writer fixture");
}

#[test]
fn query_cache_writer_preserves_distinct_deltas_queued_behind_durable_io() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-writer-queue-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x6f; 32],
        active_user_id: "user-1".to_string(),
    };
    let writer = QuickFindQueryCacheWriter::start(store.clone(), "space-a".to_string())
        .expect("start query-cache writer");
    let durable_io_guard = quick_find_cache_lock()
        .lock()
        .expect("hold durable Quick Find cache lock");

    for (query, page_id, authority_version, accepted_at) in [
        ("alpha", "page-a", 100, 110),
        ("bravo", "page-b", 120, 130),
        ("charlie", "page-c", 140, 150),
    ] {
        writer.stage(QuickFindQueryCacheWrite {
            query_key: format!("all_content|{query}"),
            results: vec![recent_page(page_id, query, accepted_at).page],
            response_is_complete: true,
            authority_version,
            accepted_at_unix_millis: accepted_at,
        });
    }
    drop(durable_io_guard);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if store
            .load_quick_find_local_search("space-a")
            .expect("load asynchronously persisted query deltas")
            .is_some_and(|persisted| {
                ["alpha", "bravo", "charlie"]
                    .into_iter()
                    .all(|query| persisted.query_is_indexed(&format!("all_content|{query}")))
            })
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "every accepted query delta must survive coalesced writer wakeups"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    std::fs::remove_dir_all(&cache_dir).expect("remove Quick Find writer queue fixture");
}
