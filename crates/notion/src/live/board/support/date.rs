use serde_json::Value;
use time::{
    format_description::FormatItem, macros::format_description, Date, PrimitiveDateTime, Time,
    UtcOffset,
};
pub(crate) fn format_date_property(value: &Value) -> String {
    let Some(start_date) = value.get("start_date").and_then(Value::as_str) else {
        return String::new();
    };
    let Some(start_time) = value.get("start_time").and_then(Value::as_str) else {
        return parse_notion_date(start_date)
            .format(date_output_format())
            .expect("failed to format Notion date");
    };
    parse_notion_datetime(start_date, start_time)
        .to_offset(local_notion_offset())
        .format(datetime_output_format())
        .expect("failed to format Notion datetime")
}

pub(crate) fn parse_notion_date(value: &str) -> Date {
    Date::parse(value, date_input_format()).expect("invalid Notion date")
}

pub(crate) fn parse_notion_datetime(start_date: &str, start_time: &str) -> time::OffsetDateTime {
    PrimitiveDateTime::new(
        parse_notion_date(start_date),
        Time::parse(start_time, time_input_format()).expect("invalid Notion time"),
    )
    .assume_utc()
}

pub(crate) fn local_notion_offset() -> UtcOffset {
    UtcOffset::current_local_offset().expect("failed to determine local timezone offset")
}

pub(crate) fn date_input_format() -> &'static [FormatItem<'static>] {
    format_description!("[year]-[month]-[day]")
}

pub(crate) fn time_input_format() -> &'static [FormatItem<'static>] {
    format_description!("[hour]:[minute]")
}

pub(crate) fn date_output_format() -> &'static [FormatItem<'static>] {
    format_description!("[month repr:long] [day], [year]")
}

pub(crate) fn datetime_output_format() -> &'static [FormatItem<'static>] {
    format_description!(
        "[month repr:long] [day], [year] [hour repr:12]:[minute] [period case:upper]"
    )
}
