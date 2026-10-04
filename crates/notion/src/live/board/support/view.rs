use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{CalendarViewConfig, TimelineViewConfig, ViewTabKind};
use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Tz;
use reqwest::Url;
use serde_json::{Map, Value};

use super::record::required_string;

mod collection_pointer;
mod schema;

pub(crate) use collection_pointer::{
    block_collection_pointer, collection_view_collection_id, view_collection_pointer,
    BlockCollectionPointer, CollectionPointer,
};
pub(crate) use schema::{
    board_active_view_group, board_active_view_property_layout, board_active_view_sorts,
    board_database_properties, board_group_by, board_table_view_columns,
    database_property_filter_type, database_property_options, resolve_active_view_id,
    status_option_colors,
};

type ViewQuery = Map<String, Value>;

pub(crate) fn database_view_query(view: &Value) -> Result<Option<&ViewQuery>, String> {
    match view.get("query2") {
        None => Ok(None),
        Some(Value::Object(query)) => Ok(Some(query)),
        Some(_) => Err("Notion active database view query2 must be an object".to_string()),
    }
}

pub(crate) fn board_timeline_view(view: &Value) -> Option<TimelineViewConfig> {
    let date_property_id = view
        .get("query2")
        .and_then(|query| query.get("timeline_by"))
        .and_then(Value::as_str)?;
    let preference = view
        .get("format")
        .and_then(|format| format.get("timeline_preference"));
    let center_timestamp_ms = preference
        .and_then(|preference| preference.get("centerTimestamp"))
        .and_then(Value::as_i64)
        .or_else(|| {
            preference
                .and_then(|preference| preference.get("centerTimestamp"))
                .and_then(Value::as_u64)
                .map(|value| value as i64)
        });
    Some(TimelineViewConfig {
        date_property_id: date_property_id.to_string(),
        relation_property_id: view
            .get("format")
            .and_then(|format| format.get("timeline_arrows_by"))
            .and_then(|value| value.get("property"))
            .and_then(Value::as_str)
            .map(str::to_string),
        zoom_level: preference
            .and_then(|preference| preference.get("zoomLevel"))
            .and_then(Value::as_str)
            .map(str::to_string),
        center_timestamp_ms,
        today_marker_timestamp_ms: view
            .get("last_edited_time")
            .and_then(Value::as_i64)
            .or_else(|| {
                view.get("last_edited_time")
                    .and_then(Value::as_u64)
                    .map(|value| value as i64)
            }),
    })
}

pub(crate) fn board_calendar_view(view: &Value) -> Result<CalendarViewConfig, String> {
    let query = view
        .get("query2")
        .ok_or_else(|| "missing Notion calendar query2 configuration".to_string())?;
    let date_property_id = required_string(query, "calendar_by")?;
    if date_property_id.trim().is_empty() {
        return Err("Notion calendar_by property ID must not be empty or whitespace".to_string());
    }
    let has_explicit_sort = match query.get("sort") {
        None => false,
        Some(sort) => !sort
            .as_array()
            .ok_or_else(|| "Notion calendar query2.sort must be an array".to_string())?
            .is_empty(),
    };
    let show_page_icon = match view
        .get("format")
        .and_then(|format| format.get("show_page_icon"))
    {
        None | Some(Value::Null) => true,
        Some(show_page_icon) => show_page_icon
            .as_bool()
            .ok_or_else(|| "Notion calendar format.show_page_icon must be a boolean".to_string())?,
    };
    Ok(CalendarViewConfig {
        date_property_id: date_property_id.to_string(),
        has_explicit_sort,
        show_page_icon,
    })
}

pub(crate) fn view_tab_kind(value: Option<&str>) -> ViewTabKind {
    match value {
        Some("table") => ViewTabKind::Table,
        Some("board") => ViewTabKind::Board,
        Some("list") => ViewTabKind::List,
        Some("gallery") => ViewTabKind::Gallery,
        Some("timeline") => ViewTabKind::Timeline,
        Some("calendar") => ViewTabKind::Calendar,
        _ => ViewTabKind::Unknown,
    }
}

