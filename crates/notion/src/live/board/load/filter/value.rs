use chrono::NaiveDate;
use serde_json::{Map, Value};

use crate::model::{
    DatabaseDateFilter, DatabaseDateFilterMode, DatabaseDatePoint, DatabaseDateRange,
    DatabasePersonFilter, DatabaseRelativeDateDirection, DatabaseRelativeDatePreset,
    DatabaseRelativeDateUnit, DatabaseStatusFilterValue, DatabaseTextFilter,
    DatabaseTextFilterValue, NonEmptyDatabaseFilterValues, NotionFilterPageId, NotionFilterUserId,
};

pub(super) fn parse_text_filter(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabaseTextFilter> {
    match operator {
        "string_is" => Some(DatabaseTextFilter::Is(parse_text_filter_value(filter)?)),
        "string_is_not" => Some(DatabaseTextFilter::IsNot(parse_text_filter_value(filter)?)),
        "string_contains" => Some(DatabaseTextFilter::Contains(parse_text_filter_value(
            filter,
        )?)),
        "string_does_not_contain" => Some(DatabaseTextFilter::DoesNotContain(
            parse_text_filter_value(filter)?,
        )),
        "string_starts_with" => Some(DatabaseTextFilter::StartsWith(parse_text_filter_value(
            filter,
        )?)),
        "string_ends_with" => Some(DatabaseTextFilter::EndsWith(parse_text_filter_value(
            filter,
        )?)),
        "is_empty" if !filter.contains_key("value") => Some(DatabaseTextFilter::IsEmpty),
        "is_not_empty" if !filter.contains_key("value") => Some(DatabaseTextFilter::IsNotEmpty),
        _ => None,
    }
}

fn parse_text_filter_value(filter: &Map<String, Value>) -> Option<DatabaseTextFilterValue> {
    let value = exact_value(filter.get("value")?)?.as_str()?.to_string();
    DatabaseTextFilterValue::try_from(value).ok()
}

pub(super) fn parse_person_filter(value: &Value) -> Option<DatabasePersonFilter> {
    let mut includes_current_user = false;
    let mut user_ids = Vec::new();
    for value in filter_values(value)? {
        let value = value.as_object()?;
        match value.get("type")?.as_str()? {
            "relative" if value.get("value")?.as_str()? == "me" => {
                includes_current_user = true;
            }
            "exact" => {
                let pointer = value.get("value")?.as_object()?;
                if pointer.get("table")?.as_str()? != "notion_user" {
                    return None;
                }
                user_ids.push(
                    NotionFilterUserId::try_from(pointer.get("id")?.as_str()?.to_string()).ok()?,
                );
            }
            _ => return None,
        }
    }
    match (includes_current_user, user_ids.is_empty()) {
        (true, true) => Some(DatabasePersonFilter::CurrentUser),
        (false, false) => Some(DatabasePersonFilter::Users(
            NonEmptyDatabaseFilterValues::try_from(user_ids).ok()?,
        )),
        (true, false) => Some(DatabasePersonFilter::CurrentUserAndUsers(
            NonEmptyDatabaseFilterValues::try_from(user_ids).ok()?,
        )),
        (false, true) => None,
    }
}

pub(super) fn parse_relation_filter_values(
    value: &Value,
) -> Option<NonEmptyDatabaseFilterValues<NotionFilterPageId>> {
    let page_ids = exact_values(value)?
        .into_iter()
        .map(|value| NotionFilterPageId::try_from(value.as_str()?.to_string()).ok())
        .collect::<Option<Vec<_>>>()?;
    NonEmptyDatabaseFilterValues::try_from(page_ids).ok()
}

pub(super) fn parse_select_filter_values(
    value: &Value,
) -> Option<NonEmptyDatabaseFilterValues<String>> {
    let values = exact_values(value)?
        .into_iter()
        .map(|value| value.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()?;
    NonEmptyDatabaseFilterValues::try_from(values).ok()
}

pub(super) fn parse_status_filter_values(
    value: &Value,
) -> Option<NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>> {
    let values = filter_values(value)?
        .into_iter()
        .map(parse_status_filter_value)
        .collect::<Option<Vec<_>>>()?;
    NonEmptyDatabaseFilterValues::try_from(values).ok()
}

pub(super) fn parse_date_point_filter(
    filter: &Map<String, Value>,
) -> Option<DatabaseDateFilter<DatabaseDatePoint>> {
    let value = filter.get("value")?;
    let point = match value.get("type")?.as_str()? {
        "relative" => {
            DatabaseDatePoint::Relative(parse_relative_date_preset(value.get("value")?.as_str()?)?)
        }
        "exact" => {
            let date = exact_value(value)?.as_object()?;
            if date.get("type")?.as_str()? != "date" || has_non_null_date_range_fields(date) {
                return None;
            }
            DatabaseDatePoint::Exact(parse_exact_date(date.get("start_date")?.as_str()?)?)
        }
        _ => return None,
    };
    Some(DatabaseDateFilter::new(
        point,
        parse_date_filter_mode(filter)?,
    ))
}

pub(super) fn parse_date_range_filter(
    filter: &Map<String, Value>,
) -> Option<DatabaseDateFilter<DatabaseDateRange>> {
    let value = filter.get("value")?.as_object()?;
    let range = match value.get("type")?.as_str()? {
        "exact" => parse_exact_date_range(value)?,
        "relative" => parse_relative_date_range(value)?,
        _ => return None,
    };
    match &range {
        DatabaseDateRange::Exact {
            start_date: None,
            end_date: None,
        }
        | DatabaseDateRange::Relative { count: 0, .. } => return None,
        DatabaseDateRange::Exact { .. }
        | DatabaseDateRange::Relative { .. }
        | DatabaseDateRange::Surrounding { .. } => {}
    }
    Some(DatabaseDateFilter::new(
        range,
        parse_date_filter_mode(filter)?,
    ))
}

fn parse_exact_date_range(value: &Map<String, Value>) -> Option<DatabaseDateRange> {
    let range = value.get("value")?.as_object()?;
    if range.get("type")?.as_str()? != "daterange" {
        return None;
    }
    let start_date = parse_optional_exact_date(range.get("start_date"))?;
    let end_date = parse_optional_exact_date(range.get("end_date"))?;
    Some(DatabaseDateRange::Exact {
        start_date,
        end_date,
    })
}

fn parse_relative_date_range(value: &Map<String, Value>) -> Option<DatabaseDateRange> {
    match value.get("value")?.as_str()? {
        "custom" => Some(DatabaseDateRange::Relative {
            direction: match value.get("direction")?.as_str()? {
                "past" => DatabaseRelativeDateDirection::Past,
                "future" => DatabaseRelativeDateDirection::Future,
                _ => return None,
            },
            count: u32::try_from(value.get("count")?.as_u64()?).ok()?,
            unit: parse_relative_date_unit(value.get("unit")?.as_str()?)?,
        }),
        "surrounding" => Some(DatabaseDateRange::Surrounding {
            unit: parse_relative_date_unit(value.get("unit")?.as_str()?)?,
        }),
        _ => None,
    }
}

fn parse_optional_exact_date(value: Option<&Value>) -> Option<Option<NaiveDate>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(value)) => parse_exact_date(value).map(Some),
        Some(_) => None,
    }
}

