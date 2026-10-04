use serde_json::{json, Value};

use crate::model::DatabasePropertyFilterCondition;

use super::{
    checkbox_filter_value, date_point_filter_value, date_range_filter_value, number_filter_value,
    person_filter_value, relation_filter_value, select_filter_value, status_filter_value,
    text_filter_value,
};

pub(super) fn property_filter_condition_value(
    condition: &DatabasePropertyFilterCondition,
) -> Value {
    match condition {
        DatabasePropertyFilterCondition::Text(filter) => text_filter_value(filter),
        condition @ (DatabasePropertyFilterCondition::NumberEquals(_)
        | DatabasePropertyFilterCondition::NumberDoesNotEqual(_)
        | DatabasePropertyFilterCondition::NumberGreaterThan(_)
        | DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(_)
        | DatabasePropertyFilterCondition::NumberLessThan(_)
        | DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(_)
        | DatabasePropertyFilterCondition::NumberIsEmpty
        | DatabasePropertyFilterCondition::NumberIsNotEmpty) => number_condition_value(condition),
        condition @ (DatabasePropertyFilterCondition::CheckboxIs(_)
        | DatabasePropertyFilterCondition::CheckboxIsNot(_)) => checkbox_condition_value(condition),
        condition @ (DatabasePropertyFilterCondition::PersonContains(_)
        | DatabasePropertyFilterCondition::PersonDoesNotContain(_)
        | DatabasePropertyFilterCondition::PersonIsEmpty
        | DatabasePropertyFilterCondition::PersonIsNotEmpty) => person_condition_value(condition),
        condition @ (DatabasePropertyFilterCondition::RelationContains(_)
        | DatabasePropertyFilterCondition::RelationDoesNotContain(_)
        | DatabasePropertyFilterCondition::RelationIsEmpty
        | DatabasePropertyFilterCondition::RelationIsNotEmpty) => {
            relation_condition_value(condition)
        }
        condition @ (DatabasePropertyFilterCondition::DateIs(_)
        | DatabasePropertyFilterCondition::DateIsBefore(_)
        | DatabasePropertyFilterCondition::DateIsAfter(_)
        | DatabasePropertyFilterCondition::DateIsOnOrBefore(_)
        | DatabasePropertyFilterCondition::DateIsOnOrAfter(_)
        | DatabasePropertyFilterCondition::DateIsWithin(_)
        | DatabasePropertyFilterCondition::DateIsRelativeTo(_)
        | DatabasePropertyFilterCondition::DateIsEmpty
        | DatabasePropertyFilterCondition::DateIsNotEmpty) => date_condition_value(condition),
        condition @ (DatabasePropertyFilterCondition::SelectIs(_)
        | DatabasePropertyFilterCondition::SelectIsNot(_)
        | DatabasePropertyFilterCondition::SelectContains(_)
        | DatabasePropertyFilterCondition::SelectDoesNotContain(_)
        | DatabasePropertyFilterCondition::SelectIsEmpty
        | DatabasePropertyFilterCondition::SelectIsNotEmpty) => select_condition_value(condition),
        condition @ (DatabasePropertyFilterCondition::StatusIs(_)
        | DatabasePropertyFilterCondition::StatusIsNot(_)) => status_condition_value(condition),
    }
}

fn number_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::NumberEquals(value) => {
            number_filter_value("number_equals", value)
        }
        DatabasePropertyFilterCondition::NumberDoesNotEqual(value) => {
            number_filter_value("number_does_not_equal", value)
        }
        DatabasePropertyFilterCondition::NumberGreaterThan(value) => {
            number_filter_value("number_greater_than", value)
        }
        DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(value) => {
            number_filter_value("number_greater_than_or_equal_to", value)
        }
        DatabasePropertyFilterCondition::NumberLessThan(value) => {
            number_filter_value("number_less_than", value)
        }
        DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(value) => {
            number_filter_value("number_less_than_or_equal_to", value)
        }
        DatabasePropertyFilterCondition::NumberIsEmpty => empty_filter_value(false),
        DatabasePropertyFilterCondition::NumberIsNotEmpty => empty_filter_value(true),
        _ => unreachable!("number filter dispatch must receive a number condition"),
    }
}

