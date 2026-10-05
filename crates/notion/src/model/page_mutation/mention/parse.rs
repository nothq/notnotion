use chrono::{Datelike, Duration, Months, NaiveDate, NaiveTime, Weekday};

#[cfg(test)]
mod tests;

/// A date (and optional time) resolved from the text typed after "@".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedMentionDate {
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
}

/// Resolve the natural-language date grammar Notion's mention menu accepts.
///
/// `today` and `now` come from the local clock so the parser stays pure. The
/// grammar covers the forms observed in Notion desktop: `today`, `tomorrow`,
/// `yesterday`, `now`, `next week`, `last month`, `in 3 days`, `2 weeks ago`,
/// weekday names with an optional `next`/`last`/`this`, `sep 7`, `7 sep 2027`,
/// `1/5`, `2026-09-07`, and a trailing time such as `9am`, `9:30 pm`, `14:00`,
/// `noon` or `midnight`. Month-and-day names without a year stay in the current
/// year; a bare past month and numeric month/day dates roll forward to their
/// next occurrence.
pub fn parse_mention_date_query(
    query: &str,
    today: NaiveDate,
    now: NaiveTime,
) -> Option<ParsedMentionDate> {
    let normalized = query.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let mut words = normalized
        .split_whitespace()
        .map(|word| word.trim_matches(','))
        .filter(|word| !word.is_empty() && *word != "at" && *word != "on")
        .collect::<Vec<_>>();
    let clock_time = take_trailing_time(&mut words);
    if words.is_empty() {
        return clock_time.map(|clock_time| ParsedMentionDate {
            date: today,
            time: Some(clock_time),
        });
    }
    let (date, implied_time) = parse_date_words(&words, today, now)?;
    Some(ParsedMentionDate {
        date,
        time: clock_time.or(implied_time),
    })
}

/// A date and the time its words imply, such as `now` or `tonight`.
type DateWithImpliedTime = (NaiveDate, Option<NaiveTime>);

fn parse_date_words(
    words: &[&str],
    today: NaiveDate,
    now: NaiveTime,
) -> Option<DateWithImpliedTime> {
    match words {
        ["today"] => Some((today, None)),
        ["tomorrow"] => Some((today + Duration::days(1), None)),
        ["yesterday"] => Some((today - Duration::days(1), None)),
        ["now"] => Some((today, Some(now))),
        ["tonight"] => Some((today, Some(NaiveTime::from_hms_opt(20, 0, 0)?))),
        ["next", unit] => shift_by_unit(today, unit, 1).map(|date| (date, None)),
        ["last", unit] | ["previous", unit] => {
            shift_by_unit(today, unit, -1).map(|date| (date, None))
        }
        ["this", unit] => shift_by_unit(today, unit, 0).map(|date| (date, None)),
        ["in", count, unit] => {
            let count = count.parse::<i64>().ok()?;
            shift_by_unit_count(today, unit, count).map(|date| (date, None))
        }
        [count, unit, "ago"] => {
            let count = count.parse::<i64>().ok()?;
            shift_by_unit_count(today, unit, -count).map(|date| (date, None))
        }
        [count, unit, "from", "now"] => {
            let count = count.parse::<i64>().ok()?;
            shift_by_unit_count(today, unit, count).map(|date| (date, None))
        }
        [single] => parse_single_word_date(single, today).map(|date| (date, None)),
        [first, second] => parse_two_word_date(first, second, today).map(|date| (date, None)),
        [first, second, third] => {
            parse_three_word_date(first, second, third, today).map(|date| (date, None))
        }
        _ => None,
    }
}

fn parse_single_word_date(word: &str, today: NaiveDate) -> Option<NaiveDate> {
    if let Some(date) = parse_relative_day_keyword(word, today) {
        return Some(date);
    }
    if let Some(weekday) = parse_weekday(word) {
        return Some(weekday_in_week(today, weekday, 0));
    }
    if let Some(month) = parse_month(word) {
        return first_day_of_month(today, month);
    }
    if let Some(date) = parse_iso_date(word) {
        return Some(date);
    }
    parse_slash_date(word, today)
}

