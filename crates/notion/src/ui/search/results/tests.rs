use super::query_text::{
    notion_search_query_match_ranges, notion_search_query_result_height,
    notion_search_query_result_metadata, notion_search_query_result_stride,
    notion_search_query_snippet_layout, notion_search_result_edited_label,
    notion_search_should_paginate,
};
use super::row::{
    notion_search_loading_skeleton_row_width, notion_search_result_badge_label,
    notion_search_result_icon_artwork,
};
use super::{
    NotionSearchQuerySnippetLayout, NotionSearchResultIconArtwork,
    NOTION_SEARCH_LOADING_SKELETON_ROW_COUNT, NOTION_SEARCH_LOADING_SKELETON_ROW_GAP,
    NOTION_SEARCH_LOADING_SKELETON_ROW_HEIGHT, NOTION_SEARCH_QUERY_MATCH_COLOR,
    NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH, NOTION_SEARCH_RECENT_TITLE_MAX_WIDTH,
};
use crate::model::{PageShellEditedAt, PageShellIcon, PageShellSearchBadge};

#[test]
fn query_result_metadata_uses_notion_separator_order() {
    assert_eq!(
        notion_search_query_result_metadata(
            Some("General / … / Team"),
            Some("Helper Bot"),
            Some("Edited Aug 20"),
        )
        .as_deref(),
        Some("General / … / Team · Helper Bot · Edited Aug 20")
    );
    assert_eq!(
        notion_search_query_result_metadata(None, Some("Helper Bot"), Some("Edited Aug 20"))
            .as_deref(),
        Some("Helper Bot · Edited Aug 20")
    );
    assert_eq!(notion_search_query_result_metadata(None, None, None), None);
}

#[test]
fn query_result_edited_label_recomputes_relative_time_from_cached_timestamp() {
    let edited_at = PageShellEditedAt {
        unix_millis: 10_000_000,
        date_label: "Jan 1".to_string(),
    };

    assert_eq!(
        notion_search_result_edited_label(Some(&edited_at), None, 10_000_000),
        Some("Edited 1h ago".to_string())
    );
    assert_eq!(
        notion_search_result_edited_label(
            Some(&edited_at),
            Some("Edited stale"),
            10_000_000 + 23 * 3_600_000,
        ),
        Some("Edited 23h ago".to_string())
    );
    assert_eq!(
        notion_search_result_edited_label(Some(&edited_at), None, 10_000_000 + 24 * 3_600_000,),
        Some("Edited 1d ago".to_string())
    );
    assert_eq!(
        notion_search_result_edited_label(Some(&edited_at), None, 10_000_000 + 6 * 24 * 3_600_000,),
        Some("Edited 6d ago".to_string())
    );
    assert_eq!(
        notion_search_result_edited_label(Some(&edited_at), None, 10_000_000 + 7 * 24 * 3_600_000,),
        Some("Edited Jan 1".to_string())
    );
    assert_eq!(
        notion_search_result_edited_label(None, Some("Edited legacy"), 10_000_000),
        Some("Edited legacy".to_string())
    );
}

#[test]
fn query_result_height_tracks_wrapped_snippet_lines() {
    assert_eq!(
        notion_search_query_result_height(NotionSearchQuerySnippetLayout::None),
        56.0
    );
    assert_eq!(
        notion_search_query_result_height(NotionSearchQuerySnippetLayout::OneLine),
        76.0
    );
    assert_eq!(
        notion_search_query_result_height(NotionSearchQuerySnippetLayout::TwoLines),
        93.0
    );
    assert_eq!(
        notion_search_query_result_stride(NotionSearchQuerySnippetLayout::None),
        56.0
    );
    assert_eq!(
        notion_search_query_result_stride(NotionSearchQuerySnippetLayout::OneLine),
        76.0
    );
    assert_eq!(
        notion_search_query_result_stride(NotionSearchQuerySnippetLayout::TwoLines),
        94.0
    );
    assert_eq!(
        notion_search_query_snippet_layout("one line", 528.0),
        NotionSearchQuerySnippetLayout::OneLine
    );
    assert_eq!(
        notion_search_query_snippet_layout("wrapped", 528.01),
        NotionSearchQuerySnippetLayout::TwoLines
    );
    assert_eq!(
        notion_search_query_snippet_layout("forced\nline", 20.0),
        NotionSearchQuerySnippetLayout::TwoLines
    );
}

