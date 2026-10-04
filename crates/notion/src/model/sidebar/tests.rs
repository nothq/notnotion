use super::{
    LoadRecentPagesResult, PageShellEditedAt, PageShellIcon, PageShellSearchBadge,
    PageShellSearchResult, QuickFindLocalSearchCache, RecentPageResult, RecentPageVisit,
};

fn search_result() -> PageShellSearchResult {
    PageShellSearchResult {
        block_id: "page-00".to_string(),
        title: "Page 0".to_string(),
        icon: PageShellIcon::named("page"),
        target_board_url: "https://www.notion.so/page-00".to_string(),
        highlight: None,
        match_snippet: None,
        editor_display_name: None,
        edited_label: None,
        edited_at: None,
        badges: Vec::new(),
    }
}

#[test]
fn recent_page_result_conversions_preserve_order_and_cap_boosts() {
    let result = LoadRecentPagesResult {
        results: (0..52)
            .map(|index| RecentPageResult {
                page: PageShellSearchResult {
                    block_id: format!("page-{index:02}"),
                    title: format!("Page {index}"),
                    target_board_url: format!("https://www.notion.so/page-{index:02}"),
                    ..search_result()
                },
                visited_at_unix_millis: 1_000 - index,
            })
            .collect(),
    };

    let boosts = result.recent_pages_for_boosting();
    assert_eq!(boosts.len(), 50);
    assert_eq!(
        boosts.first(),
        Some(&RecentPageVisit {
            page_id: "page-00".to_string(),
            visited_at_unix_millis: 1_000,
        })
    );
    assert_eq!(
        boosts.last(),
        Some(&RecentPageVisit {
            page_id: "page-49".to_string(),
            visited_at_unix_millis: 951,
        })
    );

    let pages = result.into_pages();
    assert_eq!(pages.len(), 52);
    assert_eq!(pages[0].block_id, "page-00");
    assert_eq!(pages[51].block_id, "page-51");
}

#[test]
fn legacy_search_result_json_defaults_new_row_metadata() {
    let serialized = serde_json::to_value(search_result()).expect("serialize search result");
    assert!(serialized.get("match_snippet").is_none());
    assert!(serialized.get("editor_display_name").is_none());
    assert!(serialized.get("edited_label").is_none());
    assert!(serialized.get("edited_at").is_none());
    assert!(serialized.get("badges").is_none());

    let decoded = serde_json::from_value::<PageShellSearchResult>(serialized)
        .expect("deserialize legacy search result");
    assert_eq!(decoded.match_snippet, None);
    assert_eq!(decoded.editor_display_name, None);
    assert_eq!(decoded.edited_label, None);
    assert_eq!(decoded.edited_at, None);
    assert!(decoded.badges.is_empty());
}

#[test]
fn search_result_edited_timestamp_uses_stable_serde_shape() {
    let serialized = serde_json::to_value(PageShellSearchResult {
        edited_at: Some(PageShellEditedAt {
            unix_millis: 1_725_000_000_000,
            date_label: "Aug 29".to_string(),
        }),
        ..search_result()
    })
    .expect("serialize search result edited timestamp");

    assert_eq!(
        serialized.get("edited_at"),
        Some(&serde_json::json!({
            "unix_millis": 1_725_000_000_000_u64,
            "date_label": "Aug 29",
        }))
    );
    let decoded = serde_json::from_value::<PageShellSearchResult>(serialized)
        .expect("deserialize search result edited timestamp");
    assert_eq!(
        decoded.edited_at,
        Some(PageShellEditedAt {
            unix_millis: 1_725_000_000_000,
            date_label: "Aug 29".to_string(),
        })
    );
}

#[test]
fn search_result_badges_use_stable_serde_names() {
    let serialized = serde_json::to_value(PageShellSearchResult {
        badges: vec![
            PageShellSearchBadge::CurrentPage,
            PageShellSearchBadge::Database,
        ],
        ..search_result()
    })
    .expect("serialize search badges");

    assert_eq!(
        serialized.get("badges"),
        Some(&serde_json::json!(["current_page", "database"]))
    );
}

#[test]
fn recent_page_merge_deduplicates_compact_and_dashed_uuid_forms() {
    let dashed = "00000000-0000-0000-0000-000000000004";
    let compact = "00000000000000000000000000000004";
    let cached = RecentPageResult {
        page: PageShellSearchResult {
            block_id: compact.to_string(),
            title: "Older".to_string(),
            ..search_result()
        },
        visited_at_unix_millis: 10,
    };
    let incoming = RecentPageResult {
        page: PageShellSearchResult {
            block_id: dashed.to_string(),
            title: "Newer".to_string(),
            ..search_result()
        },
        visited_at_unix_millis: 20,
    };

    let merged = LoadRecentPagesResult::merge_results(vec![cached], vec![incoming.clone()]);

    assert_eq!(merged, vec![incoming]);
}

