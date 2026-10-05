use serde_json::{json, Map, Number, Value};

use crate::model::{
    DatabaseDateFilter, DatabaseDateFilterMode, DatabaseDatePoint, DatabaseDateRange,
    DatabaseFilterGroup, DatabaseFilterGroupOperator, DatabaseFilterNode, DatabasePersonFilter,
    DatabasePropertyFilter, DatabaseRelativeDateDirection, DatabaseRelativeDatePreset,
    DatabaseRelativeDateUnit, DatabaseStatusFilterValue, DatabaseTextFilter,
    NonEmptyDatabaseFilterValues, NotionFilterPageId, NotionFilterUserId,
};

mod condition;

use condition::property_filter_condition_value;

pub(in crate::live::board) fn filter_group_value(group: &DatabaseFilterGroup) -> Value {
    let operator = match group.operator() {
        DatabaseFilterGroupOperator::And => "and",
        DatabaseFilterGroupOperator::Or => "or",
    };
    let filters = group
        .filters()
        .iter()
        .map(|filter| match filter {
            DatabaseFilterNode::Group(group) => filter_group_value(group),
            DatabaseFilterNode::Property(filter) => property_filter_value(filter),
        })
        .collect::<Vec<_>>();
    json!({ "operator": operator, "filters": filters })
}

pub(in crate::live::board) fn property_filter_value(filter: &DatabasePropertyFilter) -> Value {
    json!({
        "property": filter.property_id().as_str(),
        "filter": property_filter_condition_value(filter.condition()),
    })
}

fn number_filter_value(operator: &str, value: &Number) -> Value {
    json!({
        "operator": operator,
        "value": { "type": "exact", "value": value },
    })
}

fn checkbox_filter_value(operator: &str, checked: bool) -> Value {
    json!({
        "operator": operator,
        "value": { "type": "exact", "value": checked },
    })
}

fn person_filter_value(operator: &str, person: &DatabasePersonFilter) -> Value {
    let value = match person {
        DatabasePersonFilter::CurrentUser => json!({ "type": "relative", "value": "me" }),
        DatabasePersonFilter::Users(user_ids) => {
            Value::Array(person_filter_exact_user_values(user_ids))
        }
        DatabasePersonFilter::CurrentUserAndUsers(user_ids) => {
            let mut values = vec![json!({ "type": "relative", "value": "me" })];
            values.extend(person_filter_exact_user_values(user_ids));
            Value::Array(values)
        }
    };
    json!({ "operator": operator, "value": value })
}

fn person_filter_exact_user_values(
    user_ids: &NonEmptyDatabaseFilterValues<NotionFilterUserId>,
) -> Vec<Value> {
    user_ids
        .as_slice()
        .iter()
        .map(|user_id| {
            json!({
                "type": "exact",
                "value": { "table": "notion_user", "id": user_id.as_str() },
            })
        })
        .collect()
}

fn relation_filter_value(
    operator: &str,
    page_ids: &NonEmptyDatabaseFilterValues<NotionFilterPageId>,
) -> Value {
    let values = page_ids
        .as_slice()
        .iter()
        .map(|page_id| json!({ "type": "exact", "value": page_id.as_str() }))
        .collect::<Vec<_>>();
    json!({ "operator": operator, "value": values })
}

fn select_filter_value(operator: &str, options: &NonEmptyDatabaseFilterValues<String>) -> Value {
    let values = options
        .as_slice()
        .iter()
        .map(|option| json!({ "type": "exact", "value": option }))
        .collect::<Vec<_>>();
    json!({ "operator": operator, "value": values })
}

fn status_filter_value(
    operator: &str,
    statuses: &NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>,
) -> Value {
    let values = statuses
        .as_slice()
        .iter()
        .map(|status| match status {
            DatabaseStatusFilterValue::Option(option) => {
                json!({ "type": "is_option", "value": option })
            }
            DatabaseStatusFilterValue::Group(group) => {
                json!({ "type": "is_group", "value": group })
            }
        })
        .collect::<Vec<_>>();
    json!({ "operator": operator, "value": values })
}