#[test]
fn pagination_only_arms_near_the_virtual_list_tail() {
    assert!(!notion_search_should_paginate(0, 0));
    assert!(!notion_search_should_paginate(17, 21));
    assert!(notion_search_should_paginate(18, 21));
    assert!(notion_search_should_paginate(21, 21));
}

#[test]
fn recent_title_cap_preserves_hierarchy_space() {
    assert_eq!(NOTION_SEARCH_RECENT_TITLE_MAX_WIDTH, 380.0);
    assert_eq!(
        NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH - NOTION_SEARCH_RECENT_TITLE_MAX_WIDTH,
        148.0
    );
}

#[test]
fn query_match_ranges_are_case_insensitive_unicode_byte_ranges() {
    let value = "Café ACME café";
    let ranges = notion_search_query_match_ranges(value, "CAFÉ");

    assert_eq!(ranges, vec![0..5, 11..16]);
    assert_eq!(
        ranges
            .iter()
            .map(|range| &value[range.clone()])
            .collect::<Vec<_>>(),
        vec!["Café", "café"]
    );
}

#[test]
fn query_result_badges_use_reference_labels() {
    assert_eq!(
        notion_search_result_badge_label(PageShellSearchBadge::CurrentPage),
        ("Current Page", "current-page")
    );
    assert_eq!(
        notion_search_result_badge_label(PageShellSearchBadge::Database),
        ("Database", "database")
    );
}

#[test]
fn query_match_color_uses_the_live_reference_accent() {
    assert_eq!(NOTION_SEARCH_QUERY_MATCH_COLOR, 0x2783de);
}

#[test]
fn database_badge_forces_table_artwork_before_icon_fallbacks() {
    let database_badge = [PageShellSearchBadge::Database];
    assert_eq!(
        notion_search_result_icon_artwork(&database_badge, &PageShellIcon::named("database")),
        NotionSearchResultIconArtwork::Database
    );
    assert_eq!(
        notion_search_result_icon_artwork(&database_badge, &PageShellIcon::emoji("📊")),
        NotionSearchResultIconArtwork::Database
    );

    assert_eq!(
        notion_search_result_icon_artwork(&[], &PageShellIcon::named("page")),
        NotionSearchResultIconArtwork::DefaultPage
    );
    assert_eq!(
        notion_search_result_icon_artwork(&[], &PageShellIcon::emoji("📊")),
        NotionSearchResultIconArtwork::Explicit
    );
    assert_eq!(
        notion_search_result_icon_artwork(
            &[],
            &PageShellIcon {
                kind: "custom".to_string(),
                value: "notion://custom_emoji/00000000-0000-0000-0000-000000000001/00000000-0000-0000-0000-000000000002".to_string(),
                render_url: None,
            },
        ),
        NotionSearchResultIconArtwork::Explicit
    );
}

#[test]
fn loading_skeleton_preserves_reference_bar_geometry() {
    assert_eq!(NOTION_SEARCH_LOADING_SKELETON_ROW_COUNT, 6);
    assert_eq!(NOTION_SEARCH_LOADING_SKELETON_ROW_HEIGHT, 12.0);
    assert_eq!(NOTION_SEARCH_LOADING_SKELETON_ROW_GAP, 14.0);
    assert_eq!(
        (0..NOTION_SEARCH_LOADING_SKELETON_ROW_COUNT)
            .map(notion_search_loading_skeleton_row_width)
            .collect::<Vec<_>>(),
        vec![280.0, 220.0, 280.0, 220.0, 280.0, 220.0]
    );
}
