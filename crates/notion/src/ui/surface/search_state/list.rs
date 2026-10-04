use chrono::{DateTime, TimeZone, Utc};
use gpui::{ListOffset, ScrollHandle};

use crate::model::{notion_page_identity_key, PageShellSearchResult, RecentPageResult};
use crate::ui::Arc;

use super::{
    NotionSearchListRow, NotionSearchRecencyBucket, NotionSearchResultsState, NotionSearchState,
};

#[cfg(test)]
impl NotionSearchState {
    pub(crate) fn list_row_is_query_header(&self, list_row_index: usize) -> bool {
        self.list_rows.get(list_row_index) == Some(&NotionSearchListRow::QueryHeader)
    }

    pub(crate) fn list_row_recency_header(
        &self,
        list_row_index: usize,
    ) -> Option<(&'static str, &'static str)> {
        let NotionSearchListRow::RecencyHeader(bucket) = self.list_rows.get(list_row_index)? else {
            return None;
        };
        Some((bucket.label(), bucket.selector_segment()))
    }

    pub(crate) fn query_result_index_for_list_row(&self, list_row_index: usize) -> Option<usize> {
        let NotionSearchListRow::QueryResult(result_index) = self.list_rows.get(list_row_index)?
        else {
            return None;
        };
        Some(*result_index)
    }

    pub(crate) fn recent_result_index_for_list_row(&self, list_row_index: usize) -> Option<usize> {
        let NotionSearchListRow::RecentResult(result_index) = self.list_rows.get(list_row_index)?
        else {
            return None;
        };
        Some(*result_index)
    }
}

impl NotionSearchState {
    pub(crate) fn list_rows_snapshot(&self) -> Arc<[NotionSearchListRow]> {
        self.list_rows.clone()
    }

    pub(crate) fn list_row_index_for_result(&self, result_index: usize) -> Option<usize> {
        list_row_index_for_result(&self.list_rows, result_index)
    }

    pub(crate) fn show_loading_rows(&mut self, item_count: usize) {
        self.list_rows = Arc::default();
        self.reset_list_state(item_count);
    }

    pub(super) fn show_query_rows(&mut self, result_count: usize, preserve_scroll: bool) {
        let rows = query_list_rows(result_count);
        if preserve_scroll {
            self.replace_list_rows_preserving_scroll(rows);
        } else {
            self.replace_list_rows(rows);
        }
    }

    pub(super) fn clear_list(&mut self) {
        self.replace_list_rows(Vec::new());
    }

    pub(super) fn replace_list_rows(&mut self, rows: Vec<NotionSearchListRow>) {
        self.list_rows = rows.into();
        self.reset_list_state(self.list_rows.len());
    }

    fn replace_list_rows_preserving_scroll(&mut self, rows: Vec<NotionSearchListRow>) {
        let mut scroll = self.list_state.logical_scroll_top();
        self.list_rows = rows.into();
        scroll.item_ix = scroll.item_ix.min(self.list_rows.len().saturating_sub(1));
        self.list_state.reset(self.list_rows.len());
        self.list_state.scroll_to(scroll);
    }

    pub(super) fn reset_list(&mut self, item_count: usize) {
        self.list_rows = Arc::default();
        self.reset_list_state(item_count);
    }

    fn reset_list_state(&mut self, item_count: usize) {
        self.list_state.reset(item_count);
        self.list_state.scroll_to(ListOffset::default());
    }

    pub(super) fn reconcile_selected_result(&mut self, results: &[PageShellSearchResult]) {
        let previous_identity = match &self.results {
            NotionSearchResultsState::Loaded { results, .. } => results
                .get(self.selected_index)
                .map(|result| notion_page_identity_key(&result.block_id)),
            NotionSearchResultsState::Idle
            | NotionSearchResultsState::Loading
            | NotionSearchResultsState::Failed => None,
        };
        let selected_index = self.selected_index.min(results.len().saturating_sub(1));
        let next_identity = results
            .get(selected_index)
            .map(|result| notion_page_identity_key(&result.block_id));
        if self.selected_index != selected_index || previous_identity != next_identity {
            self.selected_index = selected_index;
            self.preview_scroll_handle = ScrollHandle::new();
        }
    }