/// Notion completes a partial day keyword while it is typed, so `tod` and even
/// `t` land on Today. A shared prefix takes the earliest entry, which is why
/// `t` is Today rather than Tomorrow.
fn parse_relative_day_keyword(word: &str, today: NaiveDate) -> Option<NaiveDate> {
    const KEYWORDS: [(&str, i64); 3] = [("today", 0), ("tomorrow", 1), ("yesterday", -1)];
    KEYWORDS
        .iter()
        .find(|(keyword, _)| keyword.starts_with(word))
        .map(|(_, offset)| today + Duration::days(*offset))
}

/// A bare month names its first day, next year once the month itself is past.
fn first_day_of_month(today: NaiveDate, month: u32) -> Option<NaiveDate> {
    let year = if month < today.month() {
        today.year() + 1
    } else {
        today.year()
    };
    NaiveDate::from_ymd_opt(year, month, 1)
}

fn parse_two_word_date(first: &str, second: &str, today: NaiveDate) -> Option<NaiveDate> {
    if let Some(weekday) = parse_weekday(second) {
        let week_offset = match first {
            "next" => 1,
            "last" | "previous" => -1,
            "this" => 0,
            _ => return None,
        };
        return Some(weekday_in_week(today, weekday, week_offset));
    }
    if let Some(month) = parse_month(first) {
        let day = parse_day_number(second)?;
        return NaiveDate::from_ymd_opt(today.year(), month, day);
    }
    if let Some(month) = parse_month(second) {
        let day = parse_day_number(first)?;
        return NaiveDate::from_ymd_opt(today.year(), month, day);
    }
    None
}

fn parse_three_word_date(
    first: &str,
    second: &str,
    third: &str,
    today: NaiveDate,
) -> Option<NaiveDate> {
    let year = parse_year(third)?;
    if let Some(month) = parse_month(first) {
        let day = parse_day_number(second)?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    if let Some(month) = parse_month(second) {
        let day = parse_day_number(first)?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    let _ = today;
    None
}

fn shift_by_unit(today: NaiveDate, unit: &str, direction: i64) -> Option<NaiveDate> {
    match unit.trim_end_matches('s') {
        "day" => Some(today + Duration::days(direction)),
        "week" => Some(today + Duration::days(7 * direction)),
        "month" => shift_months(today, direction),
        "year" => shift_months(today, 12 * direction),
        "weekend" => Some(weekday_in_week(today, Weekday::Sat, direction)),
        other => parse_weekday(other).map(|weekday| weekday_in_week(today, weekday, direction)),
    }
}

fn shift_by_unit_count(today: NaiveDate, unit: &str, count: i64) -> Option<NaiveDate> {
    match unit.trim_end_matches('s') {
        "day" => Some(today + Duration::days(count)),
        "week" => Some(today + Duration::days(7 * count)),
        "month" => shift_months(today, count),
        "year" => shift_months(today, 12 * count),
        "hour" | "minute" => Some(today),
        _ => None,
    }
}

fn shift_months(today: NaiveDate, months: i64) -> Option<NaiveDate> {
    let magnitude = u32::try_from(months.unsigned_abs()).ok()?;
    if months >= 0 {
        today.checked_add_months(Months::new(magnitude))
    } else {
        today.checked_sub_months(Months::new(magnitude))
    }
}

/// The weekday inside the Monday-to-Sunday week `week_offset` weeks away. A
/// bare weekday stays in the current week even when it has already passed,
/// which is how Notion resolves `mon` on a Wednesday.
fn weekday_in_week(today: NaiveDate, weekday: Weekday, week_offset: i64) -> NaiveDate {
    let week_start = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    week_start
        + Duration::days(7 * week_offset)
        + Duration::days(i64::from(weekday.num_days_from_monday()))
}

/// Any prefix of a weekday name from three letters up, so a half-typed `frid`
/// still resolves while the menu is open.
fn parse_weekday(word: &str) -> Option<Weekday> {
    const WEEKDAYS: [(&str, Weekday); 7] = [
        ("monday", Weekday::Mon),
        ("tuesday", Weekday::Tue),
        ("wednesday", Weekday::Wed),
        ("thursday", Weekday::Thu),
        ("friday", Weekday::Fri),
        ("saturday", Weekday::Sat),
        ("sunday", Weekday::Sun),
    ];
    if word.len() < 3 {
        return None;
    }
    WEEKDAYS
        .iter()
        .find(|(name, _)| name.starts_with(word))
        .map(|(_, weekday)| *weekday)
}

/// Any prefix of a month name from three letters up, matching how Notion
/// resolves `sep` and `aug` while they are still being typed.
fn parse_month(word: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let word = word.trim_end_matches('.');
    if word.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|name| name.starts_with(word))
        .map(|index| index as u32 + 1)
}

fn parse_day_number(word: &str) -> Option<u32> {
    let digits = word
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    let day = digits.parse::<u32>().ok()?;
    (1..=31).contains(&day).then_some(day)
}

fn parse_year(word: &str) -> Option<i32> {
    let year = word.parse::<i32>().ok()?;
    match word.len() {
        4 => Some(year),
        2 => Some(2000 + year),
        _ => None,
    }
}

fn parse_iso_date(word: &str) -> Option<NaiveDate> {
    let mut parts = word.split('-');
    let year = parts.next()?;
    let month = parts.next()?;
    let day = parts.next()?;
    if parts.next().is_some() || year.len() != 4 {
        return None;
    }
    NaiveDate::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
}

/// `M/D`, `M/D/YY` and `M/D/YYYY`; a year-less date rolls forward to its
/// next occurrence, matching Notion's handling of `1/5` typed in September.
fn parse_slash_date(word: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts = word.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        [month, day] => {
            let month = month.parse::<u32>().ok()?;
            let day = parse_day_number(day)?;
            let this_year = NaiveDate::from_ymd_opt(today.year(), month, day)?;
            if this_year < today {
                NaiveDate::from_ymd_opt(today.year() + 1, month, day)
            } else {
                Some(this_year)
            }
        }
        [month, day, year] => NaiveDate::from_ymd_opt(
            parse_year(year)?,
            month.parse().ok()?,
            parse_day_number(day)?,
        ),
        _ => None,
    }
}