pub(crate) fn format_edited_label(last_edited_time_ms: u64, time_zone: &str) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(last_edited_time_ms);
    let diff_hours = ((now_ms.saturating_sub(last_edited_time_ms)) / 3_600_000).max(1);
    if diff_hours < 24 {
        return format!("Edited {diff_hours}h ago");
    }

    let diff_days = diff_hours / 24;
    if diff_days < 7 {
        return format!("Edited {diff_days}d ago");
    }

    format!(
        "Edited {}",
        format_edited_date_label(last_edited_time_ms, time_zone)
    )
}

pub(crate) fn format_edited_date_label(last_edited_time_ms: u64, time_zone: &str) -> String {
    let time_zone = time_zone
        .parse::<Tz>()
        .expect("Notion user time zone must be a valid IANA time zone");
    let edited_at = DateTime::<Utc>::from_timestamp_millis(last_edited_time_ms as i64)
        .expect("Notion last_edited_time must be a valid Unix timestamp")
        .with_timezone(&time_zone);
    format!("{} {}", edited_at.format("%b"), edited_at.day())
}

pub fn canonicalize_board_url(board_url: &str) -> Result<String, String> {
    let parsed = Url::parse(board_url).map_err(|error| error.to_string())?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("missing board host in {board_url}"))?;
    let authority = match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let mut canonical = format!("{}://{}{}", parsed.scheme(), authority, parsed.path());
    if let Some(view_id) = parsed
        .query_pairs()
        .find_map(|(key, value)| (key == "v").then_some(value.into_owned()))
    {
        canonical.push_str("?v=");
        canonical.push_str(&view_id);
    }
    Ok(canonical)
}

pub(crate) fn card_height_for_title(title: &str) -> f32 {
    let max_line_width = 22usize;
    let mut lines = 0usize;
    let mut line_width = 0usize;

    for word in title.split_whitespace() {
        let word_width = word.chars().count();
        if line_width == 0 {
            line_width = word_width;
            lines += 1;
            continue;
        }

        if line_width + 1 + word_width > max_line_width {
            lines += 1;
            line_width = word_width;
        } else {
            line_width += 1 + word_width;
        }
    }

    match lines.max(1) {
        1 => 40.0,
        2 => 61.75,
        3 => 83.5,
        count => 83.5 + (count.saturating_sub(3) as f32 * 21.75),
    }
}

pub(crate) fn board_is_private(collection_view_block: &Value) -> bool {
    collection_view_block
        .get("permissions")
        .and_then(Value::as_array)
        .map(|permissions| {
            !permissions.is_empty()
                && permissions.iter().all(|permission| {
                    permission.get("type").and_then(Value::as_str) == Some("user_permission")
                })
        })
        .unwrap_or(false)
}

pub(crate) fn board_is_locked(collection_view_block: &Value) -> bool {
    collection_view_block
        .get("is_locked")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || collection_view_block
            .get("format")
            .and_then(|format| format.get("block_locked"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::canonicalize_board_url;

    #[gpui::test]
    fn canonicalize_board_url_preserves_page_url_without_view_id() {
        let canonical = canonicalize_board_url(
            "https://www.notion.so/acme/Operations-d4e5f6a7b8c94a4bb526d7e8f90a1b23",
        )
        .expect("canonical board url should parse");

        assert_eq!(
            canonical,
            "https://www.notion.so/acme/Operations-d4e5f6a7b8c94a4bb526d7e8f90a1b23"
        );
    }

    #[gpui::test]
    fn canonicalize_board_url_preserves_explicit_view_id() {
        let canonical = canonicalize_board_url(
            "https://www.notion.so/acme/Operations-d4e5f6a7b8c94a4bb526d7e8f90a1b23?v=1a2b3c4d5e6f7890abcdeffedcba0123",
        )
        .expect("canonical board url should parse");

        assert_eq!(
            canonical,
            "https://www.notion.so/acme/Operations-d4e5f6a7b8c94a4bb526d7e8f90a1b23?v=1a2b3c4d5e6f7890abcdeffedcba0123"
        );
    }
}
