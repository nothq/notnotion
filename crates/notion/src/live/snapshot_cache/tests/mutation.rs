use super::{
    notion_page_identity_key, quick_find_cache_lock, recent_page,
    NotionWorkspaceSnapshotCacheStore, QuickFindQueryCacheUpdate, QuickFindRecentsCacheWrite,
    QuickFindRecentsCacheWriter, QuickFindRecentsRefreshUpdate, RecentPageResult,
};

const COMPACT_PAGE_ID: &str = "00000000000000000000000000000004";
const DASHED_PAGE_ID: &str = "00000000-0000-0000-0000-000000000004";

fn seed_page_mutation_cache(store: &NotionWorkspaceSnapshotCacheStore) -> RecentPageResult {
    let stale = recent_page(COMPACT_PAGE_ID, "Payroll stale", 120);
    let unaffected = recent_page("unaffected", "Unaffected", 110);
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: &[stale.clone(), unaffected.clone()],
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 100,
            refresh_started_at_unix_millis: 100,
            refresh_committed_at_unix_millis: 110,
        },
    );
    store.persist_quick_find_recents("space-a", std::slice::from_ref(&stale));
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|payroll",
            results: &[stale.page.clone(), unaffected.page.clone()],
            response_is_complete: true,
            query_authority_version: 101,
            accepted_at_unix_millis: 120,
        },
    );
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "title_only|unrelated",
            results: std::slice::from_ref(&unaffected.page),
            response_is_complete: true,
            query_authority_version: 102,
            accepted_at_unix_millis: 130,
        },
    );
    unaffected
}

fn assert_page_mutation_cache(
    store: &NotionWorkspaceSnapshotCacheStore,
    unaffected: RecentPageResult,
) {
    assert_eq!(
        store
            .load_quick_find_recents("space-a")
            .expect("load mutation-invalidated recents"),
        Some(vec![unaffected.clone()])
    );
    let persisted = store
        .load_quick_find_local_search_with_authority("space-a")
        .expect("load mutation-invalidated local index")
        .expect("mutation invalidation should retain the cached space");
    assert_eq!(persisted.authority_version, 200);
    assert!(persisted.local_search.indexed_queries.is_empty());
    assert_eq!(persisted.local_search.pages, vec![unaffected.page]);

    let cache = store.read_quick_find_cache_for_write(&store.quick_find_path());
    let space = cache
        .spaces
        .into_iter()
        .find(|space| space.space_id == "space-a")
        .expect("mutation-invalidated space");
    assert_eq!(space.refreshed_at_unix_millis, 0);
    assert!(space.query_authority_version_by_key.is_empty());
    assert!(space
        .local_visits
        .iter()
        .all(|visit| notion_page_identity_key(&visit.page_id) != COMPACT_PAGE_ID));
}

#[test]
fn page_mutation_clears_persisted_authority_and_stale_page_details() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-mutation-cache-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x27; 32],
        active_user_id: "user-1".to_string(),
    };
    let unaffected = seed_page_mutation_cache(&store);
    store.persist_quick_find_page_mutation("space-a", &[DASHED_PAGE_ID.to_string()], 200, 140);
    assert_page_mutation_cache(&store, unaffected);
    std::fs::remove_dir_all(&cache_dir).expect("remove Quick Find mutation cache fixture");
}

#[test]
fn recents_cache_writer_never_blocks_result_delivery_on_durable_io() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-recents-writer-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x2e; 32],
        active_user_id: "user-1".to_string(),
    };
    let writer = QuickFindRecentsCacheWriter::start(store.clone(), "space-a".to_string())
        .expect("start recents-cache writer");
    let recent = recent_page("recent", "Recent", 100);
    let durable_io_guard = quick_find_cache_lock()
        .lock()
        .expect("hold durable Quick Find cache lock");
    let (staged, stage_completed) = std::sync::mpsc::sync_channel(1);
    let stage_thread = std::thread::spawn(move || {
        writer.stage(QuickFindRecentsCacheWrite {
            results: vec![recent],
            concurrent_visits: Vec::new(),
            authority_version: 100,
            refresh_started_at_unix_millis: 100,
            refresh_committed_at_unix_millis: 110,
        });
        staged.send(()).expect("report non-blocking stage");
    });
    stage_completed
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("staging must return while durable I/O is blocked");
    drop(durable_io_guard);
    stage_thread
        .join()
        .expect("join recents-cache stage thread");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let persisted = loop {
        if let Some(persisted) = store
            .load_quick_find_recents("space-a")
            .expect("load asynchronously persisted recents")
        {
            break persisted;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "background recents-cache write should complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].page.block_id, "recent");

    std::fs::remove_dir_all(&cache_dir).expect("remove Quick Find recents writer fixture");
}