/// Remove a trailing time expression such as `9am`, `9 am`, `9:30pm`,
/// `14:00`, `noon` or `midnight` from the word list.
fn take_trailing_time(words: &mut Vec<&str>) -> Option<NaiveTime> {
    let last = *words.last()?;
    if let Some(clock_time) = parse_time_word(last, None) {
        words.pop();
        return Some(clock_time);
    }
    if matches!(last, "am" | "pm" | "a.m." | "p.m.") && words.len() >= 2 {
        let meridiem = if last.starts_with('a') { "am" } else { "pm" };
        if let Some(clock_time) = parse_time_word(words[words.len() - 2], Some(meridiem)) {
            words.truncate(words.len() - 2);
            return Some(clock_time);
        }
    }
    None
}

fn parse_time_word(word: &str, forced_meridiem: Option<&str>) -> Option<NaiveTime> {
    match word {
        "noon" | "midday" => return NaiveTime::from_hms_opt(12, 0, 0),
        "midnight" => return NaiveTime::from_hms_opt(0, 0, 0),
        _ => {}
    }
    let (digits, meridiem) = if let Some(rest) = word.strip_suffix("am") {
        (rest, Some("am"))
    } else if let Some(rest) = word.strip_suffix("pm") {
        (rest, Some("pm"))
    } else if let Some(rest) = word.strip_suffix("a.m.") {
        (rest, Some("am"))
    } else if let Some(rest) = word.strip_suffix("p.m.") {
        (rest, Some("pm"))
    } else {
        (word, forced_meridiem)
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit() || c == ':') {
        return None;
    }
    let (hour, minute) = match digits.split_once(':') {
        Some((hour, minute)) => (hour.parse::<u32>().ok()?, minute.parse::<u32>().ok()?),
        None => {
            // Bare digits are a time only with an explicit meridiem ("9am");
            // otherwise they are a day number and belong to the date grammar.
            meridiem?;
            (digits.parse::<u32>().ok()?, 0)
        }
    };
    let hour = match meridiem {
        Some("am") => {
            if !(1..=12).contains(&hour) {
                return None;
            }
            hour % 12
        }
        Some("pm") => {
            if !(1..=12).contains(&hour) {
                return None;
            }
            hour % 12 + 12
        }
        _ => hour,
    };
    NaiveTime::from_hms_opt(hour, minute, 0)
}
