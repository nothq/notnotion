use super::{parse_mention_date_query, ParsedMentionDate};
use chrono::{NaiveDate, NaiveTime};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid test date")
}

fn clock_time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).expect("valid test time")
}

fn parse(query: &str) -> Option<ParsedMentionDate> {
    parse_mention_date_query(query, date(2026, 9, 2), clock_time(16, 20))
}

fn day(query: &str) -> NaiveDate {
    let parsed = parse(query).unwrap_or_else(|| panic!("{query:?} should parse"));
    assert_eq!(parsed.time, None, "{query:?} should have no time");
    parsed.date
}

#[test]
fn keywords_match_notion_desktop_capture() {
    assert_eq!(day("today"), date(2026, 9, 2));
    assert_eq!(day("tomorrow"), date(2026, 9, 3));
    assert_eq!(day("yesterday"), date(2026, 9, 1));
    assert_eq!(day("next week"), date(2026, 9, 9));
    assert_eq!(day("last month"), date(2026, 8, 2));
    assert_eq!(day("in 3 days"), date(2026, 9, 5));
    assert_eq!(day("friday"), date(2026, 9, 4));
    assert_eq!(day("Sep 7"), date(2026, 9, 7));
    assert_eq!(day("aug 30"), date(2026, 8, 30));
    assert_eq!(day("sep 6"), date(2026, 9, 6));
    assert_eq!(day("sep 13"), date(2026, 9, 13));
    assert_eq!(day("sep 14"), date(2026, 9, 14));
    assert_eq!(day("1/5"), date(2027, 1, 5));
}

#[test]
fn times_attach_to_today_or_a_date() {
    assert_eq!(
        parse("9am"),
        Some(ParsedMentionDate {
            date: date(2026, 9, 2),
            time: Some(clock_time(9, 0)),
        })
    );
    assert_eq!(
        parse("tomorrow 9:30 pm"),
        Some(ParsedMentionDate {
            date: date(2026, 9, 3),
            time: Some(clock_time(21, 30)),
        })
    );
    assert_eq!(
        parse("sep 7 at 14:00"),
        Some(ParsedMentionDate {
            date: date(2026, 9, 7),
            time: Some(clock_time(14, 0)),
        })
    );
    assert_eq!(
        parse("now"),
        Some(ParsedMentionDate {
            date: date(2026, 9, 2),
            time: Some(clock_time(16, 20)),
        })
    );
    assert_eq!(
        parse("noon").and_then(|parsed| parsed.time),
        Some(clock_time(12, 0))
    );
}

#[test]
fn weekday_offsets_use_monday_start_weeks() {
    assert_eq!(day("next monday"), date(2026, 9, 7));
    assert_eq!(day("last monday"), date(2026, 8, 24));
    assert_eq!(day("this sunday"), date(2026, 9, 6));
    assert_eq!(day("wednesday"), date(2026, 9, 2));
    // A bare weekday stays inside the current Monday-start week, so a Monday
    // typed on Wednesday is the one two days back.
    assert_eq!(day("monday"), date(2026, 8, 31));
}

#[test]
fn explicit_forms_parse() {
    assert_eq!(day("7 sep 2027"), date(2027, 9, 7));
    assert_eq!(day("september 7th"), date(2026, 9, 7));
    assert_eq!(day("2026-12-24"), date(2026, 12, 24));
    assert_eq!(day("12/24/26"), date(2026, 12, 24));
    assert_eq!(day("2 weeks ago"), date(2026, 8, 19));
    assert_eq!(day("next year"), date(2027, 9, 2));
}

#[test]
fn non_dates_do_not_parse() {
    for query in ["x", "hello world", "", "13/45", "in days", "today please"] {
        assert_eq!(parse(query), None, "{query:?} should not parse");
    }
}

#[test]
fn partial_keywords_resolve_the_way_notion_completes_them() {
    // Measured on Notion desktop on Wednesday 2 September 2026.
    assert_eq!(day("t"), date(2026, 9, 2));
    assert_eq!(day("tod"), date(2026, 9, 2));
    assert_eq!(day("tom"), date(2026, 9, 3));
    assert_eq!(day("y"), date(2026, 9, 1));
    assert_eq!(day("yes"), date(2026, 9, 1));
    assert_eq!(day("fri"), date(2026, 9, 4));
    assert_eq!(day("mon"), date(2026, 8, 31));
    assert_eq!(day("sun"), date(2026, 9, 6));
}

#[test]
fn a_bare_month_is_its_first_day_and_rolls_forward_once_it_is_past() {
    assert_eq!(day("sep"), date(2026, 9, 1));
    assert_eq!(day("september"), date(2026, 9, 1));
    assert_eq!(day("aug"), date(2027, 8, 1));
}
