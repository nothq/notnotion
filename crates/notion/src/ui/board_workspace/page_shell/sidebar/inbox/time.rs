use chrono::{DateTime, Datelike, Local, Utc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InboxDateBucket {
    Today,
    ThisWeek,
    LastWeek,
    Older,
}

impl InboxDateBucket {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::ThisWeek => "This week",
            Self::LastWeek => "Last week",
            Self::Older => "Older",
        }
    }

    pub(super) const fn element_segment(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::ThisWeek => "this-week",
            Self::LastWeek => "last-week",
            Self::Older => "older",
        }
    }
}

pub(super) fn inbox_date_bucket(timestamp_ms: Option<u64>) -> InboxDateBucket {
    let Some(timestamp) = inbox_local_datetime(timestamp_ms) else {
        return InboxDateBucket::Older;
    };
    let today = Local::now().date_naive();
    let date = timestamp.date_naive();
    if date >= today {
        return InboxDateBucket::Today;
    }
    if date.iso_week() == today.iso_week() {
        return InboxDateBucket::ThisWeek;
    }
    let previous_week = today - chrono::Duration::weeks(1);
    if date.iso_week() == previous_week.iso_week() {
        return InboxDateBucket::LastWeek;
    }
    InboxDateBucket::Older
}

fn inbox_local_datetime(timestamp_ms: Option<u64>) -> Option<DateTime<Local>> {
    let timestamp_ms = i64::try_from(timestamp_ms?).ok()?;
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms).map(|time| time.with_timezone(&Local))
}

pub(super) fn inbox_time_label(timestamp_ms: u64) -> String {
    let Some(timestamp) = inbox_local_datetime(Some(timestamp_ms)) else {
        return String::new();
    };
    if timestamp.date_naive() == Local::now().date_naive() {
        timestamp.format("%-I:%M %p").to_string()
    } else {
        timestamp.format("%b %-d").to_string()
    }
}

pub(super) fn inbox_action_label(notification_type: &str) -> &'static str {
    match notification_type {
        "block-edited" | "collection-property-edited" => "edited",
        "collection-row-created" => "created",
        "collection-row-deleted" => "deleted",
        "access-requested" => "requested access to",
        "grouped-block-property-updated" | "block-property-edited" => "updated properties in",
        "grouped-commented" | "commented" => "commented in",
        "user-invited" => "invited you to",
        "user-mentioned" => "mentioned you in",
        notification_type if notification_type.contains("reminder") => "sent a reminder for",
        notification_type if notification_type.contains("mention") => "mentioned you in",
        notification_type if notification_type.contains("comment") => "commented in",
        notification_type if notification_type.contains("invite") => "invited you to",
        notification_type
            if notification_type.contains("edit") || notification_type.contains("update") =>
        {
            "updated"
        }
        _ => "sent an update in",
    }
}
