use crate::ui::board_workspace::dialogs::filter::{prelude::*, types::*};

pub(super) fn database_simple_filter_from_draft(
    draft: &DatabaseFilterDraft,
) -> Option<DatabaseSimpleFilter> {
    let condition = database_filter_condition_from_draft(draft)?;
    let property_id = NotionDatabasePropertyId::try_from(draft.property.property_id.clone())
        .expect("loaded Notion database property IDs are non-empty");
    let property_filter = DatabasePropertyFilter::new(property_id, condition)
        .expect("validated filter draft must construct a property filter");
    Some(DatabaseSimpleFilter::new(
        draft.filter_id.clone(),
        property_filter,
    ))
}

fn database_filter_condition_from_draft(
    draft: &DatabaseFilterDraft,
) -> Option<DatabasePropertyFilterCondition> {
    let condition = match draft.operator {
        operator @ (DatabaseTextFilterOperator::TextIs
        | DatabaseTextFilterOperator::TextIsNot
        | DatabaseTextFilterOperator::TextContains
        | DatabaseTextFilterOperator::TextDoesNotContain
        | DatabaseTextFilterOperator::TextStartsWith
        | DatabaseTextFilterOperator::TextEndsWith) => text_filter_condition(draft, operator)?,
        operator @ (DatabaseTextFilterOperator::NumberEquals
        | DatabaseTextFilterOperator::NumberDoesNotEqual
        | DatabaseTextFilterOperator::NumberGreaterThan
        | DatabaseTextFilterOperator::NumberGreaterThanOrEqualTo
        | DatabaseTextFilterOperator::NumberLessThan
        | DatabaseTextFilterOperator::NumberLessThanOrEqualTo) => {
            number_filter_condition(draft, operator)?
        }
        DatabaseTextFilterOperator::CheckboxIs => {
            DatabasePropertyFilterCondition::CheckboxIs(draft.checkbox_value?)
        }
        DatabaseTextFilterOperator::CheckboxIsNot => {
            DatabasePropertyFilterCondition::CheckboxIsNot(draft.checkbox_value?)
        }
        operator @ (DatabaseTextFilterOperator::Contains
        | DatabaseTextFilterOperator::DoesNotContain) => {
            containment_filter_condition(draft, operator)?
        }
        operator @ (DatabaseTextFilterOperator::DateIs
        | DatabaseTextFilterOperator::DateIsBefore
        | DatabaseTextFilterOperator::DateIsAfter
        | DatabaseTextFilterOperator::DateIsOnOrBefore
        | DatabaseTextFilterOperator::DateIsOnOrAfter
        | DatabaseTextFilterOperator::DateIsBetween
        | DatabaseTextFilterOperator::DateIsRelativeToToday) => {
            date_filter_condition(draft, operator)
        }
        operator @ (DatabaseTextFilterOperator::SelectIs
        | DatabaseTextFilterOperator::SelectIsNot
        | DatabaseTextFilterOperator::SelectContains
        | DatabaseTextFilterOperator::SelectDoesNotContain
        | DatabaseTextFilterOperator::StatusIs
        | DatabaseTextFilterOperator::StatusIsNot) => selection_filter_condition(draft, operator)?,
        DatabaseTextFilterOperator::IsEmpty => empty_filter_condition(draft, false),
        DatabaseTextFilterOperator::IsNotEmpty => empty_filter_condition(draft, true),
    };
    Some(condition)
}

fn text_filter_condition(
    draft: &DatabaseFilterDraft,
    operator: DatabaseTextFilterOperator,
) -> Option<DatabasePropertyFilterCondition> {
    let value = || DatabaseTextFilterValue::try_from(draft.value.clone()).ok();
    let filter = match operator {
        DatabaseTextFilterOperator::TextIs => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::Is(value()?))
        }
        DatabaseTextFilterOperator::TextIsNot => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::IsNot(value()?))
        }
        DatabaseTextFilterOperator::TextContains => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::Contains(value()?))
        }
        DatabaseTextFilterOperator::TextDoesNotContain => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::DoesNotContain(value()?))
        }
        DatabaseTextFilterOperator::TextStartsWith => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::StartsWith(value()?))
        }
        DatabaseTextFilterOperator::TextEndsWith => {
            DatabasePropertyFilterCondition::Text(DatabaseTextFilter::EndsWith(value()?))
        }
        _ => return None,
    };
    Some(filter)
}

