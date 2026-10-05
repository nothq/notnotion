use super::{
    collection_entry, database_property_options, default_empty_property_value,
    normalize_property_value, page_collection_id, plain_text_from_property_value,
    plain_text_from_property_value_with_page_titles, property_schema_label, CardPageProperty,
    HashMap, Map, PropertyLookup, Value,
};

pub(super) fn page_collection<'a>(
    root: &Value,
    collections: Option<&'a Map<String, Value>>,
) -> Option<&'a Value> {
    page_collection_id(root).and_then(|collection_id| {
        collections.and_then(|collections| collection_entry(collections, collection_id).ok())
    })
}

pub(super) fn collection_schema(collection: Option<&Value>) -> Option<&Map<String, Value>> {
    collection
        .and_then(|collection| collection.get("schema"))
        .and_then(Value::as_object)
}

pub(super) fn page_status(
    root_properties: Option<&Map<String, Value>>,
    collection_schema: Option<&Map<String, Value>>,
    blocks: &Map<String, Value>,
    users: Option<&Map<String, Value>>,
) -> Result<Option<String>, String> {
    let status_property_id = collection_schema.and_then(page_status_property_id);
    let Some(value) = status_property_id
        .and_then(|property_id| root_properties.and_then(|properties| properties.get(property_id)))
    else {
        return Ok(None);
    };
    let value = plain_text_from_property_value(value, Some(blocks), users)?;
    Ok((!value.trim().is_empty()).then_some(value))
}

fn page_status_property_id(collection_schema: &Map<String, Value>) -> Option<&str> {
    collection_schema
        .iter()
        .find_map(|(property_id, property)| {
            (property.get("type").and_then(Value::as_str) == Some("status"))
                .then_some(property_id.as_str())
        })
}

pub(super) fn page_property_ids(
    collection: Option<&Value>,
    root_properties: Option<&Map<String, Value>>,
) -> Vec<String> {
    let mut property_ids = collection
        .and_then(|collection| collection.get("format"))
        .and_then(|format| format.get("collection_page_properties"))
        .and_then(Value::as_array)
        .map(|properties| {
            properties
                .iter()
                .filter_map(|property| property.get("property").and_then(Value::as_str))
                .filter(|property_id| *property_id != "title")
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(root_properties) = root_properties {
        for property_id in root_properties.keys() {
            if property_id != "title" && !property_ids.iter().any(|entry| entry == property_id) {
                property_ids.push(property_id.clone());
            }
        }
    }
    property_ids
}

pub(super) fn page_properties(
    property_ids: Vec<String>,
    root_properties: Option<&Map<String, Value>>,
    collection_schema: Option<&Map<String, Value>>,
    property_lookup: PropertyLookup<'_>,
) -> Result<Vec<CardPageProperty>, String> {
    let mut page_title_cache = HashMap::new();
    let mut properties = Vec::new();
    for property_id in property_ids {
        if let Some(property) = page_property(
            &property_id,
            root_properties,
            collection_schema,
            property_lookup,
            &mut page_title_cache,
        )? {
            properties.push(property);
        }
    }
    Ok(properties)
}

fn page_property(
    property_id: &str,
    root_properties: Option<&Map<String, Value>>,
    collection_schema: Option<&Map<String, Value>>,
    property_lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<CardPageProperty>, String> {
    let schema_property = collection_schema.and_then(|schema| schema.get(property_id));
    let property_type = schema_property
        .and_then(|property| property.get("type").and_then(Value::as_str))
        .unwrap_or("unknown");
    let label = match (property_lookup.blocks, schema_property) {
        (Some(blocks), Some(property)) => {
            property_schema_label(property, blocks, property_lookup.users)?
                .unwrap_or_else(|| property_id.to_string())
        }
        _ => property_id.to_string(),
    };
    let value = page_property_value(
        property_id,
        root_properties,
        property_type,
        property_lookup,
        page_title_cache,
    )?;
    Ok(Some(CardPageProperty {
        property_id: property_id.to_string(),
        label,
        property_type: property_type.to_string(),
        value,
        status_options: schema_property
            .map(|property| database_property_options(property, property_type))
            .transpose()?
            .unwrap_or_default(),
    }))
}

fn page_property_value(
    property_id: &str,
    root_properties: Option<&Map<String, Value>>,
    property_type: &str,
    property_lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<String, String> {
    let Some(value) = root_properties.and_then(|properties| properties.get(property_id)) else {
        return Ok(default_empty_property_value(Some(property_type)));
    };
    let value = normalize_property_value(
        Some(property_type),
        plain_text_from_property_value_with_page_titles(value, property_lookup, page_title_cache)?,
    );
    Ok(if value.trim().is_empty() {
        default_empty_property_value(Some(property_type))
    } else {
        value
    })
}
