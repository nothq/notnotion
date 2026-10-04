use crate::ui::BoardDateValue;
use crate::ui::BoardItem;
use crate::ui::BoardSnapshot;
use crate::ui::TimelineViewConfig;
use app_model::{AppearanceMode, Viewport};
use time::UtcOffset;

use super::{
    alpha, CivilDate, TIMELINE_DAY_ROW_LEFT_INSET, TIMELINE_DAY_ROW_RIGHT_INSET,
    TIMELINE_MONTH_CENTER_LEAD_DAYS, TIMELINE_MONTH_DAY_CELL_WIDTH, TIMELINE_TODAY_COLOR,
    TIMELINE_TODAY_COLUMN_INDEX, TIMELINE_VISIBLE_DAY_COUNT,
};

pub(crate) fn parse_civil_date(value: &str) -> Option<CivilDate> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some(CivilDate { year, month, day })
}

pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}

pub(crate) fn civil_from_days(days: i64) -> CivilDate {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = (yoe as i32) + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    CivilDate {
        year: year + i32::from(month <= 2),
        month: month as u32,
        day: day as u32,
    }
}

pub(crate) fn add_days(date: CivilDate, delta: i64) -> CivilDate {
    civil_from_days(date.ordinal() + delta)
}

pub(crate) fn next_month(date: CivilDate) -> CivilDate {
    if date.month == 12 {
        CivilDate {
            year: date.year + 1,
            month: 1,
            day: 1,
        }
    } else {
        CivilDate {
            year: date.year,
            month: date.month + 1,
            day: 1,
        }
    }
}

pub(crate) fn civil_date_month_start(date: CivilDate) -> CivilDate {
    CivilDate {
        year: date.year,
        month: date.month,
        day: 1,
    }
}

pub(crate) fn civil_date_for_timestamp_ms(timestamp_ms: i64) -> CivilDate {
    let days = timestamp_ms.div_euclid(86_400_000);
    civil_from_days(days)
}

pub(crate) fn civil_date_is_weekend(date: CivilDate) -> bool {
    matches!((date.ordinal() + 3).rem_euclid(7), 5 | 6)
}

pub(crate) fn local_today_civil_date() -> CivilDate {
    let now = time::OffsetDateTime::now_utc().to_offset(
        UtcOffset::current_local_offset().expect("failed to determine local timezone offset"),
    );
    CivilDate {
        year: now.year(),
        month: u8::from(now.month()) as u32,
        day: now.day().into(),
    }
}

pub(crate) fn timeline_marker_date(timeline_view: &TimelineViewConfig) -> CivilDate {
    timeline_view
        .today_marker_timestamp_ms
        .map(civil_date_for_timestamp_ms)
        .unwrap_or_else(local_today_civil_date)
}

pub(crate) fn timeline_visible_start(
    timeline_view: &TimelineViewConfig,
    today_marker_date: CivilDate,
    _has_dated_items: bool,
) -> CivilDate {
    if let Some(center_timestamp_ms) = timeline_view.center_timestamp_ms {
        if timeline_view.zoom_level.as_deref() == Some("month") {
            return add_days(
                civil_date_for_timestamp_ms(center_timestamp_ms),
                -TIMELINE_MONTH_CENTER_LEAD_DAYS,
            );
        }
        return add_days(
            civil_date_for_timestamp_ms(center_timestamp_ms),
            -((TIMELINE_VISIBLE_DAY_COUNT.saturating_sub(1) as i64) / 2),
        );
    }

    add_days(today_marker_date, -(TIMELINE_TODAY_COLUMN_INDEX as i64))
}

pub(crate) fn timeline_day_cell_width(timeline_view: &TimelineViewConfig) -> f32 {
    match timeline_view.zoom_level.as_deref() {
        Some("month") | None => TIMELINE_MONTH_DAY_CELL_WIDTH,
        _ => TIMELINE_MONTH_DAY_CELL_WIDTH,
    }
}

pub(crate) fn timeline_header_month_labels(
    content_start: CivilDate,
    content_end: CivilDate,
    day_cell_width: f32,
) -> Vec<(String, f32)> {
    let mut labels = Vec::new();
    let mut month = civil_date_month_start(content_start);
    let mut index = 0usize;
    let anchor_year = month.year;
    while month.ordinal() < content_end.ordinal() {
        let left = if index == 0 {
            0.0
        } else {
            TIMELINE_DAY_ROW_LEFT_INSET
                + (month.ordinal() - content_start.ordinal()) as f32 * day_cell_width
        };
        let label = if index == 0 || month.year != anchor_year {
            format!("{} {}", month_name(month.month), month.year)
        } else {
            month_name(month.month).to_string()
        };
        labels.push((label, left));
        month = next_month(month);
        index += 1;
    }
    labels
}

