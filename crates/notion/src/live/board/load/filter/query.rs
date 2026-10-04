use serde_json::{Map, Value};

use crate::model::{
    DatabaseAdvancedFilterState, DatabaseSimpleFilterState, DatabaseSimpleFiltersState,
    DatabaseViewFilterState,
};

use super::{filter_group_value, property_filter_value};

pub(super) fn compiled_filter_owner<'a>(
    request: &'a mut Value,
    reducer_name: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    let loader = request
        .get_mut("loader")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing loader in compiled Notion database query".to_string())?;
    loader
        .get("reducers")
        .and_then(Value::as_object)
        .and_then(|reducers| reducers.get(reducer_name))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!("missing loader.reducers.{reducer_name} in compiled Notion database query")
        })?;
    if loader.contains_key("filter") {
        return Ok(loader);
    }
    Ok(loader
        .get_mut("reducers")
        .and_then(Value::as_object_mut)
        .and_then(|reducers| reducers.get_mut(reducer_name))
        .and_then(Value::as_object_mut)
        .expect("compiled query reducer was validated above"))
}

pub(super) fn validate_preserved_unsupported_filters(
    base: &DatabaseViewFilterState,
    desired: &DatabaseViewFilterState,
) -> Result<(), String> {
    let base_simple = match base.simple() {
        DatabaseSimpleFiltersState::Entries(filters) => filters,
        DatabaseSimpleFiltersState::Unsupported => {
            return Err(
                "unsupported Notion simple filter state cannot be queried safely".to_string(),
            );
        }
    };
    let desired_simple = match desired.simple() {
        DatabaseSimpleFiltersState::Entries(filters) => filters,
        DatabaseSimpleFiltersState::Unsupported => {
            return Err(
                "unsupported Notion simple filter state cannot be queried safely".to_string(),
            );
        }
    };
    if unsupported_filter_ids(base_simple) != unsupported_filter_ids(desired_simple) {
        return Err(
            "Notion database filter query cannot add, remove, or reorder unsupported filters"
                .to_string(),
        );
    }
    match (base.advanced(), desired.advanced()) {
        (DatabaseAdvancedFilterState::Unsupported, DatabaseAdvancedFilterState::Unsupported)
        | (DatabaseAdvancedFilterState::None, DatabaseAdvancedFilterState::None)
        | (DatabaseAdvancedFilterState::None, DatabaseAdvancedFilterState::Editable(_))
        | (DatabaseAdvancedFilterState::Editable(_), DatabaseAdvancedFilterState::None)
        | (DatabaseAdvancedFilterState::Editable(_), DatabaseAdvancedFilterState::Editable(_)) => {
            Ok(())
        }
        _ => Err(
            "Notion database filter query cannot replace an unsupported advanced filter"
                .to_string(),
        ),
    }
}

fn unsupported_filter_ids(filters: &[DatabaseSimpleFilterState]) -> Vec<&str> {
    filters
        .iter()
        .filter_map(|filter| match filter {
            DatabaseSimpleFilterState::Editable(_) => None,
            DatabaseSimpleFilterState::Unsupported(filter_id) => Some(filter_id.as_str()),
        })
        .collect()
}

pub(super) fn editable_filter_values(
    state: &DatabaseViewFilterState,
) -> Result<Vec<Value>, String> {
    let filters = match state.simple() {
        DatabaseSimpleFiltersState::Entries(filters) => filters,
        DatabaseSimpleFiltersState::Unsupported => {
            return Err(
                "unsupported Notion simple filter state cannot be queried safely".to_string(),
            );
        }
    };
    let mut values = filters
        .iter()
        .filter_map(|filter| match filter {
            DatabaseSimpleFilterState::Editable(filter) => {
                Some(property_filter_value(filter.filter()))
            }
            DatabaseSimpleFilterState::Unsupported(_) => None,
        })
        .collect::<Vec<_>>();
    if let DatabaseAdvancedFilterState::Editable(filter) = state.advanced() {
        values.push(filter_group_value(filter));
    }
    Ok(values)
}

pub(super) fn strip_filter_values(filter: Value, values: &mut Vec<Value>) -> Option<Value> {
    if let Some(index) = values.iter().position(|value| value == &filter) {
        values.swap_remove(index);
        return None;
    }
    let Value::Object(mut object) = filter else {
        return Some(filter);
    };
    let filters = match object.remove("filters") {
        Some(Value::Array(filters)) => filters,
        Some(filters) => {
            object.insert("filters".to_string(), filters);
            return Some(Value::Object(object));
        }
        None => return Some(Value::Object(object)),
    };
    let filters = filters
        .into_iter()
        .filter_map(|filter| strip_filter_values(filter, values))
        .collect::<Vec<_>>();
    if filters.is_empty() {
        None
    } else {
        object.insert("filters".to_string(), Value::Array(filters));
        Some(Value::Object(object))
    }
}
