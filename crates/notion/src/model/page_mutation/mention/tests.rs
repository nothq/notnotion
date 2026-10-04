use super::{PageMention, PageMentionDate, PageMentionReminder};
use chrono::{NaiveDate, NaiveTime};
use serde_json::{json, Value};

#[test]
fn date_mention_round_trips_notion_shape() {
    let value = json!({
        "type": "datetime",
        "time_zone": "America/Toronto",
        "start_date": "2026-09-02",
        "start_time": "09:00",
        "date_format": "relative",
        "reminder": {"unit": "minute", "value": 0},
        "future_key": {"nested": true}
    });
    let date = PageMentionDate::from_notion_value(&value).expect("parse date mention");
    assert_eq!(
        date.start_date,
        NaiveDate::from_ymd_opt(2026, 9, 2).expect("valid date")
    );
    assert_eq!(
        date.start_time,
        Some(NaiveTime::from_hms_opt(9, 0, 0).expect("valid time"))
    );
    assert_eq!(date.time_zone.as_deref(), Some("America/Toronto"));
    assert_eq!(date.reminder, Some(PageMentionReminder::minutes_before(0)));
    let encoded = Value::Object(date.to_notion_value());
    assert_eq!(encoded["type"], "datetime");
    assert_eq!(encoded["start_date"], "2026-09-02");
    assert_eq!(encoded["start_time"], "09:00");
    assert_eq!(encoded["time_format"], "LT");
    assert_eq!(encoded["future_key"]["nested"], true);
    assert_eq!(
        PageMentionDate::from_notion_value(&encoded).expect("re-parse"),
        date
    );
}

#[test]
fn plain_date_mention_matches_captured_wire_shape() {
    let date = PageMentionDate::day(NaiveDate::from_ymd_opt(2026, 9, 3).expect("valid"));
    let mention = PageMention::Date(date);
    assert_eq!(
        Value::Array(mention.annotation_tuple()),
        json!(["d", {"type": "date", "start_date": "2026-09-03", "date_format": "relative"}])
    );
}

#[test]
fn unknown_formats_are_rejected() {
    let value = json!({"type": "date", "start_date": "2026-09-02", "date_format": "??"});
    assert!(PageMentionDate::from_notion_value(&value).is_err());
}