fn date_point_filter_value(operator: &str, date: &DatabaseDateFilter<DatabaseDatePoint>) -> Value {
    let value = match date.value() {
        DatabaseDatePoint::Exact(date) => json!({
            "type": "exact",
            "value": {
                "type": "date",
                "start_date": date.format("%Y-%m-%d").to_string(),
            },
        }),
        DatabaseDatePoint::Relative(preset) => json!({
            "type": "relative",
            "value": relative_date_preset_value(*preset),
        }),
    };
    date_filter_with_mode(operator, value, date.mode())
}

fn date_range_filter_value(operator: &str, date: &DatabaseDateFilter<DatabaseDateRange>) -> Value {
    let value = match date.value() {
        DatabaseDateRange::Exact {
            start_date,
            end_date,
        } => {
            let mut range = Map::new();
            range.insert("type".to_string(), Value::String("daterange".to_string()));
            if let Some(start_date) = start_date {
                range.insert(
                    "start_date".to_string(),
                    Value::String(start_date.format("%Y-%m-%d").to_string()),
                );
            }
            if let Some(end_date) = end_date {
                range.insert(
                    "end_date".to_string(),
                    Value::String(end_date.format("%Y-%m-%d").to_string()),
                );
            }
            json!({ "type": "exact", "value": Value::Object(range) })
        }
        DatabaseDateRange::Relative {
            direction,
            count,
            unit,
        } => json!({
            "type": "relative",
            "value": "custom",
            "direction": relative_date_direction_value(*direction),
            "count": count,
            "unit": relative_date_unit_value(*unit),
        }),
        DatabaseDateRange::Surrounding { unit } => json!({
            "type": "relative",
            "value": "surrounding",
            "unit": relative_date_unit_value(*unit),
        }),
    };
    date_filter_with_mode(operator, value, date.mode())
}

fn date_filter_with_mode(operator: &str, value: Value, mode: DatabaseDateFilterMode) -> Value {
    let mut filter = Map::new();
    filter.insert("operator".to_string(), Value::String(operator.to_string()));
    filter.insert("value".to_string(), value);
    if mode == DatabaseDateFilterMode::EndDate {
        filter.insert("use_end".to_string(), Value::Bool(true));
    }
    Value::Object(filter)
}

const fn relative_date_preset_value(preset: DatabaseRelativeDatePreset) -> &'static str {
    match preset {
        DatabaseRelativeDatePreset::Today => "today",
        DatabaseRelativeDatePreset::Tomorrow => "tomorrow",
        DatabaseRelativeDatePreset::Yesterday => "yesterday",
        DatabaseRelativeDatePreset::OneWeekAgo => "one_week_ago",
        DatabaseRelativeDatePreset::OneWeekFromNow => "one_week_from_now",
        DatabaseRelativeDatePreset::OneMonthAgo => "one_month_ago",
        DatabaseRelativeDatePreset::OneMonthFromNow => "one_month_from_now",
    }
}

const fn relative_date_direction_value(direction: DatabaseRelativeDateDirection) -> &'static str {
    match direction {
        DatabaseRelativeDateDirection::Past => "past",
        DatabaseRelativeDateDirection::Future => "future",
    }
}

const fn relative_date_unit_value(unit: DatabaseRelativeDateUnit) -> &'static str {
    match unit {
        DatabaseRelativeDateUnit::Day => "day",
        DatabaseRelativeDateUnit::Week => "week",
        DatabaseRelativeDateUnit::Month => "month",
        DatabaseRelativeDateUnit::Year => "year",
    }
}

fn text_filter_value(filter: &DatabaseTextFilter) -> Value {
    let (operator, value) = match filter {
        DatabaseTextFilter::Is(value) => ("string_is", Some(value)),
        DatabaseTextFilter::IsNot(value) => ("string_is_not", Some(value)),
        DatabaseTextFilter::Contains(value) => ("string_contains", Some(value)),
        DatabaseTextFilter::DoesNotContain(value) => ("string_does_not_contain", Some(value)),
        DatabaseTextFilter::StartsWith(value) => ("string_starts_with", Some(value)),
        DatabaseTextFilter::EndsWith(value) => ("string_ends_with", Some(value)),
        DatabaseTextFilter::IsEmpty => ("is_empty", None),
        DatabaseTextFilter::IsNotEmpty => ("is_not_empty", None),
    };
    match value {
        Some(value) => json!({
            "operator": operator,
            "value": { "type": "exact", "value": value.as_str() },
        }),
        None => json!({ "operator": operator }),
    }
}
