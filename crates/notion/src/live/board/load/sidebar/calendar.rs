use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::json;

use super::super::super::{NotionPrivateApiEndpoint, UserContext};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use crate::model::{LoadSidebarCalendarResult, PageShellCalendarEvent};

const CALENDAR_LOOKBACK_DAYS: i64 = 2;
const CALENDAR_LOOKAHEAD_DAYS: i64 = 7;

#[derive(Deserialize)]
struct CalendarEventsResponse {
    events: Vec<CalendarEvent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEvent {
    id: String,
    url: String,
    summary: Option<String>,
    start: CalendarEventTime,
    end: CalendarEventTime,
    is_all_day: bool,
    response_status: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarEventTime {
    date_time: Option<DateTime<FixedOffset>>,
    date: Option<String>,
}

struct UpcomingCalendarEvent {
    id: String,
    url: String,
    title: String,
    start: DateTime<Tz>,
    end: DateTime<Tz>,
    all_day: bool,
}

struct CalendarEventOccurrence {
    event_id: String,
    url: String,
    title: String,
    date: NaiveDate,
    period: CalendarEventOccurrencePeriod,
}

enum CalendarEventOccurrencePeriod {
    AllDay,
    Timed {
        start: DateTime<Tz>,
        end: DateTime<Tz>,
    },
}

pub(crate) fn load_sidebar_calendar_for_context(
    session: &NotionDesktopSession,
    space_id: &str,
    user: &UserContext,
) -> Result<LoadSidebarCalendarResult, NotionLiveError> {
    let sidebar = user.sidebar_for_space(space_id)?;
    let Some(bot_id) = sidebar.selected_calendar_bot_id.as_deref() else {
        return Ok(LoadSidebarCalendarResult {
            available: false,
            events: Vec::new(),
        });
    };
    let time_zone = user
        .time_zone
        .parse::<Tz>()
        .map_err(|error| format!("invalid Notion user time zone: {error}"))?;
    let now = Utc::now();
    let fetch_anchor = now
        .with_second(0)
        .and_then(|date_time| date_time.with_nanosecond(0))
        .expect("calendar fetch time should round to a minute");
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::GetCalendarEvents,
        &json!({
            "botId": bot_id,
            "spaceId": space_id,
            "timeMin": (fetch_anchor - Duration::days(CALENDAR_LOOKBACK_DAYS)).timestamp_millis(),
            "timeMax": (fetch_anchor + Duration::days(CALENDAR_LOOKAHEAD_DAYS)).timestamp_millis(),
            "orderBy": "startTime",
            "calendarVisibilityOverrides": [],
            "userTimeZone": user.time_zone,
            "versionedReasons": {},
            "reason": "background_load",
            "unfurlGroups": true,
            "knownUserEmails": [],
        }),
    )?;
    let response = serde_json::from_value::<CalendarEventsResponse>(response)
        .map_err(|error| format!("invalid Notion private API calendar response: {error}"))?;
    let events = upcoming_events(response.events, now, time_zone)?;
    Ok(LoadSidebarCalendarResult {
        available: true,
        events: shape_calendar_events(events, now.with_timezone(&time_zone)),
    })
}

fn upcoming_events(
    events: Vec<CalendarEvent>,
    now: DateTime<Utc>,
    time_zone: Tz,
) -> Result<Vec<UpcomingCalendarEvent>, String> {
    let mut upcoming = Vec::new();
    for event in events {
        if event.response_status.as_deref() == Some("declined") {
            continue;
        }
        let (start, end) = calendar_event_range(&event, time_zone)?;
        if event.is_all_day {
            if end.date_naive() <= now.with_timezone(&time_zone).date_naive() {
                continue;
            }
        } else if end.with_timezone(&Utc) <= now {
            continue;
        }
        let title = event
            .summary
            .as_deref()
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        upcoming.push(UpcomingCalendarEvent {
            id: event.id,
            url: event.url,
            title: if title.is_empty() {
                "No title".to_string()
            } else {
                title
            },
            start,
            end,
            all_day: event.is_all_day,
        });
    }
    upcoming.sort_by_key(|event| event.start);
    Ok(upcoming)
}

/// The local start and end of a calendar event.
type CalendarEventRange = (DateTime<Tz>, DateTime<Tz>);

fn calendar_event_range(
    event: &CalendarEvent,
    time_zone: Tz,
) -> Result<CalendarEventRange, String> {
    if !event.is_all_day {
        let start =
            event.start.date_time.as_ref().ok_or_else(|| {
                format!("missing start time for Notion calendar event {}", event.id)
            })?;
        let end =
            event.end.date_time.as_ref().ok_or_else(|| {
                format!("missing end time for Notion calendar event {}", event.id)
            })?;
        return Ok((
            start.with_timezone(&time_zone),
            end.with_timezone(&time_zone),
        ));
    }
    let start = calendar_event_date(&event.start, &event.id, "start")?;
    let end = calendar_event_date(&event.end, &event.id, "end")?;
    Ok((
        local_midnight(start, time_zone, &event.id)?,
        local_midnight(end, time_zone, &event.id)?,
    ))
}

fn calendar_event_date(
    event_time: &CalendarEventTime,
    event_id: &str,
    boundary: &str,
) -> Result<NaiveDate, String> {
    let date = event_time.date.as_deref().ok_or_else(|| {
        format!("missing {boundary} date for all-day Notion calendar event {event_id}")
    })?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|error| {
        format!("invalid {boundary} date for all-day Notion calendar event {event_id}: {error}")
    })
}

