use super::{
    recent_page, HashMap, NotionWorkspaceSnapshotCacheStore, QuickFindLocalSearchCache,
    QuickFindQueryCacheUpdate, QuickFindRecentsRefreshUpdate, RecentPageResult,
};

fn local_page_ids(cache: &QuickFindLocalSearchCache) -> Vec<&str> {
    cache
        .pages
        .iter()
        .map(|page| page.block_id.as_str())
        .collect()
}

fn seed_stale_refresh_query(
    store: &NotionWorkspaceSnapshotCacheStore,
    stale_visit: &RecentPageResult,
) {
    let mut stale_local_search = QuickFindLocalSearchCache::default();
    stale_local_search
        .record_complete_query_results("all_content|stale", [stale_visit.page.clone()]);
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|stale",
            results: &stale_local_search.pages,
            response_is_complete: true,
            query_authority_version: 1,
            accepted_at_unix_millis: 150,
        },
    );
}

fn commit_initial_refresh(
    store: &NotionWorkspaceSnapshotCacheStore,
    authoritative: &RecentPageResult,
    future_remote: &RecentPageResult,
    stale_visit: &RecentPageResult,
    concurrent_visit: &RecentPageResult,
) {
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: &[authoritative.clone(), future_remote.clone()],
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 2,
            refresh_started_at_unix_millis: 200,
            refresh_committed_at_unix_millis: 220,
        },
    );
    store.persist_quick_find_recents("space-a", std::slice::from_ref(stale_visit));
    assert_eq!(
        store
            .load_quick_find_recents("space-a")
            .expect("read refreshed space"),
        Some(vec![future_remote.clone(), authoritative.clone()])
    );
    let local_search = store
        .load_quick_find_local_search("space-a")
        .expect("read refreshed local index")
        .expect("refreshed local index should exist");
    assert!(local_search.indexed_queries.is_empty());
    assert_eq!(
        local_page_ids(&local_search),
        vec!["future-remote", "authoritative", "stale"]
    );

    store.persist_quick_find_recents("space-a", std::slice::from_ref(concurrent_visit));
    assert_eq!(
        store
            .load_quick_find_recents("space-a")
            .expect("read space with concurrent visit"),
        Some(vec![
            future_remote.clone(),
            concurrent_visit.clone(),
            authoritative.clone(),
        ])
    );
}

fn commit_newer_refresh(store: &NotionWorkspaceSnapshotCacheStore) {
    let newer = recent_page("newer", "Newer refresh", 50);
    let delayed = recent_page("delayed", "Delayed refresh", 500);
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: std::slice::from_ref(&newer),
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 3,
            refresh_started_at_unix_millis: 300,
            refresh_committed_at_unix_millis: 320,
        },
    );
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: std::slice::from_ref(&delayed),
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 2,
            refresh_started_at_unix_millis: 200,
            refresh_committed_at_unix_millis: 400,
        },
    );
    assert_eq!(
        store
            .load_quick_find_recents("space-a")
            .expect("read space after delayed refresh"),
        Some(vec![newer])
    );
    let local_search = store
        .load_quick_find_local_search("space-a")
        .expect("read final local index")
        .expect("final local index should exist");
    assert!(local_search.indexed_queries.is_empty());
    assert_eq!(
        local_page_ids(&local_search),
        vec!["newer", "future-remote", "authoritative", "stale"]
    );
}

fn assert_stale_query_rejected(store: &NotionWorkspaceSnapshotCacheStore) {
    let mut stale_query = QuickFindLocalSearchCache::default();
    stale_query.record_complete_query_results(
        "all_content|stale-query",
        [recent_page("stale-query", "Stale query", 500).page],
    );
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|stale-query",
            results: &stale_query.pages,
            response_is_complete: true,
            query_authority_version: 2,
            accepted_at_unix_millis: 500,
        },
    );
    let local_search = store
        .load_quick_find_local_search("space-a")
        .expect("read index after stale query write")
        .expect("index after stale query write should exist");
    assert!(local_search.indexed_queries.is_empty());
    assert_eq!(
        local_page_ids(&local_search),
        vec!["newer", "future-remote", "authoritative", "stale"]
    );
}

fn assert_crossing_refresh_query_rejected(store: &NotionWorkspaceSnapshotCacheStore) {
    let crossing_refresh = recent_page("crossing-refresh", "Crossing refresh", 500).page;
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|crossing-refresh",
            results: std::slice::from_ref(&crossing_refresh),
            response_is_complete: true,
            query_authority_version: 2,
            accepted_at_unix_millis: 350,
        },
    );
    let local_search = store
        .load_quick_find_local_search("space-a")
        .expect("read index after crossing-refresh query write")
        .expect("index after crossing-refresh query write should exist");
    assert!(!local_search.query_is_indexed("all_content|crossing-refresh"));
    assert_eq!(
        local_page_ids(&local_search),
        vec!["newer", "future-remote", "authoritative", "stale"]
    );
}

fn assert_current_query_accepted(store: &NotionWorkspaceSnapshotCacheStore) {
    let mut current_query = QuickFindLocalSearchCache::default();
    current_query.record_complete_query_results(
        "all_content|current-query",
        [recent_page("current-query", "Current query", 500).page],
    );
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|current-query",
            results: &current_query.pages,
            response_is_complete: true,
            query_authority_version: 3,
            accepted_at_unix_millis: 360,
        },
    );
    let local_search = store
        .load_quick_find_local_search("space-a")
        .expect("read index after current query write")
        .expect("index after current query write should exist");
    assert!(local_search.query_is_indexed("all_content|current-query"));
    assert_eq!(
        local_page_ids(&local_search),
        vec![
            "current-query",
            "newer",
            "future-remote",
            "authoritative",
            "stale",
        ]
    );
}