fn number_filter_condition(
    draft: &DatabaseFilterDraft,
    operator: DatabaseTextFilterOperator,
) -> Option<DatabasePropertyFilterCondition> {
    let value = database_number_filter_value(draft)?;
    let condition = match operator {
        DatabaseTextFilterOperator::NumberEquals => {
            DatabasePropertyFilterCondition::NumberEquals(value)
        }
        DatabaseTextFilterOperator::NumberDoesNotEqual => {
            DatabasePropertyFilterCondition::NumberDoesNotEqual(value)
        }
        DatabaseTextFilterOperator::NumberGreaterThan => {
            DatabasePropertyFilterCondition::NumberGreaterThan(value)
        }
        DatabaseTextFilterOperator::NumberGreaterThanOrEqualTo => {
            DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(value)
        }
        DatabaseTextFilterOperator::NumberLessThan => {
            DatabasePropertyFilterCondition::NumberLessThan(value)
        }
        DatabaseTextFilterOperator::NumberLessThanOrEqualTo => {
            DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(value)
        }
        _ => return None,
    };
    Some(condition)
}

fn containment_filter_condition(
    draft: &DatabaseFilterDraft,
    operator: DatabaseTextFilterOperator,
) -> Option<DatabasePropertyFilterCondition> {
    let condition = match operator {
        DatabaseTextFilterOperator::Contains => match draft.property.filter_type() {
            "person" => DatabasePropertyFilterCondition::PersonContains(
                database_person_filter_from_draft(draft)?,
            ),
            "relation" => DatabasePropertyFilterCondition::RelationContains(
                database_relation_filter_values_from_draft(draft)?,
            ),
            _ => return None,
        },
        DatabaseTextFilterOperator::DoesNotContain => match draft.property.filter_type() {
            "person" => DatabasePropertyFilterCondition::PersonDoesNotContain(
                database_person_filter_from_draft(draft)?,
            ),
            "relation" => DatabasePropertyFilterCondition::RelationDoesNotContain(
                database_relation_filter_values_from_draft(draft)?,
            ),
            _ => return None,
        },
        _ => return None,
    };
    Some(condition)
}

fn date_filter_condition(
    draft: &DatabaseFilterDraft,
    operator: DatabaseTextFilterOperator,
) -> DatabasePropertyFilterCondition {
    match operator {
        DatabaseTextFilterOperator::DateIs => DatabasePropertyFilterCondition::DateIs(
            DatabaseDateFilter::new(draft.date_point.clone(), draft.date_mode),
        ),
        DatabaseTextFilterOperator::DateIsBefore => DatabasePropertyFilterCondition::DateIsBefore(
            DatabaseDateFilter::new(draft.date_point.clone(), draft.date_mode),
        ),
        DatabaseTextFilterOperator::DateIsAfter => DatabasePropertyFilterCondition::DateIsAfter(
            DatabaseDateFilter::new(draft.date_point.clone(), draft.date_mode),
        ),
        DatabaseTextFilterOperator::DateIsOnOrBefore => {
            DatabasePropertyFilterCondition::DateIsOnOrBefore(DatabaseDateFilter::new(
                draft.date_point.clone(),
                draft.date_mode,
            ))
        }
        DatabaseTextFilterOperator::DateIsOnOrAfter => {
            DatabasePropertyFilterCondition::DateIsOnOrAfter(DatabaseDateFilter::new(
                draft.date_point.clone(),
                draft.date_mode,
            ))
        }
        DatabaseTextFilterOperator::DateIsBetween => DatabasePropertyFilterCondition::DateIsWithin(
            DatabaseDateFilter::new(draft.date_range.clone(), draft.date_mode),
        ),
        DatabaseTextFilterOperator::DateIsRelativeToToday => {
            DatabasePropertyFilterCondition::DateIsRelativeTo(DatabaseDateFilter::new(
                draft.date_range.clone(),
                draft.date_mode,
            ))
        }
        _ => unreachable!("date condition helper receives only date operators"),
    }
}

fn selection_filter_condition(
    draft: &DatabaseFilterDraft,
    operator: DatabaseTextFilterOperator,
) -> Option<DatabasePropertyFilterCondition> {
    let selected_values =
        || NonEmptyDatabaseFilterValues::try_from(draft.selected_values.clone()).ok();
    let condition = match operator {
        DatabaseTextFilterOperator::SelectIs => {
            DatabasePropertyFilterCondition::SelectIs(selected_values()?)
        }
        DatabaseTextFilterOperator::SelectIsNot => {
            DatabasePropertyFilterCondition::SelectIsNot(selected_values()?)
        }
        DatabaseTextFilterOperator::SelectContains => {
            DatabasePropertyFilterCondition::SelectContains(selected_values()?)
        }
        DatabaseTextFilterOperator::SelectDoesNotContain => {
            DatabasePropertyFilterCondition::SelectDoesNotContain(selected_values()?)
        }
        DatabaseTextFilterOperator::StatusIs => DatabasePropertyFilterCondition::StatusIs(
            database_status_filter_values_from_draft(draft)?,
        ),
        DatabaseTextFilterOperator::StatusIsNot => DatabasePropertyFilterCondition::StatusIsNot(
            database_status_filter_values_from_draft(draft)?,
        ),
        _ => return None,
    };
    Some(condition)
}

