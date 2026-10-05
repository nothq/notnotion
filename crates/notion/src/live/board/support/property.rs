use super::{BoardDateValue, HashMap, HashSet, Map, Value};

mod reference;

use reference::{append_property_chunk_text, PageReferenceResolution};

#[derive(Clone, Copy)]
pub(crate) struct PropertyLookup<'a> {
    pub(crate) blocks: Option<&'a Map<String, Value>>,
    pub(crate) users: Option<&'a Map<String, Value>>,
    collections: Option<&'a Map<String, Value>>,
    collection_views: Option<&'a Map<String, Value>>,
    unavailable_reference_block_ids: Option<&'a HashSet<String>>,
    page_reference_resolution: PageReferenceResolution,
    allow_partial_preview: bool,
}

/// The hydrated record tables of a completed page response.
pub(crate) struct CompletedPageRecords<'a> {
    pub(crate) blocks: &'a Map<String, Value>,
    pub(crate) users: Option<&'a Map<String, Value>>,
    pub(crate) collections: Option<&'a Map<String, Value>>,
    pub(crate) collection_views: Option<&'a Map<String, Value>>,
    pub(crate) unavailable_reference_block_ids: &'a HashSet<String>,
}

impl<'a> PropertyLookup<'a> {
    pub(crate) const fn preloaded(
        blocks: Option<&'a Map<String, Value>>,
        users: Option<&'a Map<String, Value>>,
    ) -> Self {
        Self {
            blocks,
            users,
            collections: None,
            collection_views: None,
            unavailable_reference_block_ids: None,
            page_reference_resolution: PageReferenceResolution::Preloaded,
            allow_partial_preview: false,
        }
    }

    pub(crate) const fn completed_page(
        records: CompletedPageRecords<'a>,
        allow_partial_preview: bool,
    ) -> Self {
        let CompletedPageRecords {
            blocks,
            users,
            collections,
            collection_views,
            unavailable_reference_block_ids,
        } = records;
        Self {
            blocks: Some(blocks),
            users,
            collections,
            collection_views,
            unavailable_reference_block_ids: Some(unavailable_reference_block_ids),
            page_reference_resolution: PageReferenceResolution::CompletedPage,
            allow_partial_preview,
        }
    }
}

pub(crate) fn normalize_property_value(property_type: Option<&str>, value: String) -> String {
    match property_type {
        Some("relation") => {
            let mut seen = HashSet::new();
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .filter(|entry| seen.insert((*entry).to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        }
        _ => value,
    }
}

pub(crate) fn extract_board_date_value(value: &Value) -> Option<BoardDateValue> {
    let chunks = value.as_array()?;
    for chunk in chunks {
        let parts = chunk.as_array()?;
        let token = parts.first().and_then(Value::as_str)?;
        if token != "‣" {
            continue;
        }

        let reference = parts
            .get(1)
            .and_then(Value::as_array)
            .and_then(|refs| refs.first())
            .and_then(Value::as_array)?;
        if reference.first().and_then(Value::as_str) != Some("d") {
            continue;
        }

        let date_value = reference.get(1)?;
        let start_date = date_value.get("start_date").and_then(Value::as_str)?;
        return Some(BoardDateValue {
            start_date: start_date.to_string(),
            end_date: date_value
                .get("end_date")
                .and_then(Value::as_str)
                .map(str::to_string),
            start_time: date_value
                .get("start_time")
                .and_then(Value::as_str)
                .map(str::to_string),
            end_time: date_value
                .get("end_time")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }

    None
}

pub(crate) fn property_schema_label(
    property: &Value,
    blocks: &Map<String, Value>,
    users: Option<&Map<String, Value>>,
) -> Result<Option<String>, String> {
    let Some(name) = property.get("name") else {
        return Ok(None);
    };
    let label = if let Some(label) = name.as_str() {
        label.to_string()
    } else {
        plain_text_from_property_value(name, Some(blocks), users)?
    };
    Ok((!label.trim().is_empty()).then_some(label))
}

pub(crate) fn plain_text_from_property_value(
    value: &Value,
    blocks: Option<&Map<String, Value>>,
    users: Option<&Map<String, Value>>,
) -> Result<String, String> {
    let mut page_title_cache = HashMap::new();
    plain_text_from_property_value_with_page_titles(
        value,
        PropertyLookup::preloaded(blocks, users),
        &mut page_title_cache,
    )
}

pub(crate) fn plain_text_from_property_value_with_page_titles(
    value: &Value,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<String, String> {
    let mut text = String::new();
    let Some(chunks) = value.as_array() else {
        return Ok(text);
    };
    for chunk in chunks {
        append_property_chunk_text(&mut text, chunk, lookup, page_title_cache)?;
    }
    Ok(text)
}

pub(crate) fn default_empty_property_value(property_type: Option<&str>) -> String {
    match property_type {
        Some("person" | "relation" | "date" | "text") => "Empty".to_string(),
        _ => String::new(),
    }
}