pub(crate) fn timeline_date_range_for_item<'a>(
    item: &'a BoardItem,
    timeline_view: &TimelineViewConfig,
) -> Option<&'a BoardDateValue> {
    item.properties
        .iter()
        .find(|property| property.property_id == timeline_view.date_property_id)
        .and_then(|property| property.date.as_ref())
}

pub(crate) fn timeline_grid_rule_color(
    appearance_mode: AppearanceMode,
    text_primary: u32,
) -> gpui::Hsla {
    alpha(
        text_primary,
        if appearance_mode == AppearanceMode::Light {
            0.09
        } else {
            0.12
        },
    )
}

pub(crate) fn timeline_weekend_band_color(
    appearance_mode: AppearanceMode,
    text_primary: u32,
) -> gpui::Hsla {
    alpha(
        text_primary,
        if appearance_mode == AppearanceMode::Light {
            0.035
        } else {
            0.055
        },
    )
}

pub(crate) fn timeline_today_line_color(appearance_mode: AppearanceMode) -> gpui::Hsla {
    alpha(
        TIMELINE_TODAY_COLOR,
        if appearance_mode == AppearanceMode::Light {
            0.62
        } else {
            0.48
        },
    )
}

pub(crate) fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Month",
    }
}

pub(crate) fn timeline_bar(
    date_range: &BoardDateValue,
    month_start: CivilDate,
    month_end: CivilDate,
    cell_width: f32,
) -> Option<(f32, f32)> {
    let start = parse_civil_date(&date_range.start_date)?;
    let end = date_range
        .end_date
        .as_deref()
        .and_then(parse_civil_date)
        .unwrap_or(start);
    let start_ordinal = start.ordinal().max(month_start.ordinal());
    let end_ordinal = end.ordinal().min(month_end.ordinal() - 1);
    if end_ordinal < month_start.ordinal() || start_ordinal >= month_end.ordinal() {
        return None;
    }

    let left = (start_ordinal - month_start.ordinal()) as f32 * cell_width + 4.0;
    let width = ((end_ordinal - start_ordinal + 1) as f32 * cell_width - 8.0).max(12.0);
    Some((left, width))
}

#[derive(Clone, Copy)]
pub(crate) struct TimelineContentRange {
    pub(crate) start: CivilDate,
    pub(crate) end: CivilDate,
}

pub(crate) fn timeline_content_range(
    timeline_view: &TimelineViewConfig,
    dated_items: &[(&BoardItem, &BoardDateValue)],
    today_marker_date: CivilDate,
) -> TimelineContentRange {
    let initial_visible_start =
        timeline_visible_start(timeline_view, today_marker_date, !dated_items.is_empty());
    let initial_visible_end = add_days(initial_visible_start, TIMELINE_VISIBLE_DAY_COUNT as i64);
    let earliest_date = dated_items
        .iter()
        .filter_map(|(_, date_range)| parse_civil_date(&date_range.start_date))
        .chain(std::iter::once(initial_visible_start))
        .min_by_key(|date| date.ordinal())
        .expect("timeline range should include an initial visible start");
    let latest_date = dated_items
        .iter()
        .filter_map(|(_, date_range)| {
            date_range
                .end_date
                .as_deref()
                .and_then(parse_civil_date)
                .or_else(|| parse_civil_date(&date_range.start_date))
        })
        .chain(std::iter::once(add_days(initial_visible_end, -1)))
        .max_by_key(|date| date.ordinal())
        .expect("timeline range should include an initial visible end");
    let start = civil_date_month_start(earliest_date);
    let end = next_month(civil_date_month_start(latest_date));
    TimelineContentRange { start, end }
}

pub(crate) fn timeline_grid_width(day_count: usize, day_cell_width: f32) -> f32 {
    (day_count as f32 * day_cell_width + TIMELINE_DAY_ROW_LEFT_INSET + TIMELINE_DAY_ROW_RIGHT_INSET)
        .max(720.0)
}

pub(crate) fn timeline_initial_scroll_x(board: &BoardSnapshot, viewport: Viewport) -> f32 {
    let Some(timeline_view) = board.timeline_view.as_ref() else {
        return 0.0;
    };
    let dated_items = board
        .items
        .iter()
        .filter_map(|item| {
            timeline_date_range_for_item(item, timeline_view).map(|date_range| (item, date_range))
        })
        .collect::<Vec<_>>();
    let today_marker_date = timeline_marker_date(timeline_view);
    let content = timeline_content_range(timeline_view, &dated_items, today_marker_date);
    let initial_visible_start =
        timeline_visible_start(timeline_view, today_marker_date, !dated_items.is_empty());
    let day_cell_width = timeline_day_cell_width(timeline_view);
    let day_count = (content.end.ordinal() - content.start.ordinal()).max(1) as usize;
    let content_width = timeline_grid_width(day_count, day_cell_width);
    let max_scroll_x = (content_width - viewport.app_width()).max(0.0);
    ((initial_visible_start.ordinal() - content.start.ordinal()) as f32 * day_cell_width)
        .clamp(0.0, max_scroll_x)
}
