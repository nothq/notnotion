use crate::model::DatabasePropertyFilterCondition as Condition;
use crate::ui::board_workspace::dialogs::filter::date_helpers::{
    database_date_point_label, database_date_range_label,
};
use crate::ui::board_workspace::dialogs::filter::prelude::*;

pub(super) fn database_filter_chip_label(
    property_label: &str,
    condition: &DatabasePropertyFilterCondition,
) -> String {
    match condition {
        Condition::Text(filter) => text_filter_chip_label(property_label, filter),
        number @ (Condition::NumberEquals(_)
        | Condition::NumberDoesNotEqual(_)
        | Condition::NumberGreaterThan(_)
        | Condition::NumberGreaterThanOrEqualTo(_)
        | Condition::NumberLessThan(_)
        | Condition::NumberLessThanOrEqualTo(_)) => {
            number_filter_chip_label(property_label, number)
        }
        checkbox @ (Condition::CheckboxIs(_) | Condition::CheckboxIsNot(_)) => {
            checkbox_filter_chip_label(property_label, checkbox)
        }
        Condition::PersonContains(value) | Condition::PersonDoesNotContain(value) => {
            person_filter_chip_label(property_label, value)
        }
        Condition::RelationContains(values) | Condition::RelationDoesNotContain(values) => {
            relation_filter_chip_label(property_label, values)
        }
        Condition::DateIs(value)
        | Condition::DateIsBefore(value)
        | Condition::DateIsAfter(value)
        | Condition::DateIsOnOrBefore(value)
        | Condition::DateIsOnOrAfter(value) => format!(
            "{property_label}: {}",
            database_date_point_label(value.value())
        ),
        Condition::DateIsWithin(value) | Condition::DateIsRelativeTo(value) => format!(
            "{property_label}: {}",
            database_date_range_label(value.value())
        ),
        Condition::SelectIs(values)
        | Condition::SelectIsNot(values)
        | Condition::SelectContains(values)
        | Condition::SelectDoesNotContain(values) => {
            format!("{property_label}: {}", values.as_slice().join(", "))
        }
        Condition::StatusIs(values) | Condition::StatusIsNot(values) => {
            status_filter_chip_label(property_label, values)
        }
        empty @ (Condition::PersonIsEmpty
        | Condition::NumberIsEmpty
        | Condition::RelationIsEmpty
        | Condition::DateIsEmpty
        | Condition::SelectIsEmpty
        | Condition::PersonIsNotEmpty
        | Condition::NumberIsNotEmpty
        | Condition::RelationIsNotEmpty
        | Condition::DateIsNotEmpty
        | Condition::SelectIsNotEmpty) => empty_filter_chip_label(property_label, empty),
    }
}

fn empty_filter_chip_label(
    property_label: &str,
    condition: &DatabasePropertyFilterCondition,
) -> String {
    let not_empty = matches!(
        condition,
        DatabasePropertyFilterCondition::PersonIsNotEmpty
            | DatabasePropertyFilterCondition::NumberIsNotEmpty
            | DatabasePropertyFilterCondition::RelationIsNotEmpty
            | DatabasePropertyFilterCondition::DateIsNotEmpty
            | DatabasePropertyFilterCondition::SelectIsNotEmpty
    );
    if not_empty {
        format!("{property_label} is not empty")
    } else {
        format!("{property_label} is empty")
    }
}

fn text_filter_chip_label(property_label: &str, filter: &DatabaseTextFilter) -> String {
    match filter {
        DatabaseTextFilter::Contains(value) => format!("{property_label}: {}", value.as_str()),
        DatabaseTextFilter::Is(value) => format!("{property_label} is {}", value.as_str()),
        DatabaseTextFilter::IsNot(value) => format!("{property_label} is not {}", value.as_str()),
        DatabaseTextFilter::DoesNotContain(value) => {
            format!("{property_label} does not contain {}", value.as_str())
        }
        DatabaseTextFilter::StartsWith(value) => {
            format!("{property_label} starts with {}", value.as_str())
        }
        DatabaseTextFilter::EndsWith(value) => {
            format!("{property_label} ends with {}", value.as_str())
        }
        DatabaseTextFilter::IsEmpty => format!("{property_label} is empty"),
        DatabaseTextFilter::IsNotEmpty => format!("{property_label} is not empty"),
    }
}

fn number_filter_chip_label(
    property_label: &str,
    condition: &DatabasePropertyFilterCondition,
) -> String {
    match condition {
        DatabasePropertyFilterCondition::NumberEquals(value) => {
            format!("{property_label} = {value}")
        }
        DatabasePropertyFilterCondition::NumberDoesNotEqual(value) => {
            format!("{property_label} ≠ {value}")
        }
        DatabasePropertyFilterCondition::NumberGreaterThan(value) => {
            format!("{property_label} > {value}")
        }
        DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(value) => {
            format!("{property_label} ≥ {value}")
        }
        DatabasePropertyFilterCondition::NumberLessThan(value) => {
            format!("{property_label} < {value}")
        }
        DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(value) => {
            format!("{property_label} ≤ {value}")
        }
        _ => unreachable!("number label helper receives only number conditions"),
    }
}