fn empty_filter_condition(
    draft: &DatabaseFilterDraft,
    not_empty: bool,
) -> DatabasePropertyFilterCondition {
    if not_empty {
        match draft.property.filter_type() {
            "number" => DatabasePropertyFilterCondition::NumberIsNotEmpty,
            "person" => DatabasePropertyFilterCondition::PersonIsNotEmpty,
            "relation" => DatabasePropertyFilterCondition::RelationIsNotEmpty,
            "date" => DatabasePropertyFilterCondition::DateIsNotEmpty,
            "select" | "multi_select" => DatabasePropertyFilterCondition::SelectIsNotEmpty,
            _ => DatabasePropertyFilterCondition::Text(DatabaseTextFilter::IsNotEmpty),
        }
    } else {
        match draft.property.filter_type() {
            "number" => DatabasePropertyFilterCondition::NumberIsEmpty,
            "person" => DatabasePropertyFilterCondition::PersonIsEmpty,
            "relation" => DatabasePropertyFilterCondition::RelationIsEmpty,
            "date" => DatabasePropertyFilterCondition::DateIsEmpty,
            "select" | "multi_select" => DatabasePropertyFilterCondition::SelectIsEmpty,
            _ => DatabasePropertyFilterCondition::Text(DatabaseTextFilter::IsEmpty),
        }
    }
}

pub(super) fn database_number_filter_value(
    draft: &DatabaseFilterDraft,
) -> Option<serde_json::Number> {
    draft.value.trim().parse().ok()
}

pub(super) fn database_checkbox_filter_value(value: &str) -> Option<bool> {
    match value {
        CHECKBOX_FILTER_CHECKED_VALUE => Some(true),
        CHECKBOX_FILTER_UNCHECKED_VALUE => Some(false),
        _ => None,
    }
}

pub(super) fn database_person_filter_from_draft(
    draft: &DatabaseFilterDraft,
) -> Option<DatabasePersonFilter> {
    let includes_current_user = draft
        .selected_values
        .iter()
        .any(|value| value == CURRENT_USER_FILTER_VALUE);
    let user_ids = draft
        .selected_values
        .iter()
        .filter(|value| value.as_str() != CURRENT_USER_FILTER_VALUE)
        .cloned()
        .map(NotionFilterUserId::try_from)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    match (includes_current_user, user_ids.is_empty()) {
        (true, true) => Some(DatabasePersonFilter::CurrentUser),
        (false, false) => Some(DatabasePersonFilter::Users(
            NonEmptyDatabaseFilterValues::try_from(user_ids).ok()?,
        )),
        (true, false) => Some(DatabasePersonFilter::CurrentUserAndUsers(
            NonEmptyDatabaseFilterValues::try_from(user_ids).ok()?,
        )),
        (false, true) => None,
    }
}

pub(super) fn database_relation_filter_values_from_draft(
    draft: &DatabaseFilterDraft,
) -> Option<NonEmptyDatabaseFilterValues<NotionFilterPageId>> {
    let page_ids = draft
        .selected_values
        .iter()
        .cloned()
        .map(NotionFilterPageId::try_from)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    NonEmptyDatabaseFilterValues::try_from(page_ids).ok()
}

pub(super) fn database_status_filter_values_from_draft(
    draft: &DatabaseFilterDraft,
) -> Option<NonEmptyDatabaseFilterValues<DatabaseStatusFilterValue>> {
    let values = draft
        .selected_values
        .iter()
        .map(|value| {
            value
                .strip_prefix(STATUS_OPTION_FILTER_VALUE_PREFIX)
                .map(|value| DatabaseStatusFilterValue::Option(value.to_string()))
                .or_else(|| {
                    value
                        .strip_prefix(STATUS_GROUP_FILTER_VALUE_PREFIX)
                        .map(|value| DatabaseStatusFilterValue::Group(value.to_string()))
                })
        })
        .collect::<Option<Vec<_>>>()?;
    NonEmptyDatabaseFilterValues::try_from(values).ok()
}
