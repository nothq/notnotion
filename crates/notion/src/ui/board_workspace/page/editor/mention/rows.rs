use chrono::{Duration, NaiveDateTime, NaiveTime, Timelike};

use super::PageMentionClock;
use crate::model::{
    mention_date_label, parse_mention_date_query, relative_day_label, NotionWorkspaceUser,
    PageMentionDate, PageMentionDateFormat, PageMentionReminder, PageShellSearchResult,
    ParsedMentionDate,
};

const DEFAULT_REMINDER_TIME: (u32, u32) = (9, 0);

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PageMentionMenuRow {
    Date {
        label: String,
        sublabel: Option<String>,
        mention: PageMentionDate,
    },
    Reminder {
        sublabel: String,
        mention: PageMentionDate,
    },
    Person {
        user: NotionWorkspaceUser,
    },
    Invite {
        query: String,
    },
    Page {
        result: PageShellSearchResult,
    },
    NewPage {
        title: String,
    },
}

impl PageMentionMenuRow {
    /// The primary text the row shows, used for ghost completion.
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Date { label, .. } => label.clone(),
            Self::Reminder { .. } => "Remind me".to_string(),
            Self::Person { user } => user.name.clone(),
            Self::Invite { query } if query.is_empty() => "Invite…".to_string(),
            Self::Invite { query } => format!("Invite \"{query}\"…"),
            Self::Page { result } => result.title.clone(),
            Self::NewPage { title } => format!("New \"{title}\" page"),
        }
    }

    pub(crate) fn is_two_line(&self) -> bool {
        matches!(
            self,
            Self::Page { result } if result.highlight.as_deref().is_some_and(|caption| !caption.is_empty())
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PageMentionMenuSection {
    pub(crate) title: &'static str,
    pub(crate) rows: Vec<PageMentionMenuRow>,
}

pub(super) struct PageMentionMenuInputs<'a> {
    pub(super) query: &'a str,
    pub(super) clock: &'a PageMentionClock,
    pub(super) users: &'a [NotionWorkspaceUser],
    pub(super) pages: &'a [PageShellSearchResult],
}

/// Build the sections Notion shows for a query: Date, People, Link to page
/// and New page, in that order, omitting sections with nothing to offer.
pub(super) fn build_mention_menu_sections(
    inputs: PageMentionMenuInputs<'_>,
) -> Vec<PageMentionMenuSection> {
    let query = inputs.query.trim();
    let mut sections = Vec::with_capacity(4);
    if let Some(section) = date_section(query, inputs.clock) {
        sections.push(section);
    }
    sections.push(PageMentionMenuSection {
        title: "People",
        rows: people_rows(query, inputs.users),
    });
    if !inputs.pages.is_empty() {
        sections.push(PageMentionMenuSection {
            title: "Link to page",
            rows: inputs
                .pages
                .iter()
                .cloned()
                .map(|result| PageMentionMenuRow::Page { result })
                .collect(),
        });
    }
    if !query.is_empty() {
        sections.push(PageMentionMenuSection {
            title: "New page",
            rows: vec![PageMentionMenuRow::NewPage {
                title: query.to_string(),
            }],
        });
    }
    sections
}

fn date_section(query: &str, clock: &PageMentionClock) -> Option<PageMentionMenuSection> {
    let parsed = if query.is_empty() {
        ParsedMentionDate {
            date: clock.today,
            time: None,
        }
    } else {
        parse_mention_date_query(query, clock.today, clock.now)?
    };
    let reminder = reminder_row(parsed, query.is_empty(), clock);
    let mut rows = vec![date_row(parsed, reminder.is_none(), clock)];
    rows.extend(reminder);
    Some(PageMentionMenuSection {
        title: "Date",
        rows,
    })
}

/// Notion prints the full date once: on the date row when the query has no
/// reminder row, and on the reminder row otherwise.
fn date_row(
    parsed: ParsedMentionDate,
    with_full_date: bool,
    clock: &PageMentionClock,
) -> PageMentionMenuRow {
    let mention = match parsed.time {
        Some(time) => PageMentionDate::day_time(parsed.date, time, clock.time_zone.clone()),
        None => PageMentionDate::day(parsed.date),
    };
    let label = mention_date_label(&mention, clock.today);
    let sublabel = (with_full_date && parsed.time.is_none()).then(|| full_date(parsed.date, clock));
    PageMentionMenuRow::Date {
        label,
        sublabel,
        mention,
    }
}

/// Notion offers a reminder row only when the reminder instant is still ahead.
fn reminder_row(
    parsed: ParsedMentionDate,
    empty_query: bool,
    clock: &PageMentionClock,
) -> Option<PageMentionMenuRow> {
    let (date, time) = if empty_query {
        (clock.today + Duration::days(1), default_reminder_time())
    } else {
        (
            parsed.date,
            parsed.time.unwrap_or_else(default_reminder_time),
        )
    };
    let instant = NaiveDateTime::new(date, time);
    if instant <= NaiveDateTime::new(clock.today, clock.now) {
        return None;
    }
    let mut mention = PageMentionDate::day_time(date, time, clock.time_zone.clone());
    mention.reminder = Some(PageMentionReminder::minutes_before(0));
    let sublabel = if empty_query {
        format!(
            "{} {}",
            relative_day_label(date, clock.today),
            short_time(time)
        )
    } else {
        let mut absolute = mention.clone();
        absolute.date_format = PageMentionDateFormat::FullDate;
        if parsed.time.is_none() {
            absolute.start_time = None;
        }
        mention_date_label(&absolute, clock.today)
    };
    Some(PageMentionMenuRow::Reminder { sublabel, mention })
}

fn people_rows(query: &str, users: &[NotionWorkspaceUser]) -> Vec<PageMentionMenuRow> {
    let needle = query.to_lowercase();
    let mut rows = users
        .iter()
        .filter(|user| needle.is_empty() || user.name.to_lowercase().contains(&needle))
        .cloned()
        .map(|user| PageMentionMenuRow::Person { user })
        .collect::<Vec<_>>();
    rows.push(PageMentionMenuRow::Invite {
        query: query.to_string(),
    });
    rows
}

fn full_date(date: chrono::NaiveDate, clock: &PageMentionClock) -> String {
    let mut mention = PageMentionDate::day(date);
    mention.date_format = PageMentionDateFormat::FullDate;
    mention_date_label(&mention, clock.today)
}

fn default_reminder_time() -> NaiveTime {
    NaiveTime::from_hms_opt(DEFAULT_REMINDER_TIME.0, DEFAULT_REMINDER_TIME.1, 0)
        .expect("default reminder time is a valid clock time")
}

/// "9am" / "9:30pm": the compact time Notion prints in the empty-query
/// reminder row.
fn short_time(time: NaiveTime) -> String {
    let (is_pm, hour12) = time.hour12();
    let meridiem = if is_pm { "pm" } else { "am" };
    if time.minute() == 0 {
        format!("{hour12}{meridiem}")
    } else {
        format!("{hour12}:{:02}{meridiem}", time.minute())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_mention_menu_sections, PageMentionMenuInputs, PageMentionMenuRow,
        PageMentionMenuSection,
    };
    use crate::ui::board_workspace::page::editor::mention::PageMentionClock;
    use chrono::{NaiveDate, NaiveTime};

    fn clock() -> PageMentionClock {
        PageMentionClock {
            today: NaiveDate::from_ymd_opt(2026, 9, 2).expect("valid"),
            now: NaiveTime::from_hms_opt(12, 20, 0).expect("valid"),
            time_zone: "America/Toronto".to_string(),
        }
    }

    fn sections(query: &str) -> Vec<PageMentionMenuSection> {
        build_mention_menu_sections(PageMentionMenuInputs {
            query,
            clock: &clock(),
            users: &[],
            pages: &[],
        })
    }

    #[test]
    fn empty_query_offers_today_and_a_tomorrow_reminder() {
        let sections = sections("");
        assert_eq!(sections[0].title, "Date");
        assert_eq!(sections[0].rows[0].label(), "Today");
        assert!(matches!(
            &sections[0].rows[0],
            PageMentionMenuRow::Date { sublabel: None, .. }
        ));
        assert!(matches!(
            &sections[0].rows[1],
            PageMentionMenuRow::Reminder { sublabel, .. } if sublabel == "Tomorrow 9am"
        ));
        assert_eq!(sections[1].title, "People");
        assert_eq!(sections[1].rows[0].label(), "Invite…");
        assert_eq!(sections.len(), 2);
    }

    #[test]
    fn today_query_shows_the_full_date_and_no_past_reminder() {
        let sections = sections("today");
        let PageMentionMenuRow::Date {
            label, sublabel, ..
        } = &sections[0].rows[0]
        else {
            panic!("expected a date row");
        };
        assert_eq!(label, "Today");
        assert_eq!(sublabel.as_deref(), Some("September 2, 2026"));
        assert_eq!(sections[0].rows.len(), 1);
        assert_eq!(sections[1].rows[0].label(), "Invite \"today\"…");
        assert_eq!(
            sections.last().map(|section| section.title),
            Some("New page")
        );
    }

    #[test]
    fn explicit_date_query_shows_relative_label_and_reminder() {
        let sections = sections("sep 7");
        assert_eq!(sections[0].rows[0].label(), "Next Monday");
        assert!(matches!(
            &sections[0].rows[0],
            PageMentionMenuRow::Date { sublabel: None, .. }
        ));
        assert!(matches!(
            &sections[0].rows[1],
            PageMentionMenuRow::Reminder { sublabel, .. } if sublabel == "September 7, 2026"
        ));
    }

    #[test]
    fn unparsable_query_has_no_date_section() {
        let sections = sections("x");
        assert_eq!(sections[0].title, "People");
    }
}