fn local_midnight(date: NaiveDate, time_zone: Tz, event_id: &str) -> Result<DateTime<Tz>, String> {
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .expect("calendar date should have a midnight");
    time_zone
        .from_local_datetime(&midnight)
        .single()
        .ok_or_else(|| format!("ambiguous midnight for Notion calendar event {event_id}"))
}

fn shape_calendar_events(
    events: Vec<UpcomingCalendarEvent>,
    now: DateTime<Tz>,
) -> Vec<PageShellCalendarEvent> {
    let today = now.date_naive();
    let tomorrow = today
        .succ_opt()
        .expect("calendar date should have a successor");
    let mut occurrences = events
        .into_iter()
        .flat_map(calendar_event_occurrences)
        .collect::<Vec<_>>();
    occurrences.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| occurrence_rank(&left.period).cmp(&occurrence_rank(&right.period)))
    });
    let mut previous_date = None;
    occurrences
        .into_iter()
        .map(|occurrence| {
            let date = occurrence.date;
            let first_on_date = previous_date != Some(date);
            previous_date = Some(date);
            let time_label =
                occurrence_time_label(&occurrence.period, date, today, tomorrow, first_on_date);
            let ongoing = matches!(
                &occurrence.period,
                CalendarEventOccurrencePeriod::Timed { start, end }
                    if *start <= now && now < *end
            );
            PageShellCalendarEvent {
                event_id: occurrence.event_id,
                title: occurrence.title,
                time_label,
                target_url: occurrence.url,
                ongoing,
            }
        })
        .collect()
}

fn calendar_event_occurrences(event: UpcomingCalendarEvent) -> Vec<CalendarEventOccurrence> {
    if !event.all_day {
        return vec![CalendarEventOccurrence {
            event_id: event.id,
            url: event.url,
            title: event.title,
            date: event.start.date_naive(),
            period: CalendarEventOccurrencePeriod::Timed {
                start: event.start,
                end: event.end,
            },
        }];
    }
    let mut occurrences = Vec::new();
    let mut date = event.start.date_naive();
    let end_exclusive = event.end.date_naive();
    while date < end_exclusive {
        occurrences.push(CalendarEventOccurrence {
            event_id: format!("{}:{date}", event.id),
            url: event.url.clone(),
            title: event.title.clone(),
            date,
            period: CalendarEventOccurrencePeriod::AllDay,
        });
        date = date
            .succ_opt()
            .expect("calendar event date should have a successor");
    }
    occurrences
}

fn occurrence_rank(period: &CalendarEventOccurrencePeriod) -> i64 {
    match period {
        CalendarEventOccurrencePeriod::AllDay => 0,
        CalendarEventOccurrencePeriod::Timed { start, .. } => start.timestamp() + 1,
    }
}

fn occurrence_time_label(
    period: &CalendarEventOccurrencePeriod,
    date: NaiveDate,
    today: NaiveDate,
    tomorrow: NaiveDate,
    first_on_date: bool,
) -> String {
    match period {
        CalendarEventOccurrencePeriod::AllDay => {
            format_all_day_label(date, today, tomorrow, date.weekday(), first_on_date)
        }
        CalendarEventOccurrencePeriod::Timed { start, end } if date == today => {
            format_time_range(*start, *end)
        }
        CalendarEventOccurrencePeriod::Timed { start, .. } if !first_on_date => {
            format_clock(*start, true)
        }
        CalendarEventOccurrencePeriod::Timed { start, .. } if date == tomorrow => {
            format!("Tomorrow {}", format_clock(*start, true))
        }
        CalendarEventOccurrencePeriod::Timed { start, .. } => {
            format!("{} {}", start.weekday(), format_clock(*start, true))
        }
    }
}

fn format_all_day_label(
    date: NaiveDate,
    today: NaiveDate,
    tomorrow: NaiveDate,
    weekday: chrono::Weekday,
    first_on_date: bool,
) -> String {
    if date == today || !first_on_date {
        "All day".to_string()
    } else if date == tomorrow {
        "Tomorrow · All day".to_string()
    } else {
        format!("{weekday} · All day")
    }
}

fn format_time_range(start: DateTime<Tz>, end: DateTime<Tz>) -> String {
    if start.date_naive() != end.date_naive() {
        return format_clock(start, true);
    }
    let same_period = (start.hour() < 12) == (end.hour() < 12);
    let start = format_clock(start, !same_period);
    let end = format_clock(end, true);
    format!("{start}\u{2009}–\u{2009}{end}")
}

fn format_clock(date_time: DateTime<Tz>, include_period: bool) -> String {
    let hour = match date_time.hour() % 12 {
        0 => 12,
        hour => hour,
    };
    let mut clock = if date_time.minute() == 0 {
        hour.to_string()
    } else {
        format!("{hour}:{:02}", date_time.minute())
    };
    if include_period {
        clock.push('\u{2009}');
        clock.push_str(if date_time.hour() < 12 { "AM" } else { "PM" });
    }
    clock
}