#[test]
fn authoritative_quick_find_refresh_invalidates_exclusions_without_discarding_search_pages() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-refresh-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x4c; 32],
        active_user_id: "user-1".to_string(),
    };
    let authoritative = recent_page("authoritative", "Authoritative", 100);
    let future_remote = recent_page("future-remote", "Future remote", 10_000);
    let stale_visit = recent_page("stale", "Stale", 150);
    let concurrent_visit = recent_page("concurrent", "Concurrent", 250);

    seed_stale_refresh_query(&store, &stale_visit);
    commit_initial_refresh(
        &store,
        &authoritative,
        &future_remote,
        &stale_visit,
        &concurrent_visit,
    );
    commit_newer_refresh(&store);
    assert_stale_query_rejected(&store);
    assert_crossing_refresh_query_rejected(&store);
    assert_current_query_accepted(&store);

    std::fs::remove_dir_all(&cache_dir).expect("remove Quick Find cache fixture");
}

#[test]
fn newer_recents_authority_wins_when_refresh_timestamps_tie() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-refresh-tie-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x57; 32],
        active_user_id: "user-1".to_string(),
    };
    let older = recent_page("older", "Older refresh", 100);
    let newer = recent_page("newer", "Newer refresh", 200);

    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: std::slice::from_ref(&older),
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 100,
            refresh_started_at_unix_millis: 500,
            refresh_committed_at_unix_millis: 510,
        },
    );
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: std::slice::from_ref(&newer),
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 101,
            refresh_started_at_unix_millis: 500,
            refresh_committed_at_unix_millis: 520,
        },
    );
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: std::slice::from_ref(&older),
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 100,
            refresh_started_at_unix_millis: 500,
            refresh_committed_at_unix_millis: 530,
        },
    );

    assert_eq!(
        store
            .load_quick_find_recents("space-a")
            .expect("read recents after tied refresh timestamps"),
        Some(vec![newer])
    );
    let persisted = store.read_quick_find_cache_for_write(&store.quick_find_path());
    let space = persisted
        .spaces
        .into_iter()
        .find(|space| space.space_id == "space-a")
        .expect("tied-timestamp space should be persisted");
    assert_eq!(space.authority_version, 101);

    std::fs::remove_dir_all(&cache_dir).expect("remove refresh-tie cache fixture");
}

fn persist_cross_writer_order(store: &NotionWorkspaceSnapshotCacheStore) {
    let stale_query_page = recent_page("stale-query", "Stale query", 150).page;
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|stale-query",
            results: std::slice::from_ref(&stale_query_page),
            response_is_complete: true,
            query_authority_version: 1,
            accepted_at_unix_millis: 150,
        },
    );

    // Simulate a query writer reaching disk before the refresh writer which established its
    // authority generation. Wall-clock timestamps may tie, so the delayed durable write must
    // use causal generation instead of time to preserve the newer query membership.
    let current_query_page = recent_page("current-query", "Current query", 340).page;
    store.persist_quick_find_query_cache(
        "space-a",
        QuickFindQueryCacheUpdate {
            query_key: "all_content|current-query",
            results: std::slice::from_ref(&current_query_page),
            response_is_complete: true,
            query_authority_version: 2,
            accepted_at_unix_millis: 340,
        },
    );
    let refreshed = recent_page("refreshed", "Refreshed", 200);
    store.persist_quick_find_recents_refresh(
        "space-a",
        QuickFindRecentsRefreshUpdate {
            results: std::slice::from_ref(&refreshed),
            in_memory_concurrent_visits: &[],
            refresh_authority_version: 2,
            refresh_started_at_unix_millis: 200,
            refresh_committed_at_unix_millis: 300,
        },
    );
}

fn assert_cross_writer_order(store: &NotionWorkspaceSnapshotCacheStore) {
    let local_search = store
        .load_quick_find_local_search("space-a")
        .expect("read index after cross-writer ordering")
        .expect("cross-writer index should exist");
    assert!(!local_search.query_is_indexed("all_content|stale-query"));
    assert!(local_search.query_is_indexed("all_content|current-query"));
    assert_eq!(
        local_page_ids(&local_search),
        vec!["current-query", "refreshed", "stale-query"]
    );

    let persisted = store.read_quick_find_cache_for_write(&store.quick_find_path());
    let space = persisted
        .spaces
        .into_iter()
        .find(|space| space.space_id == "space-a")
        .expect("cross-writer space should be persisted");
    assert_eq!(space.local_search_updated_at_unix_millis, 340);
    assert_eq!(
        space.query_authority_version_by_key,
        HashMap::from([("all_content|current-query".to_string(), 2)])
    );
}

#[test]
fn delayed_recents_writer_preserves_same_generation_query_authority() {
    let cache_dir = std::env::temp_dir().join(format!(
        "acme-notion-quick-find-cross-writer-test-{}",
        uuid::Uuid::new_v4()
    ));
    let store = NotionWorkspaceSnapshotCacheStore {
        path: cache_dir.join("workspace.bin"),
        key: [0x71; 32],
        active_user_id: "user-1".to_string(),
    };
    persist_cross_writer_order(&store);
    assert_cross_writer_order(&store);
    std::fs::remove_dir_all(&cache_dir).expect("remove cross-writer cache fixture");
}