    pub(crate) fn set_selected_index(&mut self, selected_index: usize) {
        if self.selected_index == selected_index {
            return;
        }
        self.selected_index = selected_index;
        self.preview_scroll_handle = ScrollHandle::new();
    }

    pub(crate) fn reset_selected_index(&mut self) {
        self.selected_index = 0;
        self.preview_scroll_handle = ScrollHandle::new();
    }
}

pub(super) fn query_list_rows(result_count: usize) -> Vec<NotionSearchListRow> {
    std::iter::once(NotionSearchListRow::QueryHeader)
        .chain((0..result_count).map(NotionSearchListRow::QueryResult))
        .collect()
}

pub(super) fn recent_list_rows<Tz>(
    results: &[RecentPageResult],
    now: &DateTime<Tz>,
) -> Vec<NotionSearchListRow>
where
    Tz: TimeZone,
{
    let buckets = results
        .iter()
        .map(|result| notion_search_recency_bucket(result.visited_at_unix_millis, now))
        .collect::<Vec<_>>();
    grouped_recent_list_rows(&buckets)
}

pub(super) fn grouped_recent_list_rows(
    result_buckets: &[NotionSearchRecencyBucket],
) -> Vec<NotionSearchListRow> {
    let mut rows = Vec::with_capacity(result_buckets.len() + NotionSearchRecencyBucket::ALL.len());
    for bucket in NotionSearchRecencyBucket::ALL {
        let matching_indices = result_buckets
            .iter()
            .enumerate()
            .filter_map(|(index, result_bucket)| (*result_bucket == bucket).then_some(index));
        let mut matching_indices = matching_indices.peekable();
        if matching_indices.peek().is_none() {
            continue;
        }
        rows.push(NotionSearchListRow::RecencyHeader(bucket));
        rows.extend(matching_indices.map(NotionSearchListRow::RecentResult));
    }
    rows
}

pub(super) fn list_row_index_for_result(
    rows: &[NotionSearchListRow],
    result_index: usize,
) -> Option<usize> {
    rows.iter()
        .position(|row| row.result_index() == Some(result_index))
}

pub(super) fn notion_search_recency_bucket<Tz>(
    visited_at_unix_millis: u64,
    now: &DateTime<Tz>,
) -> NotionSearchRecencyBucket
where
    Tz: TimeZone,
{
    let timestamp = i64::try_from(visited_at_unix_millis)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or(DateTime::<Utc>::MAX_UTC);
    let visited_date = timestamp.with_timezone(&now.timezone()).date_naive();
    let today = now.date_naive();
    let elapsed_millis = now
        .timestamp_millis()
        .saturating_sub(timestamp.timestamp_millis());

    notion_search_recency_bucket_from_parts(visited_date, today, elapsed_millis)
}

pub(super) fn notion_search_recency_bucket_from_parts(
    visited_date: chrono::NaiveDate,
    today: chrono::NaiveDate,
    elapsed_millis: i64,
) -> NotionSearchRecencyBucket {
    let yesterday = today
        .pred_opt()
        .expect("current local date must have a predecessor");
    if visited_date == today && elapsed_millis <= 86_400_000 {
        NotionSearchRecencyBucket::Today
    } else if visited_date == yesterday && elapsed_millis <= 172_800_000 {
        NotionSearchRecencyBucket::Yesterday
    } else if elapsed_millis <= 604_800_000 {
        NotionSearchRecencyBucket::PastWeek
    } else if elapsed_millis <= 2_592_000_000 {
        NotionSearchRecencyBucket::PastThirtyDays
    } else {
        NotionSearchRecencyBucket::Older
    }
}
