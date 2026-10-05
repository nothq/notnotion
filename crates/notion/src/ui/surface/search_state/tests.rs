use chrono::{TimeZone, Utc};

use super::list::{
    grouped_recent_list_rows, list_row_index_for_result, notion_search_recency_bucket,
    notion_search_recency_bucket_from_parts, query_list_rows,
};
use super::{
    NotionSearchListRow, NotionSearchRecencyBucket, NotionSearchResultsState, NotionSearchState,
};

#[test]
fn opening_during_a_closed_preload_shows_skeleton_rows() {
    let mut state = NotionSearchState::default();
    let (preload_token, showing_cached_recents) = state.begin_recents_refresh(false);
    assert!(preload_token.is_some());
    assert!(!showing_cached_recents);

    state.begin_session("opened-session".to_string());
    let (new_token, showing_cached_recents) = state.begin_recents_refresh(true);

    assert_eq!(new_token, None);
    assert!(!showing_cached_recents);
    assert!(matches!(state.results, NotionSearchResultsState::Loading));
    assert_eq!(state.list_state.item_count(), 6);
}

#[test]
fn runtime_replacement_does_not_inherit_an_in_flight_recents_refresh() {
    let mut previous = NotionSearchState::default();
    let (old_token, _) = previous.begin_recents_refresh(false);
    let old_token = old_token.expect("previous runtime should start a refresh");

    let mut replacement = NotionSearchState::default();
    replacement.inherit_completed_recents(&previous);

    assert!(!replacement.recents_refresh_is_current(old_token));
    let (replacement_token, _) = replacement.begin_recents_refresh(false);
    assert!(replacement_token.is_some());
    assert_ne!(replacement_token, Some(old_token));
}

#[test]
fn recency_buckets_combine_local_days_with_elapsed_time() {
    let timezone =
        chrono::FixedOffset::west_opt(5 * 60 * 60).expect("test timezone offset should be valid");
    let now = timezone
        .with_ymd_and_hms(2026, 1, 15, 12, 0, 0)
        .single()
        .expect("test time should be unambiguous");
    let today = timezone
        .with_ymd_and_hms(2026, 1, 15, 0, 0, 0)
        .single()
        .expect("test time should be unambiguous");
    let yesterday = timezone
        .with_ymd_and_hms(2026, 1, 14, 0, 0, 0)
        .single()
        .expect("test time should be unambiguous");
    assert_eq!(
        notion_search_recency_bucket(today.timestamp_millis() as u64, &now),
        NotionSearchRecencyBucket::Today
    );
    assert_eq!(
        notion_search_recency_bucket(yesterday.timestamp_millis() as u64, &now),
        NotionSearchRecencyBucket::Yesterday
    );

    let fall_back_day =
        chrono::NaiveDate::from_ymd_opt(2026, 11, 1).expect("fall-back date should be valid");
    assert_eq!(
        notion_search_recency_bucket_from_parts(fall_back_day, fall_back_day, 88_200_000),
        NotionSearchRecencyBucket::PastWeek
    );

    let day_after_fall_back =
        chrono::NaiveDate::from_ymd_opt(2026, 11, 2).expect("day after fall-back should be valid");
    assert_eq!(
        notion_search_recency_bucket_from_parts(fall_back_day, day_after_fall_back, 174_600_000,),
        NotionSearchRecencyBucket::PastWeek
    );
}

#[test]
fn recency_bucket_elapsed_boundaries_match_notion() {
    let now = Utc
        .with_ymd_and_hms(2026, 9, 3, 12, 0, 0)
        .single()
        .expect("test time should be unambiguous");
    let bucket_at_elapsed = |elapsed_millis| {
        let visited_at = now - chrono::Duration::milliseconds(elapsed_millis);
        notion_search_recency_bucket(visited_at.timestamp_millis() as u64, &now)
    };

    assert_eq!(
        bucket_at_elapsed(604_800_000),
        NotionSearchRecencyBucket::PastWeek
    );
    assert_eq!(
        bucket_at_elapsed(604_800_001),
        NotionSearchRecencyBucket::PastThirtyDays
    );
    assert_eq!(
        bucket_at_elapsed(2_592_000_000),
        NotionSearchRecencyBucket::PastThirtyDays
    );
    assert_eq!(
        bucket_at_elapsed(2_592_000_001),
        NotionSearchRecencyBucket::Older
    );
}

#[test]
fn recents_are_grouped_without_empty_headers_and_keep_result_mapping() {
    let rows = grouped_recent_list_rows(&[
        NotionSearchRecencyBucket::PastThirtyDays,
        NotionSearchRecencyBucket::Today,
        NotionSearchRecencyBucket::PastWeek,
        NotionSearchRecencyBucket::Today,
    ]);

    assert_eq!(
        rows,
        vec![
            NotionSearchListRow::RecencyHeader(NotionSearchRecencyBucket::Today),
            NotionSearchListRow::RecentResult(1),
            NotionSearchListRow::RecentResult(3),
            NotionSearchListRow::RecencyHeader(NotionSearchRecencyBucket::PastWeek),
            NotionSearchListRow::RecentResult(2),
            NotionSearchListRow::RecencyHeader(NotionSearchRecencyBucket::PastThirtyDays),
            NotionSearchListRow::RecentResult(0),
        ]
    );
    assert_eq!(list_row_index_for_result(&rows, 0), Some(6));
    assert_eq!(list_row_index_for_result(&rows, 1), Some(1));
    assert_eq!(list_row_index_for_result(&rows, 2), Some(4));
    assert_eq!(list_row_index_for_result(&rows, 3), Some(2));
    assert_eq!(list_row_index_for_result(&rows, 4), None);
}

#[test]
fn query_header_is_not_a_result_index() {
    let rows = query_list_rows(3);

    assert_eq!(
        rows,
        vec![
            NotionSearchListRow::QueryHeader,
            NotionSearchListRow::QueryResult(0),
            NotionSearchListRow::QueryResult(1),
            NotionSearchListRow::QueryResult(2),
        ]
    );
    assert_eq!(list_row_index_for_result(&rows, 0), Some(1));
    assert_eq!(list_row_index_for_result(&rows, 2), Some(3));
}
