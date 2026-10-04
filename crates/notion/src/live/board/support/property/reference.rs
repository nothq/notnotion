use std::collections::HashMap;

use serde_json::{Map, Value};

use super::super::format_date_property;
use super::super::record::{
    loaded_record_value, record_is_deleted, record_value_state, required_string,
    title_property_allow_empty, RecordValueState,
};
use super::super::view::collection_view_collection_id;
use super::{plain_text_from_property_value_with_page_titles, PropertyLookup};

#[derive(Clone, Copy)]
pub(super) enum PageReferenceResolution {
    Preloaded,
    CompletedPage,
}

enum CompletedPageReference<'a> {
    Present(&'a Value),
    Unavailable,
}

enum PageReferencePresentation {
    Resolved(String),
    Untitled,
    UnresolvedId(String),
}

impl PageReferencePresentation {
    fn into_text(self) -> String {
        match self {
            Self::Resolved(title) | Self::UnresolvedId(title) => title,
            Self::Untitled => "Untitled".to_string(),
        }
    }
}

pub(super) fn append_property_chunk_text(
    text: &mut String,
    chunk: &Value,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<(), String> {
    let Some(parts) = chunk.as_array() else {
        return Ok(());
    };
    let Some(token) = parts.first().and_then(Value::as_str) else {
        return Ok(());
    };
    if token != "‣" {
        text.push_str(token);
        return Ok(());
    }
    if let Some(reference_text) = referenced_property_text(parts, lookup, page_title_cache)? {
        text.push_str(&reference_text);
    }
    Ok(())
}

fn referenced_property_text(
    parts: &[Value],
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<String>, String> {
    let Some(reference) = parts
        .get(1)
        .and_then(Value::as_array)
        .and_then(|refs| refs.first())
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    match reference.first().and_then(Value::as_str) {
        Some("p") => match reference.get(1).and_then(Value::as_str) {
            Some(block_id) => page_reference_title(block_id, lookup, page_title_cache),
            None => Ok(None),
        },
        Some("u") => Ok(reference
            .get(1)
            .and_then(Value::as_str)
            .map(|user_id| user_reference_name(user_id, lookup.users))),
        Some("d") => Ok(reference.get(1).map(format_date_property)),
        _ => Ok(None),
    }
}

fn page_reference_title(
    block_id: &str,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<String>, String> {
    if let Some(title) = page_title_cache.get(block_id) {
        return Ok(Some(title.clone()));
    }
    let title = match lookup.page_reference_resolution {
        PageReferenceResolution::Preloaded => {
            preloaded_page_reference(block_id, lookup.blocks)?.into_text()
        }
        PageReferenceResolution::CompletedPage => {
            return completed_page_reference_title(block_id, lookup, page_title_cache);
        }
    };
    page_title_cache.insert(block_id.to_string(), title.clone());
    Ok(Some(title))
}

fn preloaded_page_reference(
    block_id: &str,
    blocks: Option<&Map<String, Value>>,
) -> Result<PageReferencePresentation, String> {
    let block = match record_value_state(blocks.and_then(|blocks| blocks.get(block_id))) {
        RecordValueState::Incomplete => {
            return Ok(PageReferencePresentation::UnresolvedId(
                block_id.to_string(),
            ));
        }
        RecordValueState::Unavailable => return Ok(PageReferencePresentation::Untitled),
        RecordValueState::Present(block) if record_is_deleted(block) => {
            return Ok(PageReferencePresentation::Untitled);
        }
        RecordValueState::Present(block) => block,
    };
    let title = title_property_allow_empty(block)?.unwrap_or_default();
    Ok(if title.trim().is_empty() {
        PageReferencePresentation::Untitled
    } else {
        PageReferencePresentation::Resolved(title)
    })
}

fn completed_page_reference_title(
    block_id: &str,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<String>, String> {
    let block = match completed_page_reference(block_id, lookup)? {
        CompletedPageReference::Unavailable => {
            let title = "Untitled".to_string();
            page_title_cache.insert(block_id.to_string(), title.clone());
            return Ok(Some(title));
        }
        CompletedPageReference::Present(block) => block,
    };
    page_title_cache.insert(block_id.to_string(), "Untitled".to_string());
    let block_type = required_string(block, "type")
        .map_err(|error| format!("Notion page reference {block_id}: {error}"))?;
    let title = match block_type {
        "collection_view" | "collection_view_page" => {
            let collection_id = collection_view_collection_id(block, lookup.collection_views, None)
                .map_err(|error| format!("Notion page reference {block_id}: {error}"))?;
            let Some(collections) = lookup.collections else {
                if lookup.allow_partial_preview {
                    return Ok(Some("Untitled".to_string()));
                }
                return Err(format!(
                    "missing hydrated collection table for page reference {block_id}"
                ));
            };
            completed_collection_reference_name(
                collection_id,
                collections,
                lookup,
                page_title_cache,
            )?
        }
        _ => completed_block_reference_title(block, lookup, page_title_cache)?,
    };
    page_title_cache.insert(block_id.to_string(), title.clone());
    Ok(Some(title))
}

fn completed_page_reference<'a>(
    block_id: &str,
    lookup: PropertyLookup<'a>,
) -> Result<CompletedPageReference<'a>, String> {
    if lookup
        .unavailable_reference_block_ids
        .is_some_and(|block_ids| block_ids.contains(block_id))
    {
        return Ok(CompletedPageReference::Unavailable);
    }
    match record_value_state(lookup.blocks.and_then(|blocks| blocks.get(block_id))) {
        RecordValueState::Present(block) if record_is_deleted(block) => {
            Ok(CompletedPageReference::Unavailable)
        }
        RecordValueState::Present(block) => Ok(CompletedPageReference::Present(block)),
        RecordValueState::Unavailable => Ok(CompletedPageReference::Unavailable),
        RecordValueState::Incomplete if lookup.allow_partial_preview => {
            Ok(CompletedPageReference::Unavailable)
        }
        RecordValueState::Incomplete => {
            Err(format!("missing hydrated Notion page reference {block_id}"))
        }
    }
}

fn completed_block_reference_title(
    block: &Value,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<String, String> {
    let Some(title) = block
        .get("properties")
        .and_then(|properties| properties.get("title"))
    else {
        return Ok("Untitled".to_string());
    };
    let title = plain_text_from_property_value_with_page_titles(title, lookup, page_title_cache)?;
    if title.trim().is_empty() {
        Ok("Untitled".to_string())
    } else {
        Ok(title)
    }
}

fn completed_collection_name(
    collection: &Value,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<String, String> {
    let Some(name) = collection.get("name") else {
        return Ok("New database".to_string());
    };
    let preloaded = PropertyLookup::preloaded(lookup.blocks, lookup.users);
    let name = plain_text_from_property_value_with_page_titles(name, preloaded, page_title_cache)?;
    if name.trim().is_empty() {
        Ok("New database".to_string())
    } else {
        Ok(name)
    }
}

fn completed_collection_reference_name(
    collection_id: &str,
    collections: &Map<String, Value>,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<String, String> {
    match record_value_state(collections.get(collection_id)) {
        RecordValueState::Unavailable => Ok("Untitled".to_string()),
        RecordValueState::Incomplete if lookup.allow_partial_preview => Ok("Untitled".to_string()),
        RecordValueState::Incomplete => Err(format!(
            "missing hydrated Notion collection reference {collection_id}"
        )),
        RecordValueState::Present(collection) => {
            completed_collection_name(collection, lookup, page_title_cache)
        }
    }
}

fn user_reference_name(user_id: &str, users: Option<&Map<String, Value>>) -> String {
    users
        .and_then(|users| {
            users
                .get(user_id)
                .and_then(loaded_record_value)
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
        })
        .unwrap_or(user_id)
        .to_string()
}
