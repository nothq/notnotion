use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A reminder menu label and the reminder it selects, if any.
type ReminderMenuOption = (&'static str, Option<PageMentionReminder>);

/// A reminder attached to a date mention, as Notion stores it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageMentionReminder {
    pub unit: String,
    pub value: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

impl PageMentionReminder {
    /// The choices Notion offers for a date without a time.
    pub fn day_options() -> [ReminderMenuOption; 5] {
        [
            ("None", None),
            (
                "On day of event (9:00 AM)",
                Some(Self::days_before(0, Some("09:00"))),
            ),
            (
                "1 day before (9:00 AM)",
                Some(Self::days_before(1, Some("09:00"))),
            ),
            (
                "2 days before (9:00 AM)",
                Some(Self::days_before(2, Some("09:00"))),
            ),
            (
                "1 week before (9:00 AM)",
                Some(Self::days_before(7, Some("09:00"))),
            ),
        ]
    }

    /// The choices Notion offers for a date with a time.
    pub fn time_options() -> [ReminderMenuOption; 8] {
        [
            ("None", None),
            ("At time of event", Some(Self::minutes_before(0))),
            ("5 minutes before", Some(Self::minutes_before(5))),
            ("10 minutes before", Some(Self::minutes_before(10))),
            ("15 minutes before", Some(Self::minutes_before(15))),
            ("30 minutes before", Some(Self::minutes_before(30))),
            ("1 hour before", Some(Self::hours_before(1))),
            ("2 hours before", Some(Self::hours_before(2))),
        ]
    }

    pub fn days_before(days: i64, reminder_clock: Option<&str>) -> Self {
        Self {
            unit: "day".to_string(),
            value: days,
            time: reminder_clock.map(str::to_string),
        }
    }

    pub fn minutes_before(minutes: i64) -> Self {
        Self {
            unit: "minute".to_string(),
            value: minutes,
            time: None,
        }
    }

    pub fn hours_before(hours: i64) -> Self {
        Self {
            unit: "hour".to_string(),
            value: hours,
            time: None,
        }
    }

    pub fn menu_label(&self, with_time: bool) -> String {
        let options: Vec<ReminderMenuOption> = if with_time {
            Self::time_options().into_iter().collect()
        } else {
            Self::day_options().into_iter().collect()
        };
        options
            .iter()
            .find(|(_, option)| option.as_ref() == Some(self))
            .map(|(label, _)| (*label).to_string())
            .unwrap_or_else(|| {
                let unit = if self.value == 1 {
                    self.unit.clone()
                } else {
                    format!("{}s", self.unit)
                };
                format!("{} {unit} before", self.value)
            })
    }

    pub(super) fn from_notion_value(value: &Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| "Notion date reminder must be an object".to_string())?;
        let unit = object
            .get("unit")
            .and_then(Value::as_str)
            .ok_or_else(|| "Notion date reminder requires a unit".to_string())?
            .to_string();
        let value = object
            .get("value")
            .and_then(Value::as_i64)
            .ok_or_else(|| "Notion date reminder requires an integer value".to_string())?;
        let reminder_clock = match object.get("time") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => Some(value.clone()),
            Some(_) => return Err("Notion date reminder time must be a string".to_string()),
        };
        Ok(Self {
            unit,
            value,
            time: reminder_clock,
        })
    }

    pub(super) fn to_notion_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("unit".to_string(), Value::String(self.unit.clone()));
        object.insert("value".to_string(), Value::Number(self.value.into()));
        if let Some(reminder_clock) = &self.time {
            object.insert("time".to_string(), Value::String(reminder_clock.clone()));
        }
        Value::Object(object)
    }
}