fn authoritative_alias_cache() -> QuickFindLocalSearchCache {
    let compact = "00000000000000000000000000000004";
    let dashed = "00000000-0000-0000-0000-000000000004";
    let mut cache = QuickFindLocalSearchCache::default();
    cache.add_provisional_pages([PageShellSearchResult {
        block_id: compact.to_string(),
        title: "Cached title".to_string(),
        match_snippet: Some("query-specific snippet".to_string()),
        badges: vec![
            PageShellSearchBadge::CurrentPage,
            PageShellSearchBadge::Database,
        ],
        ..search_result()
    }]);
    cache.record_complete_query_results(
        " All_Content|AcMe ",
        [PageShellSearchResult {
            block_id: dashed.to_string(),
            title: "Authoritative title".to_string(),
            match_snippet: Some("another query-specific snippet".to_string()),
            badges: vec![
                PageShellSearchBadge::Database,
                PageShellSearchBadge::CurrentPage,
            ],
            ..search_result()
        }],
    );

    assert_eq!(cache.pages.len(), 1);
    assert_eq!(cache.pages[0].block_id, dashed);
    assert_eq!(cache.pages[0].title, "Authoritative title");
    assert_eq!(cache.pages[0].match_snippet, None);
    assert_eq!(cache.pages[0].badges, vec![PageShellSearchBadge::Database]);
    assert!(cache.query_is_indexed("all_content|acme"));
    assert_eq!(
        cache.indexed_result_ids("all_content|acme"),
        Some([compact.to_string()].as_slice())
    );
    cache
}

fn assert_query_membership_updates(cache: &mut QuickFindLocalSearchCache) {
    cache.record_partial_query_results(
        "all_content|acme",
        [PageShellSearchResult {
            block_id: "verified-partial".to_string(),
            title: "Acme partial result".to_string(),
            ..search_result()
        }],
    );
    assert_eq!(
        cache.indexed_result_ids("all_content|acme"),
        Some([compact_id(), "verified-partial".to_string()].as_slice())
    );

    cache.merge_query_results([PageShellSearchResult {
        block_id: "stale-overlap".to_string(),
        title: "Acme stale overlap".to_string(),
        ..search_result()
    }]);
    cache.record_complete_query_results(
        "all_content|acme",
        [PageShellSearchResult {
            block_id: "authoritative-only".to_string(),
            title: "Acme authoritative".to_string(),
            ..search_result()
        }],
    );
    assert_eq!(
        cache.indexed_result_ids("all_content|acme"),
        Some(["authoritative-only".to_string()].as_slice())
    );
    assert!(cache
        .pages
        .iter()
        .any(|page| page.block_id == "stale-overlap"));
}

fn compact_id() -> String {
    "00000000000000000000000000000004".to_string()
}

fn assert_query_authority_invalidation(cache: &mut QuickFindLocalSearchCache) {
    cache.invalidate_query_authority([PageShellSearchResult {
        block_id: "fresh-recent".to_string(),
        title: "Fresh recent".to_string(),
        ..search_result()
    }]);
    assert!(cache.indexed_queries.is_empty());
    assert!(cache
        .pages
        .iter()
        .any(|page| page.block_id == "authoritative-only"));
    assert!(cache
        .pages
        .iter()
        .any(|page| page.block_id == "fresh-recent"));
}

fn assert_local_search_cache_bounds(cache: &mut QuickFindLocalSearchCache) {
    for index in 0..QuickFindLocalSearchCache::MAX_INDEXED_QUERIES + 2 {
        cache.record_complete_query_results(&format!("all_content|query-{index}"), []);
    }
    cache.record_complete_query_results(
        "all_content|bulk",
        (0..QuickFindLocalSearchCache::MAX_PAGES + 2).map(|index| PageShellSearchResult {
            block_id: format!("bulk-page-{index}"),
            title: format!("Bulk page {index}"),
            ..search_result()
        }),
    );

    assert_eq!(
        cache.indexed_queries.len(),
        QuickFindLocalSearchCache::MAX_INDEXED_QUERIES
    );
    assert!(cache.query_is_indexed("all_content|bulk"));
    assert!(!cache.query_is_indexed("all_content|acme"));
    assert_eq!(cache.pages.len(), QuickFindLocalSearchCache::MAX_PAGES);
    assert_eq!(
        cache.pages.first().map(|page| page.block_id.as_str()),
        Some("bulk-page-0")
    );
    assert_eq!(
        cache.pages.last().map(|page| page.block_id.as_str()),
        Some("bulk-page-511")
    );
}

#[test]
fn local_search_cache_deduplicates_sanitizes_and_bounds_persisted_state() {
    let mut cache = authoritative_alias_cache();
    assert_query_membership_updates(&mut cache);
    assert_query_authority_invalidation(&mut cache);
    assert_local_search_cache_bounds(&mut cache);
}

#[test]
fn page_mutation_drops_identity_aliases_and_revokes_every_indexed_query() {
    let compact = "00000000000000000000000000000004";
    let dashed = "00000000-0000-0000-0000-000000000004";
    let unaffected = PageShellSearchResult {
        block_id: "unaffected".to_string(),
        title: "Unaffected".to_string(),
        ..search_result()
    };
    let mut cache = QuickFindLocalSearchCache::default();
    cache.record_complete_query_results(
        "all_content|payroll",
        [
            PageShellSearchResult {
                block_id: compact.to_string(),
                title: "Payroll".to_string(),
                ..search_result()
            },
            unaffected.clone(),
        ],
    );
    cache.record_complete_query_results("title_only|unrelated", [unaffected.clone()]);

    cache.invalidate_page_mutation(dashed);

    assert!(cache.indexed_queries.is_empty());
    assert_eq!(cache.pages, vec![unaffected]);
}
