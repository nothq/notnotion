use crate::model::{DatabaseProperty, DatabasePropertyOption, DatabaseStatusGroup};
use serde_json::{Map, Value};

use super::super::super::{
    property::property_schema_label,
    record::{required_array, required_string},
};

pub(crate) fn board_database_properties(
    collection_schema: Option<&Map<String, Value>>,
    blocks: &Map<String, Value>,
    users: Option<&Map<String, Value>>,
) -> Result<Vec<DatabaseProperty>, String> {
    let collection_schema =
        collection_schema.ok_or_else(|| "missing Notion collection property schema".to_string())?;
    let mut properties = collection_schema
        .iter()
        .map(|(property_id, property)| {
            let label = property_schema_label(property, blocks, users)?
                .ok_or_else(|| format!("missing Notion label for property {property_id}"))?;
            let property_type = property
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("missing Notion type for property {property_id}"))?;
            Ok(DatabaseProperty {
                property_id: property_id.clone(),
                label,
                property_type: property_type.to_string(),
                filter_type: database_property_filter_type(property, property_type),
                options: database_property_options(property, property_type)?,
                status_groups: database_status_groups(property, property_type)?,
                relation_collection_id: database_relation_collection_id(
                    property,
                    property_type,
                    property_id,
                )?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    properties.sort_by(|left, right| {
        let left_is_title = left.property_type == "title";
        let right_is_title = right.property_type == "title";
        right_is_title
            .cmp(&left_is_title)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    Ok(properties)
}

pub(crate) fn database_property_filter_type(
    property: &Value,
    property_type: &str,
) -> Option<String> {
    let filter_type = match property_type {
        "created_time" | "last_edited_time" | "last_visited_time" => "date",
        "created_by" | "last_edited_by" => "person",
        "auto_increment_id" => "number",
        "formula" => formula_filter_type(property)?,
        "rollup" => rollup_filter_type(property)?,
        _ => return None,
    };
    Some(filter_type.to_string())
}

fn formula_filter_type(property: &Value) -> Option<&'static str> {
    let result_type = property
        .get("formula2")
        .and_then(|formula| formula.get("result_type"))
        .and_then(|result| result.get("type"))
        .and_then(Value::as_str)
        .or_else(|| {
            property
                .get("formula")
                .and_then(|formula| formula.get("result_type"))
                .and_then(Value::as_str)
        })?;
    normalized_computed_filter_type(result_type)
}

fn rollup_filter_type(property: &Value) -> Option<&'static str> {
    let aggregation = property
        .get("aggregation")
        .and_then(|aggregation| match aggregation {
            Value::String(operator) => Some(operator.as_str()),
            Value::Object(aggregation) => aggregation.get("operator").and_then(Value::as_str),
            _ => None,
        });
    match aggregation {
        Some(
            "count" | "count_values" | "unique" | "empty" | "not_empty" | "checked" | "unchecked"
            | "percent_empty" | "percent_not_empty" | "percent_checked" | "percent_unchecked"
            | "percent_per_group" | "sum" | "average" | "median" | "range",
        ) => return Some("number"),
        Some("count_per_group") => return Some("text"),
        Some("earliest_date" | "latest_date" | "date_range") => return Some("date"),
        Some("min" | "max") => {}
        Some("show_original" | "show_unique" | "show_unique_values") | None => return None,
        Some(_) => return None,
    }
    property
        .get("target_property_type")
        .and_then(Value::as_str)
        .and_then(normalized_computed_filter_type)
}

fn normalized_computed_filter_type(property_type: &str) -> Option<&'static str> {
    match property_type {
        "number" | "auto_increment_id" => Some("number"),
        "date" | "created_time" | "last_edited_time" | "last_visited_time" => Some("date"),
        "checkbox" => Some("checkbox"),
        "person" | "created_by" | "last_edited_by" => Some("person"),
        "title" | "text" | "url" | "email" | "phone_number" | "block" => Some("text"),
        _ => None,
    }
}

pub(crate) fn database_property_options(
    property: &Value,
    property_type: &str,
) -> Result<Vec<DatabasePropertyOption>, String> {
    if !matches!(property_type, "select" | "multi_select" | "status") {
        return Ok(Vec::new());
    }
    let options = match property.get("options") {
        Some(_) => required_array(property, "options")?.as_slice(),
        None => &[],
    };
    options
        .iter()
        .map(|option| {
            Ok(DatabasePropertyOption {
                id: required_string(option, "id")?.to_string(),
                value: required_string(option, "value")?.to_string(),
                color: match option.get("color") {
                    None => "default".to_string(),
                    Some(Value::String(color)) => color.clone(),
                    Some(_) => {
                        return Err("Notion database option color must be a string".to_string())
                    }
                },
            })
        })
        .collect()
}

fn database_status_groups(
    property: &Value,
    property_type: &str,
) -> Result<Vec<DatabaseStatusGroup>, String> {
    if property_type != "status" {
        return Ok(Vec::new());
    }
    let groups = match property.get("groups") {
        Some(_) => required_array(property, "groups")?.as_slice(),
        None => &[],
    };
    groups
        .iter()
        .map(|group| {
            let option_ids = match group.get("optionIds") {
                Some(_) => required_array(group, "optionIds")?.as_slice(),
                None => &[],
            };
            Ok(DatabaseStatusGroup {
                id: required_string(group, "id")?.to_string(),
                name: required_string(group, "name")?.to_string(),
                color: required_string(group, "color")?.to_string(),
                option_ids: option_ids
                    .iter()
                    .map(|option_id| {
                        option_id.as_str().map(str::to_string).ok_or_else(|| {
                            "Notion status group option ID must be a string".to_string()
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect()
}

fn database_relation_collection_id(
    property: &Value,
    property_type: &str,
    property_id: &str,
) -> Result<Option<String>, String> {
    if property_type != "relation" {
        return Ok(None);
    }
    let collection_id = property
        .get("collection_pointer")
        .and_then(|pointer| pointer.get("id"))
        .and_then(Value::as_str)
        .or_else(|| property.get("collection_id").and_then(Value::as_str))
        .ok_or_else(|| {
            format!("missing Notion relation collection pointer for property {property_id}")
        })?;
    if collection_id.trim().is_empty() {
        return Err(format!(
            "Notion relation collection pointer for property {property_id} is empty"
        ));
    }
    Ok(Some(collection_id.to_string()))
}
