use std::collections::HashSet;

use serde_json::{Map, Number, Value};

use super::{
    exact_value, parse_date_point_filter, parse_date_range_filter, parse_person_filter,
    parse_relation_filter_values, parse_select_filter_values, parse_status_filter_values,
    parse_text_filter, DatabaseAdvancedFilterState, DatabaseFilterGroup,
    DatabaseFilterGroupOperator, DatabaseFilterNode, DatabasePropertyFilter,
    DatabasePropertyFilterCondition, DatabaseSimpleFilter, DatabaseSimpleFilterState,
    DatabaseSimpleFiltersState, DatabaseViewFilterState, NotionDatabaseFilterId,
    NotionDatabasePropertyId,
};

pub(in crate::live::board::load) fn database_view_filter_state(
    view: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> DatabaseViewFilterState {
    DatabaseViewFilterState::new(
        simple_filter_state(view, collection_schema),
        advanced_filter_state(view, collection_schema),
    )
}

fn simple_filter_state(
    view: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> DatabaseSimpleFiltersState {
    let format = match view.get("format") {
        None | Some(Value::Null) => return DatabaseSimpleFiltersState::default(),
        Some(Value::Object(format)) => format,
        Some(_) => return DatabaseSimpleFiltersState::Unsupported,
    };
    let filters = match format.get("property_filters") {
        None | Some(Value::Null) => return DatabaseSimpleFiltersState::default(),
        Some(Value::Array(filters)) => filters,
        Some(_) => return DatabaseSimpleFiltersState::Unsupported,
    };
    let mut filter_ids = HashSet::new();
    let mut parsed = Vec::with_capacity(filters.len());
    for filter in filters {
        let Some(filter) = filter.as_object() else {
            return DatabaseSimpleFiltersState::Unsupported;
        };
        let Some(filter_id) = filter
            .get("id")
            .and_then(Value::as_str)
            .and_then(|filter_id| NotionDatabaseFilterId::try_from(filter_id.to_string()).ok())
        else {
            return DatabaseSimpleFiltersState::Unsupported;
        };
        if !filter_ids.insert(filter_id.clone()) {
            return DatabaseSimpleFiltersState::Unsupported;
        }
        let state = filter
            .get("filter")
            .and_then(|filter| parse_property_filter(filter, collection_schema))
            .map(|filter| {
                DatabaseSimpleFilterState::Editable(DatabaseSimpleFilter::new(
                    filter_id.clone(),
                    filter,
                ))
            })
            .unwrap_or(DatabaseSimpleFilterState::Unsupported(filter_id));
        parsed.push(state);
    }
    DatabaseSimpleFiltersState::Entries(parsed)
}

fn advanced_filter_state(
    view: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> DatabaseAdvancedFilterState {
    let query = match view.get("query2") {
        None | Some(Value::Null) => return DatabaseAdvancedFilterState::None,
        Some(Value::Object(query)) => query,
        Some(_) => return DatabaseAdvancedFilterState::Unsupported,
    };
    let filter = match query.get("filter") {
        None | Some(Value::Null) => return DatabaseAdvancedFilterState::None,
        Some(filter) => filter,
    };
    parse_filter_group(filter, collection_schema)
        .map(DatabaseAdvancedFilterState::Editable)
        .unwrap_or(DatabaseAdvancedFilterState::Unsupported)
}

fn parse_filter_group(
    value: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> Option<DatabaseFilterGroup> {
    let value = value.as_object()?;
    let operator = match value.get("operator")?.as_str()? {
        "and" => DatabaseFilterGroupOperator::And,
        "or" => DatabaseFilterGroupOperator::Or,
        _ => return None,
    };
    let filters = value
        .get("filters")?
        .as_array()?
        .iter()
        .map(|filter| parse_filter_node(filter, collection_schema))
        .collect::<Option<Vec<_>>>()?;
    Some(DatabaseFilterGroup::new(operator, filters))
}

fn parse_filter_node(
    value: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> Option<DatabaseFilterNode> {
    let value_object = value.as_object()?;
    match (
        value_object.contains_key("property"),
        value_object.contains_key("filters"),
    ) {
        (true, false) => {
            parse_property_filter(value, collection_schema).map(DatabaseFilterNode::Property)
        }
        (false, true) => {
            parse_filter_group(value, collection_schema).map(DatabaseFilterNode::Group)
        }
        _ => None,
    }
}

fn parse_property_filter(
    value: &Value,
    collection_schema: Option<&Map<String, Value>>,
) -> Option<DatabasePropertyFilter> {
    let value = value.as_object()?;
    let property_id = value.get("property")?.as_str()?;
    let property_schema = collection_schema?.get(property_id)?;
    let property_type = property_schema.get("type")?.as_str()?;
    let filter_type = super::super::super::support::view::database_property_filter_type(
        property_schema,
        property_type,
    )
    .unwrap_or_else(|| property_type.to_string());
    let filter = value.get("filter")?.as_object()?;
    let operator = filter.get("operator")?.as_str()?;
    let condition = parse_property_filter_condition(&filter_type, operator, filter)?;
    DatabasePropertyFilter::new(
        NotionDatabasePropertyId::try_from(property_id.to_string()).ok()?,
        condition,
    )
    .ok()
}

fn parse_property_filter_condition(
    property_type: &str,
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match property_type {
        "title" | "text" | "url" | "email" | "phone_number" => {
            parse_text_filter(operator, filter).map(DatabasePropertyFilterCondition::Text)
        }
        "number" => parse_number_filter_condition(operator, filter),
        "checkbox" => parse_checkbox_filter_condition(operator, filter),
        "person" => parse_person_filter_condition(operator, filter),
        "relation" => parse_relation_filter_condition(operator, filter),
        "date" => parse_date_filter_condition(operator, filter),
        "select" | "multi_select" => parse_select_filter_condition(property_type, operator, filter),
        "status" => parse_status_filter_condition(operator, filter),
        _ => None,
    }
}

fn parse_number_filter_condition(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match operator {
        "number_equals" => Some(DatabasePropertyFilterCondition::NumberEquals(
            parse_number_filter_value(filter)?,
        )),
        "number_does_not_equal" => Some(DatabasePropertyFilterCondition::NumberDoesNotEqual(
            parse_number_filter_value(filter)?,
        )),
        "number_greater_than" => Some(DatabasePropertyFilterCondition::NumberGreaterThan(
            parse_number_filter_value(filter)?,
        )),
        "number_greater_than_or_equal_to" => {
            Some(DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(
                parse_number_filter_value(filter)?,
            ))
        }
        "number_less_than" => Some(DatabasePropertyFilterCondition::NumberLessThan(
            parse_number_filter_value(filter)?,
        )),
        "number_less_than_or_equal_to" => {
            Some(DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(
                parse_number_filter_value(filter)?,
            ))
        }
        "is_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::NumberIsEmpty)
        }
        "is_not_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::NumberIsNotEmpty)
        }
        _ => None,
    }
}

fn parse_number_filter_value(filter: &Map<String, Value>) -> Option<Number> {
    exact_value(filter.get("value")?)?.as_number().cloned()
}

fn parse_checkbox_filter_condition(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    let checked = exact_value(filter.get("value")?)?.as_bool()?;
    match operator {
        "checkbox_is" => Some(DatabasePropertyFilterCondition::CheckboxIs(checked)),
        "checkbox_is_not" => Some(DatabasePropertyFilterCondition::CheckboxIsNot(checked)),
        _ => None,
    }
}

fn parse_person_filter_condition(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match operator {
        "person_contains" => Some(DatabasePropertyFilterCondition::PersonContains(
            parse_person_filter(filter.get("value")?)?,
        )),
        "person_does_not_contain" => Some(DatabasePropertyFilterCondition::PersonDoesNotContain(
            parse_person_filter(filter.get("value")?)?,
        )),
        "is_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::PersonIsEmpty)
        }
        "is_not_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::PersonIsNotEmpty)
        }
        _ => None,
    }
}

fn parse_relation_filter_condition(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match operator {
        "relation_contains" => Some(DatabasePropertyFilterCondition::RelationContains(
            parse_relation_filter_values(filter.get("value")?)?,
        )),
        "relation_does_not_contain" => {
            Some(DatabasePropertyFilterCondition::RelationDoesNotContain(
                parse_relation_filter_values(filter.get("value")?)?,
            ))
        }
        "is_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::RelationIsEmpty)
        }
        "is_not_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::RelationIsNotEmpty)
        }
        _ => None,
    }
}

