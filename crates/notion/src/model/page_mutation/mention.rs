use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

mod label;
mod parse;
mod reminder;
mod request;

pub use label::{mention_date_label, relative_day_label};
pub use parse::{parse_mention_date_query, ParsedMentionDate};
pub use reminder::PageMentionReminder;
pub use request::{InsertPageMentionRequest, UpdatePageMentionRequest};

#[cfg(test)]
mod tests;

/// The character Notion stores for every inline mention (U+2023).
///
/// notnotion keeps this exact character in the editable text so that UTF-8 and
/// UTF-16 offsets stay identical to the CRDT title; the label is a display
/// concern of the text input.
pub const PAGE_MENTION_TOKEN: char = '\u{2023}';
pub const PAGE_MENTION_TOKEN_STR: &str = "\u{2023}";

const NOTION_DATE_INPUT_FORMAT: &str = "%Y-%m-%d";
const NOTION_TIME_INPUT_FORMAT: &str = "%H:%M";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageMentionKind {
    Date,
    User,
    Page,
}

impl PageMentionKind {
    /// Notion's rich-text annotation key for this mention kind.
    pub const fn annotation_key(self) -> &'static str {
        match self {
            Self::Date => "d",
            Self::User => "u",
            Self::Page => "p",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mention", rename_all = "snake_case")]
pub enum PageMention {
    Date(PageMentionDate),
    User {
        user_id: String,
        display_name: String,
    },
    Page {
        block_id: String,
        title: String,
    },
}

impl PageMention {
    pub const fn kind(&self) -> PageMentionKind {
        match self {
            Self::Date(_) => PageMentionKind::Date,
            Self::User { .. } => PageMentionKind::User,
            Self::Page { .. } => PageMentionKind::Page,
        }
    }

    pub fn date(&self) -> Option<&PageMentionDate> {
        match self {
            Self::Date(date) => Some(date),
            Self::User { .. } | Self::Page { .. } => None,
        }
    }

    /// The label Notion shows after the "@" for this mention on `today`.
    pub fn label(&self, today: NaiveDate) -> String {
        match self {
            Self::Date(date) => mention_date_label(date, today),
            Self::User { display_name, .. } => display_name.clone(),
            Self::Page { title, .. } => title.clone(),
        }
    }

    /// The Notion annotation tuple `["d", {...}]`, `["u", id]` or `["p", id]`.
    pub fn annotation_tuple(&self) -> Vec<Value> {
        match self {
            Self::Date(date) => vec![
                Value::String("d".to_string()),
                Value::Object(date.to_notion_value()),
            ],
            Self::User { user_id, .. } => vec![
                Value::String("u".to_string()),
                Value::String(user_id.clone()),
            ],
            Self::Page { block_id, .. } => vec![
                Value::String("p".to_string()),
                Value::String(block_id.clone()),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageMentionDateFormat {
    #[default]
    Relative,
    FullDate,
    MonthDayYear,
    DayMonthYear,
    YearMonthDay,
}

impl PageMentionDateFormat {
    pub const ALL: [Self; 5] = [
        Self::FullDate,
        Self::MonthDayYear,
        Self::DayMonthYear,
        Self::YearMonthDay,
        Self::Relative,
    ];

    /// The value Notion stores under `date_format`.
    pub const fn notion_value(self) -> &'static str {
        match self {
            Self::Relative => "relative",
            Self::FullDate => "ll",
            Self::MonthDayYear => "MM/DD/YYYY",
            Self::DayMonthYear => "DD/MM/YYYY",
            Self::YearMonthDay => "YYYY/MM/DD",
        }
    }

    pub fn from_notion_value(value: &str) -> Result<Self, String> {
        Ok(match value {
            "relative" => Self::Relative,
            "ll" => Self::FullDate,
            "MM/DD/YYYY" => Self::MonthDayYear,
            "DD/MM/YYYY" => Self::DayMonthYear,
            "YYYY/MM/DD" => Self::YearMonthDay,
            other => return Err(format!("unsupported Notion date format {other}")),
        })
    }

    /// The label Notion's date picker shows for this format.
    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::Relative => "Relative",
            Self::FullDate => "Full date",
            Self::MonthDayYear => "Month/Day/Year",
            Self::DayMonthYear => "Day/Month/Year",
            Self::YearMonthDay => "Year/Month/Day",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageMentionTimeFormat {
    #[default]
    TwelveHour,
    TwentyFourHour,
}

impl PageMentionTimeFormat {
    pub const ALL: [Self; 2] = [Self::TwelveHour, Self::TwentyFourHour];

    pub const fn notion_value(self) -> &'static str {
        match self {
            Self::TwelveHour => "LT",
            Self::TwentyFourHour => "H:mm",
        }
    }

    pub fn from_notion_value(value: &str) -> Result<Self, String> {
        Ok(match value {
            "LT" => Self::TwelveHour,
            "H:mm" => Self::TwentyFourHour,
            other => return Err(format!("unsupported Notion time format {other}")),
        })
    }

    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::TwelveHour => "12 hour",
            Self::TwentyFourHour => "24 hour",
        }
    }
}

/// Notion's date object stored under the `d` annotation of a `‣` token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageMentionDate {
    pub start_date: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub date_format: PageMentionDateFormat,
    #[serde(default)]
    pub time_format: PageMentionTimeFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder: Option<PageMentionReminder>,
    /// Keys of Notion's date object this client does not model, preserved so
    /// an edited mention round-trips without dropping upstream data.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
}

const KNOWN_DATE_KEYS: [&str; 9] = [
    "type",
    "start_date",
    "start_time",
    "end_date",
    "end_time",
    "time_zone",
    "date_format",
    "time_format",
    "reminder",
];

impl PageMentionDate {
    /// A plain relative date mention, the shape the mention menu inserts.
    pub fn day(start_date: NaiveDate) -> Self {
        Self {
            start_date,
            start_time: None,
            end_date: None,
            end_time: None,
            time_zone: None,
            date_format: PageMentionDateFormat::Relative,
            time_format: PageMentionTimeFormat::TwelveHour,
            reminder: None,
            extra: Map::new(),
        }
    }

    /// A relative date-time mention in the given IANA zone.
    pub fn day_time(start_date: NaiveDate, start_time: NaiveTime, time_zone: String) -> Self {
        Self {
            start_time: Some(start_time),
            time_zone: Some(time_zone),
            ..Self::day(start_date)
        }
    }

    pub const fn has_time(&self) -> bool {
        self.start_time.is_some()
    }

    pub const fn has_end(&self) -> bool {
        self.end_date.is_some()
    }

    /// Notion's `type` discriminator for this shape.
    pub const fn notion_type(&self) -> &'static str {
        match (self.has_time(), self.has_end()) {
            (false, false) => "date",
            (true, false) => "datetime",
            (false, true) => "daterange",
            (true, true) => "datetimerange",
        }
    }

    pub fn from_notion_value(value: &Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| "Notion date mention must be an object".to_string())?;
        let start_date = required_date(object, "start_date")?;
        let start_time = optional_time(object, "start_time")?;
        let end_date = optional_date(object, "end_date")?;
        let end_time = optional_time(object, "end_time")?;
        if end_time.is_some() && end_date.is_none() {
            return Err("Notion date mention has an end time without an end date".to_string());
        }
        let time_zone = match object.get("time_zone") {
            None | Some(Value::Null) => None,
            Some(Value::String(zone)) => Some(zone.clone()),
            Some(_) => return Err("Notion date mention time_zone must be a string".to_string()),
        };
        let date_format = match object.get("date_format") {
            None | Some(Value::Null) => PageMentionDateFormat::Relative,
            Some(Value::String(format)) => PageMentionDateFormat::from_notion_value(format)?,
            Some(_) => return Err("Notion date mention date_format must be a string".to_string()),
        };
        let time_format = match object.get("time_format") {
            None | Some(Value::Null) => PageMentionTimeFormat::TwelveHour,
            Some(Value::String(format)) => PageMentionTimeFormat::from_notion_value(format)?,
            Some(_) => return Err("Notion date mention time_format must be a string".to_string()),
        };
        let reminder = match object.get("reminder") {
            None | Some(Value::Null) => None,
            Some(value) => Some(PageMentionReminder::from_notion_value(value)?),
        };
        let extra = object
            .iter()
            .filter(|(key, _)| !KNOWN_DATE_KEYS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        Ok(Self {
            start_date,
            start_time,
            end_date,
            end_time,
            time_zone,
            date_format,
            time_format,
            reminder,
            extra,
        })
    }

    pub fn to_notion_value(&self) -> Map<String, Value> {
        let mut object = Map::new();
        object.insert(
            "type".to_string(),
            Value::String(self.notion_type().to_string()),
        );
        object.insert(
            "start_date".to_string(),
            Value::String(self.start_date.format(NOTION_DATE_INPUT_FORMAT).to_string()),
        );
        if let Some(start_time) = self.start_time {
            object.insert(
                "start_time".to_string(),
                Value::String(start_time.format(NOTION_TIME_INPUT_FORMAT).to_string()),
            );
        }
        if let Some(end_date) = self.end_date {
            object.insert(
                "end_date".to_string(),
                Value::String(end_date.format(NOTION_DATE_INPUT_FORMAT).to_string()),
            );
        }
        if let Some(end_time) = self.end_time {
            object.insert(
                "end_time".to_string(),
                Value::String(end_time.format(NOTION_TIME_INPUT_FORMAT).to_string()),
            );
        }
        if let Some(time_zone) = &self.time_zone {
            object.insert("time_zone".to_string(), Value::String(time_zone.clone()));
        }
        object.insert(
            "date_format".to_string(),
            Value::String(self.date_format.notion_value().to_string()),
        );
        if self.has_time() {
            object.insert(
                "time_format".to_string(),
                Value::String(self.time_format.notion_value().to_string()),
            );
        }
        if let Some(reminder) = &self.reminder {
            object.insert("reminder".to_string(), reminder.to_notion_value());
        }
        for (key, value) in &self.extra {
            object.insert(key.clone(), value.clone());
        }
        object
    }
}

fn required_date(object: &Map<String, Value>, key: &str) -> Result<NaiveDate, String> {
    optional_date(object, key)?.ok_or_else(|| format!("Notion date mention requires {key}"))
}

fn optional_date(object: &Map<String, Value>, key: &str) -> Result<Option<NaiveDate>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => NaiveDate::parse_from_str(value, NOTION_DATE_INPUT_FORMAT)
            .map(Some)
            .map_err(|error| format!("Notion date mention {key} {value:?} is invalid: {error}")),
        Some(_) => Err(format!("Notion date mention {key} must be a string")),
    }
}

fn optional_time(object: &Map<String, Value>, key: &str) -> Result<Option<NaiveTime>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => NaiveTime::parse_from_str(value, NOTION_TIME_INPUT_FORMAT)
            .map(Some)
            .map_err(|error| format!("Notion date mention {key} {value:?} is invalid: {error}")),
        Some(_) => Err(format!("Notion date mention {key} must be a string")),
    }
}
