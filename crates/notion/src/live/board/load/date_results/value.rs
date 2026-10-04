use chrono::NaiveDate;
use serde_json::{Map, Value};

pub(super) fn new_calendar_date_value(
    target_date: NaiveDate,
    date_property: &Map<String, Value>,
) -> Value {
    let mut date_value = Map::from_iter([
        ("type".to_string(), Value::String("date".to_string())),
        (
            "start_date".to_string(),
            Value::String(format_calendar_date(target_date)),
        ),
    ]);
    if let Some(default_reminder) = date_property.get("default_reminder") {
        date_value.insert("reminder".to_string(), default_reminder.clone());
    }
    Value::Object(date_value)
}

pub(super) fn decode_date_property_value(
    property_value: Option<&Value>,
    block_id: &str,
    property_id: &str,
) -> Result<Option<Value>, String> {
    let Some(property_value) = property_value else {
        return Ok(None);
    };
    let chunks = property_value.as_array().ok_or_else(|| {
        format!("Notion Calendar date property {property_id} on page {block_id} is not an array")
    })?;
    if chunks.is_empty() {
        return Ok(None);
    }
    if chunks.len() != 1 {
        return Err(format!(
            "Notion Calendar date property {property_id} on page {block_id} must contain one date value"
        ));
    }
    let chunk = chunks[0].as_array().ok_or_else(|| {
        format!("Notion Calendar date property {property_id} on page {block_id} has an invalid CRDT chunk")
    })?;
    if chunk.first().and_then(Value::as_str) != Some("‣") {
        return Err(format!(
            "Notion Calendar date property {property_id} on page {block_id} has an invalid CRDT token"
        ));
    }
    let references = chunk.get(1).and_then(Value::as_array).ok_or_else(|| {
        format!("Notion Calendar date property {property_id} on page {block_id} has no references")
    })?;
    if references.len() != 1 {
        return Err(format!(
            "Notion Calendar date property {property_id} on page {block_id} must contain one reference"
        ));
    }
    let reference = references[0].as_array().ok_or_else(|| {
        format!("Notion Calendar date property {property_id} on page {block_id} has an invalid reference")
    })?;
    if reference.first().and_then(Value::as_str) != Some("d") {
        return Err(format!(
            "Notion Calendar date property {property_id} on page {block_id} is not a date reference"
        ));
    }
    let date_value = reference.get(1).cloned().ok_or_else(|| {
        format!("Notion Calendar date property {property_id} on page {block_id} has no date value")
    })?;
    if !date_value.is_object() {
        return Err(format!(
            "Notion Calendar date property {property_id} on page {block_id} has a non-object date value"
        ));
    }
    Ok(Some(date_value))
}

pub(super) fn move_calendar_date_value(
    mut date_value: Value,
    target_date: NaiveDate,
) -> Result<Value, String> {
    let date = date_value
        .as_object_mut()
        .expect("decoded Notion Calendar date value must remain an object");
    let value_type = date
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "Notion Calendar date value is missing its type".to_string())?
        .to_string();
    let start_date = required_calendar_date(date, "start_date")?;
    let target_date_string = format_calendar_date(target_date);
    match value_type.as_str() {
        "date" | "datetime" => {
            date.insert("start_date".to_string(), Value::String(target_date_string));
        }
        "daterange" | "datetimerange" => {
            let end_date = required_calendar_date(date, "end_date")?;
            let duration = end_date.signed_duration_since(start_date);
            if duration.num_days() < 0 {
                return Err("Notion Calendar date range ends before it starts".to_string());
            }
            let target_end_date = target_date.checked_add_signed(duration).ok_or_else(|| {
                "Notion Calendar date range exceeds the supported date range".to_string()
            })?;
            date.insert("start_date".to_string(), Value::String(target_date_string));
            date.insert(
                "end_date".to_string(),
                Value::String(format_calendar_date(target_end_date)),
            );
        }
        unsupported => {
            return Err(format!(
                "unsupported Notion Calendar date value type {unsupported}"
            ));
        }
    }
    Ok(date_value)
}

#[derive(Clone, Debug)]
enum ExistingCalendarDateValueKind {
    Date,
    DateTime { start_time: String },
    DateRange,
    DateTimeRange,
}

impl ExistingCalendarDateValueKind {
    fn parse(date: &Map<String, Value>) -> Result<Self, String> {
        let value_type = date
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "Notion Calendar date value is missing its type".to_string())?;
        let start_date = required_calendar_date(date, "start_date")?;
        match value_type {
            "date" => Ok(Self::Date),
            "datetime" => Ok(Self::DateTime {
                start_time: required_calendar_time(date, "start_time")?.to_string(),
            }),
            "daterange" => {
                require_ordered_calendar_date_range(date, start_date)?;
                Ok(Self::DateRange)
            }
            "datetimerange" => {
                require_ordered_calendar_date_range(date, start_date)?;
                required_calendar_time(date, "start_time")?;
                required_calendar_time(date, "end_time")?;
                Ok(Self::DateTimeRange)
            }
            unsupported => Err(format!(
                "unsupported Notion Calendar date value type {unsupported}"
            )),
        }
    }
}

pub(super) fn resize_calendar_date_value(
    mut date_value: Value,
    inclusive_start_date: NaiveDate,
    inclusive_end_date: NaiveDate,
) -> Result<Value, String> {
    let date = date_value
        .as_object_mut()
        .expect("decoded Notion Calendar date value must remain an object");
    let kind = ExistingCalendarDateValueKind::parse(date)?;
    match kind {
        ExistingCalendarDateValueKind::Date | ExistingCalendarDateValueKind::DateRange => {
            date.insert("type".to_string(), Value::String("daterange".to_string()));
            date.remove("start_time");
            date.remove("end_time");
            date.remove("time_zone");
        }
        ExistingCalendarDateValueKind::DateTime { start_time } => {
            date.insert(
                "type".to_string(),
                Value::String("datetimerange".to_string()),
            );
            date.insert("end_time".to_string(), Value::String(start_time));
        }
        ExistingCalendarDateValueKind::DateTimeRange => {
            date.insert(
                "type".to_string(),
                Value::String("datetimerange".to_string()),
            );
        }
    }
    date.insert(
        "start_date".to_string(),
        Value::String(format_calendar_date(inclusive_start_date)),
    );
    date.insert(
        "end_date".to_string(),
        Value::String(format_calendar_date(inclusive_end_date)),
    );
    Ok(date_value)
}

fn require_ordered_calendar_date_range(
    date: &Map<String, Value>,
    start_date: NaiveDate,
) -> Result<(), String> {
    let end_date = required_calendar_date(date, "end_date")?;
    if end_date < start_date {
        return Err("Notion Calendar date range ends before it starts".to_string());
    }
    Ok(())
}

fn required_calendar_time<'a>(
    date: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, String> {
    date.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion Calendar date value is missing {field}"))
}

fn required_calendar_date(date: &Map<String, Value>, field: &str) -> Result<NaiveDate, String> {
    let value = date
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion Calendar date value is missing {field}"))?;
    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| format!("invalid Notion Calendar {field} {value}: {error}"))?;
    if format_calendar_date(parsed) != value {
        return Err(format!(
            "Notion Calendar date value {field} must use YYYY-MM-DD: {value}"
        ));
    }
    Ok(parsed)
}

fn format_calendar_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}
