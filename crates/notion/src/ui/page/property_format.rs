use crate::ui::CardPageProperty;
use time::{
    format_description::FormatItem, macros::format_description, Date, PrimitiveDateTime, Time,
    UtcOffset,
};

pub(crate) fn format_page_property_value(property: &CardPageProperty) -> String {
    if property.property_type != "date" {
        return property.value.clone();
    }
    format_date_like_value(&property.value).unwrap_or_else(|| property.value.clone())
}

pub(crate) fn format_date_like_value(value: &str) -> Option<String> {
    let date = parse_date_component(value)?;
    let time = parse_time_component(value);
    Some(match time {
        Some(time) => PrimitiveDateTime::new(date, time)
            .assume_utc()
            .to_offset(local_page_property_offset())
            .format(page_property_datetime_output_format())
            .ok()?,
        None => date.format(page_property_date_output_format()).ok()?,
    })
}

pub(crate) fn parse_date_component(value: &str) -> Option<Date> {
    let date = value.split_whitespace().next()?;
    Date::parse(date, page_property_date_input_format()).ok()
}

pub(crate) fn parse_time_component(value: &str) -> Option<Time> {
    value
        .split_whitespace()
        .nth(1)
        .and_then(|time| Time::parse(time, page_property_time_input_format()).ok())
}

pub(crate) fn local_page_property_offset() -> UtcOffset {
    UtcOffset::current_local_offset().expect("failed to determine local timezone offset")
}

pub(crate) fn page_property_date_input_format() -> &'static [FormatItem<'static>] {
    format_description!("[year]-[month]-[day]")
}

pub(crate) fn page_property_time_input_format() -> &'static [FormatItem<'static>] {
    format_description!("[hour]:[minute]")
}

pub(crate) fn page_property_date_output_format() -> &'static [FormatItem<'static>] {
    format_description!("[month repr:long] [day], [year]")
}

pub(crate) fn page_property_datetime_output_format() -> &'static [FormatItem<'static>] {
    format_description!(
        "[month repr:long] [day], [year] [hour repr:12]:[minute] [period case:upper]"
    )
}