fn parse_date_filter_condition(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match operator {
        "date_is" => Some(DatabasePropertyFilterCondition::DateIs(
            parse_date_point_filter(filter)?,
        )),
        "date_is_before" => Some(DatabasePropertyFilterCondition::DateIsBefore(
            parse_date_point_filter(filter)?,
        )),
        "date_is_after" => Some(DatabasePropertyFilterCondition::DateIsAfter(
            parse_date_point_filter(filter)?,
        )),
        "date_is_on_or_before" => Some(DatabasePropertyFilterCondition::DateIsOnOrBefore(
            parse_date_point_filter(filter)?,
        )),
        "date_is_on_or_after" => Some(DatabasePropertyFilterCondition::DateIsOnOrAfter(
            parse_date_point_filter(filter)?,
        )),
        "date_is_within" => Some(DatabasePropertyFilterCondition::DateIsWithin(
            parse_date_range_filter(filter)?,
        )),
        "date_is_relative_to" => Some(DatabasePropertyFilterCondition::DateIsRelativeTo(
            parse_date_range_filter(filter)?,
        )),
        "is_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::DateIsEmpty)
        }
        "is_not_empty" if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::DateIsNotEmpty)
        }
        _ => None,
    }
}

fn parse_select_filter_condition(
    property_type: &str,
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match (property_type, operator) {
        ("select", "enum_is") => Some(DatabasePropertyFilterCondition::SelectIs(
            parse_select_filter_values(filter.get("value")?)?,
        )),
        ("select", "enum_is_not") => Some(DatabasePropertyFilterCondition::SelectIsNot(
            parse_select_filter_values(filter.get("value")?)?,
        )),
        ("multi_select", "enum_contains") => Some(DatabasePropertyFilterCondition::SelectContains(
            parse_select_filter_values(filter.get("value")?)?,
        )),
        ("multi_select", "enum_does_not_contain") => {
            Some(DatabasePropertyFilterCondition::SelectDoesNotContain(
                parse_select_filter_values(filter.get("value")?)?,
            ))
        }
        (_, "is_empty") if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::SelectIsEmpty)
        }
        (_, "is_not_empty") if !filter.contains_key("value") => {
            Some(DatabasePropertyFilterCondition::SelectIsNotEmpty)
        }
        _ => None,
    }
}

fn parse_status_filter_condition(
    operator: &str,
    filter: &Map<String, Value>,
) -> Option<DatabasePropertyFilterCondition> {
    match operator {
        "status_is" => Some(DatabasePropertyFilterCondition::StatusIs(
            parse_status_filter_values(filter.get("value")?)?,
        )),
        "status_is_not" => Some(DatabasePropertyFilterCondition::StatusIsNot(
            parse_status_filter_values(filter.get("value")?)?,
        )),
        _ => None,
    }
}
