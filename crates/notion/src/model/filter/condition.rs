use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{
    NonEmptyDatabaseFilterValues, NotionDatabasePropertyId, NotionFilterPageId, NotionFilterUserId,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DatabasePersonFilter {
    CurrentUser,
    Users(NonEmptyDatabaseFilterValues<NotionFilterUserId>),
    CurrentUserAndUsers(NonEmptyDatabaseFilterValues<NotionFilterUserId>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DatabaseStatusFilterValue {
    Option(String),
    Group(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseDateFilterMode {
    StartDate,
    EndDate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseRelativeDatePreset {
    Today,
    Tomorrow,
    Yesterday,
    OneWeekAgo,
    OneWeekFromNow,
    OneMonthAgo,
    OneMonthFromNow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseRelativeDateDirection {
    Past,
    Future,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseRelativeDateUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DatabaseDatePoint {
    Exact(NaiveDate),
    Relative(DatabaseRelativeDatePreset),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DatabaseDateRange {
    Exact {
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    },
    Relative {
        direction: DatabaseRelativeDateDirection,
        count: u32,
        unit: DatabaseRelativeDateUnit,
    },
    Surrounding {
        unit: DatabaseRelativeDateUnit,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabaseDateFilter<T> {
    value: T,
    mode: DatabaseDateFilterMode,
}

impl<T> DatabaseDateFilter<T> {
    pub fn new(value: T, mode: DatabaseDateFilterMode) -> Self {
        Self { value, mode }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn mode(&self) -> DatabaseDateFilterMode {
        self.mode
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DatabaseTextFilterValue(String);

impl DatabaseTextFilterValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DatabaseTextFilterValue {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        require_non_empty_filter_value(&value, "text")?;
        Ok(Self(value))
    }
}

impl FromStr for DatabaseTextFilterValue {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl From<DatabaseTextFilterValue> for String {
    fn from(value: DatabaseTextFilterValue) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DatabaseTextFilter {
    Is(DatabaseTextFilterValue),
    IsNot(DatabaseTextFilterValue),
    Contains(DatabaseTextFilterValue),
    DoesNotContain(DatabaseTextFilterValue),
    StartsWith(DatabaseTextFilterValue),
    EndsWith(DatabaseTextFilterValue),
    IsEmpty,
    IsNotEmpty,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum DatabasePropertyFilterCondition {
    Text(DatabaseTextFilter),
    NumberEquals(serde_json::Number),
    NumberDoesNotEqual(serde_json::Number),
    NumberGreaterThan(serde_json::Number),
    NumberGreaterThanOrEqualTo(serde_json::Number),
    NumberLessThan(serde_json::Number),
    NumberLessThanOrEqualTo(serde_json::Number),
    NumberIsEmpty,
    NumberIsNotEmpty,
    CheckboxIs(bool),
    CheckboxIsNot(bool),
    PersonContains(DatabasePersonFilter),
    PersonDoesNotContain(DatabasePersonFilter),
    PersonIsEmpty,
    PersonIsNotEmpty,
    RelationContains(NonEmptyDatabaseFilterValues<NotionFilterPageId>),
    RelationDoesNotContain(NonEmptyDatabaseFilterValues<NotionFilterPageId>),
    RelationIsEmpty,
    RelationIsNotEmpty,
    DateIs(DatabaseDateFilter<DatabaseDatePoint>),
    DateIsBefore(DatabaseDateFilter<DatabaseDatePoint>),
    DateIsAfter(DatabaseDateFilter<DatabaseDatePoint>),
    DateIsOnOrBefore(DatabaseDateFilter<DatabaseDatePoint>),
    DateIsOnOrAfter(DatabaseDateFilter<DatabaseDatePoint>),
    DateIsWithin(DatabaseDateFilter<DatabaseDateRange>),
    DateIsRelativeTo(DatabaseDateFilter<DatabaseDateRange>),
    DateIsEmpty,
    DateIsNotEmpty,
    SelectIs(NonEmptyDatabaseFilterValues<String>),
    SelectIsNot(NonEmptyDatabaseFilterValues<String>),
    SelectContains(NonEmptyDatabaseFilterValues<String>),
    SelectDoesNotContain(NonEmptyDatabaseFilterValues<String>),
    SelectIsEmpty,
    SelectIsNotEmpty,
    StatusIs(NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>),
    StatusIsNot(NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DatabasePropertyFilter {
    property_id: NotionDatabasePropertyId,
    condition: DatabasePropertyFilterCondition,
}

impl DatabasePropertyFilter {
    pub fn new(
        property_id: NotionDatabasePropertyId,
        condition: DatabasePropertyFilterCondition,
    ) -> Result<Self, String> {
        validate_property_filter_condition(&condition)?;
        Ok(Self {
            property_id,
            condition,
        })
    }

    pub fn property_id(&self) -> &NotionDatabasePropertyId {
        &self.property_id
    }

    pub fn condition(&self) -> &DatabasePropertyFilterCondition {
        &self.condition
    }
}

fn validate_property_filter_condition(
    condition: &DatabasePropertyFilterCondition,
) -> Result<(), String> {
    match condition {
        DatabasePropertyFilterCondition::SelectIs(values)
        | DatabasePropertyFilterCondition::SelectIsNot(values)
        | DatabasePropertyFilterCondition::SelectContains(values)
        | DatabasePropertyFilterCondition::SelectDoesNotContain(values) => {
            validate_string_filter_values(values, "select option")
        }
        DatabasePropertyFilterCondition::StatusIs(values)
        | DatabasePropertyFilterCondition::StatusIsNot(values) => validate_status_values(values),
        DatabasePropertyFilterCondition::DateIsWithin(filter)
        | DatabasePropertyFilterCondition::DateIsRelativeTo(filter) => {
            validate_date_range(filter.value())
        }
        DatabasePropertyFilterCondition::Text(_)
        | DatabasePropertyFilterCondition::NumberEquals(_)
        | DatabasePropertyFilterCondition::NumberDoesNotEqual(_)
        | DatabasePropertyFilterCondition::NumberGreaterThan(_)
        | DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(_)
        | DatabasePropertyFilterCondition::NumberLessThan(_)
        | DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(_)
        | DatabasePropertyFilterCondition::NumberIsEmpty
        | DatabasePropertyFilterCondition::NumberIsNotEmpty
        | DatabasePropertyFilterCondition::CheckboxIs(_)
        | DatabasePropertyFilterCondition::CheckboxIsNot(_)
        | DatabasePropertyFilterCondition::PersonContains(_)
        | DatabasePropertyFilterCondition::PersonDoesNotContain(_)
        | DatabasePropertyFilterCondition::PersonIsEmpty
        | DatabasePropertyFilterCondition::PersonIsNotEmpty
        | DatabasePropertyFilterCondition::RelationContains(_)
        | DatabasePropertyFilterCondition::RelationDoesNotContain(_)
        | DatabasePropertyFilterCondition::RelationIsEmpty
        | DatabasePropertyFilterCondition::RelationIsNotEmpty
        | DatabasePropertyFilterCondition::DateIs(_)
        | DatabasePropertyFilterCondition::DateIsBefore(_)
        | DatabasePropertyFilterCondition::DateIsAfter(_)
        | DatabasePropertyFilterCondition::DateIsOnOrBefore(_)
        | DatabasePropertyFilterCondition::DateIsOnOrAfter(_)
        | DatabasePropertyFilterCondition::DateIsEmpty
        | DatabasePropertyFilterCondition::DateIsNotEmpty
        | DatabasePropertyFilterCondition::SelectIsEmpty
        | DatabasePropertyFilterCondition::SelectIsNotEmpty => Ok(()),
    }
}

fn validate_string_filter_values(
    values: &NonEmptyDatabaseFilterValues<String>,
    kind: &str,
) -> Result<(), String> {
    values
        .as_slice()
        .iter()
        .try_for_each(|value| require_non_empty_filter_value(value, kind))
}

fn validate_status_values(
    values: &NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>,
) -> Result<(), String> {
    values.as_slice().iter().try_for_each(|value| {
        let (value, kind) = match value {
            DatabaseStatusFilterValue::Option(value) => (value, "status option"),
            DatabaseStatusFilterValue::Group(value) => (value, "status group"),
        };
        require_non_empty_filter_value(value, kind)
    })
}

fn validate_date_range(range: &DatabaseDateRange) -> Result<(), String> {
    match range {
        DatabaseDateRange::Exact {
            start_date: None,
            end_date: None,
        } => Err("Notion exact database date range requires at least one date".to_string()),
        DatabaseDateRange::Relative { count: 0, .. } => {
            Err("Notion relative database date range count must be positive".to_string())
        }
        DatabaseDateRange::Exact { .. }
        | DatabaseDateRange::Relative { .. }
        | DatabaseDateRange::Surrounding { .. } => Ok(()),
    }
}

fn require_non_empty_filter_value(value: &str, kind: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("Notion database filter {kind} must not be empty"));
    }
    Ok(())
}