fn checkbox_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::CheckboxIs(checked) => {
            checkbox_filter_value("checkbox_is", *checked)
        }
        DatabasePropertyFilterCondition::CheckboxIsNot(checked) => {
            checkbox_filter_value("checkbox_is_not", *checked)
        }
        _ => unreachable!("checkbox filter dispatch must receive a checkbox condition"),
    }
}

fn person_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::PersonContains(person) => {
            person_filter_value("person_contains", person)
        }
        DatabasePropertyFilterCondition::PersonDoesNotContain(person) => {
            person_filter_value("person_does_not_contain", person)
        }
        DatabasePropertyFilterCondition::PersonIsEmpty => empty_filter_value(false),
        DatabasePropertyFilterCondition::PersonIsNotEmpty => empty_filter_value(true),
        _ => unreachable!("person filter dispatch must receive a person condition"),
    }
}

fn relation_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::RelationContains(page_ids) => {
            relation_filter_value("relation_contains", page_ids)
        }
        DatabasePropertyFilterCondition::RelationDoesNotContain(page_ids) => {
            relation_filter_value("relation_does_not_contain", page_ids)
        }
        DatabasePropertyFilterCondition::RelationIsEmpty => empty_filter_value(false),
        DatabasePropertyFilterCondition::RelationIsNotEmpty => empty_filter_value(true),
        _ => unreachable!("relation filter dispatch must receive a relation condition"),
    }
}

fn date_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::DateIs(date) => date_point_filter_value("date_is", date),
        DatabasePropertyFilterCondition::DateIsBefore(date) => {
            date_point_filter_value("date_is_before", date)
        }
        DatabasePropertyFilterCondition::DateIsAfter(date) => {
            date_point_filter_value("date_is_after", date)
        }
        DatabasePropertyFilterCondition::DateIsOnOrBefore(date) => {
            date_point_filter_value("date_is_on_or_before", date)
        }
        DatabasePropertyFilterCondition::DateIsOnOrAfter(date) => {
            date_point_filter_value("date_is_on_or_after", date)
        }
        DatabasePropertyFilterCondition::DateIsWithin(date) => {
            date_range_filter_value("date_is_within", date)
        }
        DatabasePropertyFilterCondition::DateIsRelativeTo(date) => {
            date_range_filter_value("date_is_relative_to", date)
        }
        DatabasePropertyFilterCondition::DateIsEmpty => empty_filter_value(false),
        DatabasePropertyFilterCondition::DateIsNotEmpty => empty_filter_value(true),
        _ => unreachable!("date filter dispatch must receive a date condition"),
    }
}

fn select_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::SelectIs(options) => {
            select_filter_value("enum_is", options)
        }
        DatabasePropertyFilterCondition::SelectIsNot(options) => {
            select_filter_value("enum_is_not", options)
        }
        DatabasePropertyFilterCondition::SelectContains(options) => {
            select_filter_value("enum_contains", options)
        }
        DatabasePropertyFilterCondition::SelectDoesNotContain(options) => {
            select_filter_value("enum_does_not_contain", options)
        }
        DatabasePropertyFilterCondition::SelectIsEmpty => empty_filter_value(false),
        DatabasePropertyFilterCondition::SelectIsNotEmpty => empty_filter_value(true),
        _ => unreachable!("select filter dispatch must receive a select condition"),
    }
}

fn status_condition_value(condition: &DatabasePropertyFilterCondition) -> Value {
    match condition {
        DatabasePropertyFilterCondition::StatusIs(statuses) => {
            status_filter_value("status_is", statuses)
        }
        DatabasePropertyFilterCondition::StatusIsNot(statuses) => {
            status_filter_value("status_is_not", statuses)
        }
        _ => unreachable!("status filter dispatch must receive a status condition"),
    }
}

fn empty_filter_value(not_empty: bool) -> Value {
    if not_empty {
        json!({ "operator": "is_not_empty" })
    } else {
        json!({ "operator": "is_empty" })
    }
}