fn parse_date_filter_mode(filter: &Map<String, Value>) -> Option<DatabaseDateFilterMode> {
    match filter.get("use_end") {
        None => Some(DatabaseDateFilterMode::StartDate),
        Some(Value::Bool(false)) => Some(DatabaseDateFilterMode::StartDate),
        Some(Value::Bool(true)) => Some(DatabaseDateFilterMode::EndDate),
        Some(_) => None,
    }
}

fn parse_relative_date_preset(value: &str) -> Option<DatabaseRelativeDatePreset> {
    match value {
        "today" => Some(DatabaseRelativeDatePreset::Today),
        "tomorrow" => Some(DatabaseRelativeDatePreset::Tomorrow),
        "yesterday" => Some(DatabaseRelativeDatePreset::Yesterday),
        "one_week_ago" => Some(DatabaseRelativeDatePreset::OneWeekAgo),
        "one_week_from_now" => Some(DatabaseRelativeDatePreset::OneWeekFromNow),
        "one_month_ago" => Some(DatabaseRelativeDatePreset::OneMonthAgo),
        "one_month_from_now" => Some(DatabaseRelativeDatePreset::OneMonthFromNow),
        _ => None,
    }
}

fn parse_relative_date_unit(value: &str) -> Option<DatabaseRelativeDateUnit> {
    match value {
        "day" => Some(DatabaseRelativeDateUnit::Day),
        "week" => Some(DatabaseRelativeDateUnit::Week),
        "month" => Some(DatabaseRelativeDateUnit::Month),
        "year" => Some(DatabaseRelativeDateUnit::Year),
        _ => None,
    }
}

fn parse_status_filter_value(value: &Value) -> Option<DatabaseStatusFilterValue> {
    let value = value.as_object()?;
    let name = value.get("value")?.as_str()?.to_string();
    match value.get("type")?.as_str()? {
        "is_option" => Some(DatabaseStatusFilterValue::Option(name)),
        "is_group" => Some(DatabaseStatusFilterValue::Group(name)),
        _ => None,
    }
}

fn exact_values(value: &Value) -> Option<Vec<&Value>> {
    filter_values(value)?.into_iter().map(exact_value).collect()
}

fn filter_values(value: &Value) -> Option<Vec<&Value>> {
    match value {
        Value::Array(values) if !values.is_empty() => Some(values.iter().collect()),
        Value::Object(_) => Some(vec![value]),
        _ => None,
    }
}

pub(super) fn exact_value(value: &Value) -> Option<&Value> {
    let value = value.as_object()?;
    (value.get("type")?.as_str()? == "exact")
        .then(|| value.get("value"))
        .flatten()
}

fn parse_exact_date(value: &str) -> Option<NaiveDate> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    (date.format("%Y-%m-%d").to_string() == value).then_some(date)
}

fn has_non_null_date_range_fields(value: &Map<String, Value>) -> bool {
    ["end_date", "start_time", "end_time", "time_zone"]
        .into_iter()
        .any(|field| value.get(field).is_some_and(|value| !value.is_null()))
}
