use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike, Weekday};

use super::{PageMentionDate, PageMentionDateFormat, PageMentionTimeFormat};

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The label Notion renders for a date mention on `today`, without the "@".
pub fn mention_date_label(date: &PageMentionDate, today: NaiveDate) -> String {
    let mut label = day_label(date.start_date, date.date_format, today);
    if let Some(start_time) = date.start_time {
        label.push(' ');
        label.push_str(&time_label(start_time, date.time_format));
    }
    if let Some(end_date) = date.end_date {
        label.push_str(" → ");
        if end_date == date.start_date && date.end_time.is_some() {
            if let Some(end_time) = date.end_time {
                label.push_str(&time_label(end_time, date.time_format));
            }
        } else {
            label.push_str(&day_label(end_date, date.date_format, today));
            if let Some(end_time) = date.end_time {
                label.push(' ');
                label.push_str(&time_label(end_time, date.time_format));
            }
        }
    }
    label
}

fn day_label(date: NaiveDate, format: PageMentionDateFormat, today: NaiveDate) -> String {
    match format {
        PageMentionDateFormat::Relative => relative_day_label(date, today),
        PageMentionDateFormat::FullDate => full_date_label(date),
        PageMentionDateFormat::MonthDayYear => {
            format!("{:02}/{:02}/{}", date.month(), date.day(), date.year())
        }
        PageMentionDateFormat::DayMonthYear => {
            format!("{:02}/{:02}/{}", date.day(), date.month(), date.year())
        }
        PageMentionDateFormat::YearMonthDay => {
            format!("{}/{:02}/{:02}", date.year(), date.month(), date.day())
        }
    }
}

/// Notion's relative day label: Today, Tomorrow, Yesterday, a weekday inside
/// the current Monday-to-Sunday week, "Next"/"Last" plus weekday for the
/// adjacent weeks, and the full date otherwise.
pub fn relative_day_label(date: NaiveDate, today: NaiveDate) -> String {
    match (date - today).num_days() {
        0 => return "Today".to_string(),
        1 => return "Tomorrow".to_string(),
        -1 => return "Yesterday".to_string(),
        _ => {}
    }
    let week = week_start(date);
    let this_week = week_start(today);
    let weekday = weekday_name(date.weekday());
    if week == this_week {
        weekday.to_string()
    } else if week == this_week + Duration::days(7) {
        format!("Next {weekday}")
    } else if week == this_week - Duration::days(7) {
        format!("Last {weekday}")
    } else {
        full_date_label(date)
    }
}

pub(super) fn full_date_label(date: NaiveDate) -> String {
    format!(
        "{} {}, {}",
        MONTH_NAMES[(date.month0()) as usize],
        date.day(),
        date.year()
    )
}

fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

pub(super) const fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

/// "9:00 AM" for the 12-hour format, "09:00" for the 24-hour format.
pub(super) fn time_label(clock_time: NaiveTime, format: PageMentionTimeFormat) -> String {
    match format {
        PageMentionTimeFormat::TwelveHour => {
            let (is_pm, hour12) = clock_time.hour12();
            format!(
                "{}:{:02} {}",
                hour12,
                clock_time.minute(),
                if is_pm { "PM" } else { "AM" }
            )
        }
        PageMentionTimeFormat::TwentyFourHour => {
            format!("{:02}:{:02}", clock_time.hour(), clock_time.minute())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mention_date_label, relative_day_label};
    use crate::model::{PageMentionDate, PageMentionDateFormat, PageMentionTimeFormat};
    use chrono::{NaiveDate, NaiveTime, Timelike};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid test date")
    }

    fn today() -> NaiveDate {
        date(2026, 9, 2)
    }

    #[test]
    fn relative_labels_match_notion_desktop_capture() {
        let cases = [
            (date(2026, 9, 2), "Today"),
            (date(2026, 9, 3), "Tomorrow"),
            (date(2026, 9, 1), "Yesterday"),
            (date(2026, 9, 7), "Next Monday"),
            (date(2026, 8, 30), "Last Sunday"),
            (date(2026, 9, 6), "Sunday"),
            (date(2026, 9, 13), "Next Sunday"),
            (date(2026, 9, 14), "September 14, 2026"),
            (date(2026, 9, 9), "Next Wednesday"),
            (date(2026, 9, 4), "Friday"),
            (date(2027, 1, 5), "January 5, 2027"),
            (date(2026, 9, 5), "Saturday"),
            (date(2026, 8, 2), "August 2, 2026"),
            (date(2026, 8, 31), "Monday"),
            (date(2026, 8, 27), "Last Thursday"),
            (date(2026, 8, 20), "August 20, 2026"),
        ];
        for (candidate, expected) in cases {
            assert_eq!(
                relative_day_label(candidate, today()),
                expected,
                "{candidate}"
            );
        }
    }

    #[test]
    fn date_time_labels_append_twelve_hour_time() {
        let mention = PageMentionDate::day_time(
            today(),
            NaiveTime::from_hms_opt(9, 0, 0).expect("valid time"),
            "America/Toronto".to_string(),
        );
        assert_eq!(mention_date_label(&mention, today()), "Today 9:00 AM");
        let evening = PageMentionDate {
            start_time: Some(NaiveTime::from_hms_opt(21, 30, 0).expect("valid time")),
            time_format: PageMentionTimeFormat::TwentyFourHour,
            ..mention
        };
        assert_eq!(mention_date_label(&evening, today()), "Today 21:30");
        assert_eq!(evening.start_time.expect("time").hour(), 21);
    }

    #[test]
    fn absolute_formats_ignore_today() {
        let mut mention = PageMentionDate::day(date(2026, 9, 2));
        mention.date_format = PageMentionDateFormat::FullDate;
        assert_eq!(mention_date_label(&mention, today()), "September 2, 2026");
        mention.date_format = PageMentionDateFormat::MonthDayYear;
        assert_eq!(mention_date_label(&mention, today()), "09/02/2026");
        mention.date_format = PageMentionDateFormat::DayMonthYear;
        assert_eq!(mention_date_label(&mention, today()), "02/09/2026");
        mention.date_format = PageMentionDateFormat::YearMonthDay;
        assert_eq!(mention_date_label(&mention, today()), "2026/09/02");
    }

    #[test]
    fn ranges_render_with_an_arrow() {
        let mut mention = PageMentionDate::day(date(2026, 9, 2));
        mention.end_date = Some(date(2026, 9, 5));
        assert_eq!(mention_date_label(&mention, today()), "Today → Saturday");
    }
}