fn checkbox_filter_chip_label(
    property_label: &str,
    condition: &DatabasePropertyFilterCondition,
) -> String {
    match condition {
        DatabasePropertyFilterCondition::CheckboxIs(checked) => format!(
            "{property_label}: {}",
            if *checked { "Checked" } else { "Unchecked" }
        ),
        DatabasePropertyFilterCondition::CheckboxIsNot(checked) => format!(
            "{property_label} is not {}",
            if *checked { "checked" } else { "unchecked" }
        ),
        _ => unreachable!("checkbox label helper receives only checkbox conditions"),
    }
}

fn person_filter_chip_label(property_label: &str, value: &DatabasePersonFilter) -> String {
    match value {
        DatabasePersonFilter::CurrentUser => format!("{property_label}: Me"),
        DatabasePersonFilter::Users(users) => {
            format!("{property_label}: {} people", users.as_slice().len())
        }
        DatabasePersonFilter::CurrentUserAndUsers(users) => {
            format!("{property_label}: Me + {} people", users.as_slice().len())
        }
    }
}

fn relation_filter_chip_label<T>(
    property_label: &str,
    values: &NonEmptyDatabaseFilterValues<T>,
) -> String {
    let count = values.as_slice().len();
    let noun = if count == 1 { "page" } else { "pages" };
    format!("{property_label}: {count} {noun}")
}

fn status_filter_chip_label(
    property_label: &str,
    values: &NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>,
) -> String {
    let labels = values
        .as_slice()
        .iter()
        .map(|value| match value {
            DatabaseStatusFilterValue::Option(value) | DatabaseStatusFilterValue::Group(value) => {
                value.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{property_label}: {labels}")
}

pub(super) fn database_filter_entry_is_dirty(
    filter: &DatabaseSimpleFilterState,
    persisted: &DatabaseViewFilterState,
) -> bool {
    let DatabaseSimpleFiltersState::Entries(persisted_filters) = persisted.simple() else {
        return true;
    };
    persisted_filters
        .iter()
        .find(|persisted_filter| persisted_filter.filter_id() == filter.filter_id())
        != Some(filter)
}

pub(super) fn database_filter_property_type(property_type: &str) -> bool {
    matches!(
        property_type,
        "title"
            | "text"
            | "url"
            | "email"
            | "phone_number"
            | "number"
            | "checkbox"
            | "person"
            | "relation"
            | "date"
            | "select"
            | "multi_select"
            | "status"
    )
}

pub(super) fn new_database_filter_id() -> crate::model::NotionDatabaseFilterId {
    crate::model::NotionDatabaseFilterId::try_from(uuid::Uuid::new_v4().to_string())
        .expect("generated Notion filter IDs are non-empty")
}

pub(super) fn default_database_property_filter(
    property: &DatabaseProperty,
) -> Option<DatabasePropertyFilter> {
    let condition = match property.filter_type() {
        "title" | "text" | "url" | "email" | "phone_number" => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::IsEmpty)
        }
        "number" => DatabasePropertyFilterCondition::NumberIsEmpty,
        "checkbox" => DatabasePropertyFilterCondition::CheckboxIs(true),
        "person" => {
            DatabasePropertyFilterCondition::PersonContains(DatabasePersonFilter::CurrentUser)
        }
        "relation" => DatabasePropertyFilterCondition::RelationIsEmpty,
        "date" => DatabasePropertyFilterCondition::DateIsRelativeTo(DatabaseDateFilter::new(
            DatabaseDateRange::Surrounding {
                unit: DatabaseRelativeDateUnit::Week,
            },
            DatabaseDateFilterMode::StartDate,
        )),
        "select" => match property.options.first() {
            Some(option) => DatabasePropertyFilterCondition::SelectIs(
                NonEmptyDatabaseFilterValues::one(option.value.clone()),
            ),
            None => DatabasePropertyFilterCondition::SelectIsEmpty,
        },
        "multi_select" => match property.options.first() {
            Some(option) => DatabasePropertyFilterCondition::SelectContains(
                NonEmptyDatabaseFilterValues::one(option.value.clone()),
            ),
            None => DatabasePropertyFilterCondition::SelectIsEmpty,
        },
        "status" => {
            let value = property
                .status_groups
                .first()
                .map(|group| DatabaseStatusFilterValue::Group(group.name.clone()))
                .or_else(|| {
                    property
                        .options
                        .first()
                        .map(|option| DatabaseStatusFilterValue::Option(option.value.clone()))
                })?;
            DatabasePropertyFilterCondition::StatusIs(NonEmptyDatabaseFilterValues::one(value))
        }
        _ => return None,
    };
    DatabasePropertyFilter::new(
        NotionDatabasePropertyId::try_from(property.property_id.clone())
            .expect("loaded Notion database property IDs are non-empty"),
        condition,
    )
    .ok()
}
