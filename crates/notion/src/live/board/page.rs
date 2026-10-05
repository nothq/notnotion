use super::load::explicit_page_shell_icon;
use super::page_state::PageMutationState;
use super::{
    block_value, collection_database_name, collection_entry, database_property_options,
    default_empty_property_value, load_complete_page_response_typed, loaded_record_value,
    normalize_property_value, page_collection_id, page_property_rank,
    plain_text_from_property_value, plain_text_from_property_value_with_page_titles,
    property_schema_label, required_string, title_property_allow_empty, CardPage, CardPageBlock,
    CardPageBlockKind, CardPageProperty, CardPageStructuralBlock, CompletePageResponse,
    CompletedPageRecords, HashMap, Map, PropertyLookup, Value,
};
use crate::model::{
    CardPageBlockApiType, CardPageBlockColor, CardPageBlockLastEdited, CardPageCodeLanguage,
    CardPageCodeWrap, CardPageEditableBlock, CardPageFont, CardPageFormat, CardPageQuoteSize,
    CardPageToDoState,
};

mod blocks;
mod comments;
mod layout;
mod properties;
mod rich_text;

use blocks::page_blocks;
use comments::page_discussions_from_response;
use layout::page_layout_block;
use properties::{
    collection_schema, page_collection, page_properties, page_property_ids, page_status,
};
use rich_text::{parse_card_page_table_cell, parse_card_page_text};

pub(in crate::live) struct LoadedCardPageSnapshot {
    pub(in crate::live) page: CardPage,
    pub(in crate::live) mutation_state: PageMutationState,
    pub(in crate::live) response: CompletePageResponse,
}

struct PageBlockParseContext<'a> {
    space_id: &'a str,
    blocks: &'a Map<String, Value>,
    collections: Option<&'a Map<String, Value>>,
    collection_views: Option<&'a Map<String, Value>>,
    users: Option<&'a Map<String, Value>>,
    unavailable_reference_block_ids: &'a std::collections::HashSet<String>,
    opaque_unavailable_blocks: &'a super::ProvenOpaqueUnavailableBlocks,
    allow_partial_preview: bool,
}

impl<'a> PageBlockParseContext<'a> {
    fn property_lookup(&self) -> PropertyLookup<'a> {
        PropertyLookup::completed_page(
            CompletedPageRecords {
                blocks: self.blocks,
                users: self.users,
                collections: self.collections,
                collection_views: self.collection_views,
                unavailable_reference_block_ids: self.unavailable_reference_block_ids,
            },
            self.allow_partial_preview,
        )
    }
}

struct ParsedPageProperties {
    status: Option<String>,
    properties: Vec<CardPageProperty>,
}

pub fn load_card_page(block_id: &str) -> Result<CardPage, String> {
    load_card_page_snapshot(block_id).map(|snapshot| snapshot.page)
}

fn load_card_page_snapshot(block_id: &str) -> Result<LoadedCardPageSnapshot, String> {
    let response = load_complete_page_response_typed(block_id)?;
    card_page_snapshot_from_response(block_id, response)
}

pub(in crate::live) fn load_card_page_snapshot_with_session(
    session: &crate::live::credentials::NotionDesktopSession,
    block_id: &str,
) -> Result<LoadedCardPageSnapshot, crate::live::NotionLiveError> {
    let response = super::load_complete_page_response_with_session(session, block_id)?;
    card_page_snapshot_from_response(block_id, response)
        .map_err(crate::live::NotionLiveError::Fatal)
}

pub(super) fn card_page_snapshot_from_response(
    block_id: &str,
    response: CompletePageResponse,
) -> Result<LoadedCardPageSnapshot, String> {
    Ok(LoadedCardPageSnapshot {
        page: card_page_from_response(block_id, &response)?,
        mutation_state: PageMutationState::parse(block_id, &response)?,
        response,
    })
}

pub(super) fn card_page_from_response(
    block_id: &str,
    response: &CompletePageResponse,
) -> Result<CardPage, String> {
    let response_value = &response.value()?;
    let record_map = response_value
        .get("recordMap")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing recordMap".to_string())?;
    let mut page = card_page_from_record_map(
        block_id,
        record_map,
        response.unavailable_reference_block_ids(),
        response.opaque_unavailable_blocks(),
        false,
    )?;
    page.discussions = page_discussions_from_response(
        block_id,
        page.blocks.iter().map(|block| block.block_id.clone()),
        response_value,
    )?;
    page.validate_block_hierarchy()?;
    Ok(page)
}

pub(super) fn card_page_preview_from_record_map(
    block_id: &str,
    record_map: &Map<String, Value>,
) -> Result<CardPage, String> {
    let unavailable_reference_block_ids = std::collections::HashSet::new();
    let opaque_unavailable_blocks = super::ProvenOpaqueUnavailableBlocks::new();
    let page = card_page_from_record_map(
        block_id,
        record_map,
        &unavailable_reference_block_ids,
        &opaque_unavailable_blocks,
        true,
    )?;
    page.validate_block_hierarchy()?;
    Ok(page)
}

fn card_page_from_record_map(
    block_id: &str,
    record_map: &Map<String, Value>,
    unavailable_reference_block_ids: &std::collections::HashSet<String>,
    opaque_unavailable_blocks: &super::ProvenOpaqueUnavailableBlocks,
    allow_partial_preview: bool,
) -> Result<CardPage, String> {
    let blocks = record_map
        .get("block")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing recordMap.block".to_string())?;
    let root = block_value(blocks, block_id)?;
    let comments_writable = page_comments_writable(blocks, block_id);
    let collections = record_map.get("collection").and_then(Value::as_object);
    let collection_views = record_map.get("collection_view").and_then(Value::as_object);
    let users = record_map.get("notion_user").and_then(Value::as_object);
    let space_id = required_string(root, "space_id")?;
    let title = title_property_allow_empty(root)?.unwrap_or_default();
    let context = PageBlockParseContext {
        space_id,
        blocks,
        collections,
        collection_views,
        users,
        unavailable_reference_block_ids,
        opaque_unavailable_blocks,
        allow_partial_preview,
    };
    let parsed_properties = parsed_page_properties(root, &context)?;
    let blocks = page_blocks(block_id, root, &context)?;
    Ok(CardPage {
        block_id: block_id.to_string(),
        title,
        status: parsed_properties.status,
        properties: parsed_properties.properties,
        blocks,
        discussions: Vec::new(),
        comments_writable,
        format: page_format(root),
    })
}

fn page_format(root: &Value) -> CardPageFormat {
    let format = root.get("format");
    let value = |key: &str| format.and_then(|format| format.get(key));
    let flag = |key: &str| value(key).and_then(Value::as_bool).unwrap_or(false);
    CardPageFormat {
        small_text: flag("page_small_text"),
        full_width: flag("page_full_width"),
        font: match value("page_font").and_then(Value::as_str) {
            Some("serif") => CardPageFont::Serif,
            Some("mono") => CardPageFont::Mono,
            _ => CardPageFont::Default,
        },
    }
}

fn parsed_page_properties(
    root: &Value,
    context: &PageBlockParseContext<'_>,
) -> Result<ParsedPageProperties, String> {
    let root_properties = root.get("properties").and_then(Value::as_object);
    let collection = page_collection(root, context.collections);
    let collection_schema = collection_schema(collection);
    let status = page_status(
        root_properties,
        collection_schema,
        context.blocks,
        context.users,
    )?;
    let property_ids = page_property_ids(collection, root_properties);
    let property_lookup = context.property_lookup();
    let mut properties = page_properties(
        property_ids,
        root_properties,
        collection_schema,
        property_lookup,
    )?;
    properties.sort_by(|left, right| {
        page_property_rank(&left.label)
            .cmp(&page_property_rank(&right.label))
            .then_with(|| left.label.cmp(&right.label))
    });
    Ok(ParsedPageProperties { status, properties })
}

fn page_comments_writable(blocks: &Map<String, Value>, block_id: &str) -> bool {
    blocks
        .get(block_id)
        .and_then(comment_effective_role)
        .is_some_and(|role| {
            matches!(
                role,
                "comment_only"
                    | "content_only_editor"
                    | "read_and_write"
                    | "membership_admin"
                    | "editor"
            )
        })
}

fn comment_effective_role(entry: &Value) -> Option<&str> {
    entry.get("role").and_then(Value::as_str).or_else(|| {
        entry
            .get("value")
            .and_then(Value::as_object)
            .and_then(|value| value.get("role"))
            .and_then(Value::as_str)
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{card_page_from_record_map, card_page_preview_from_record_map};

    #[test]
    fn quick_find_preview_projection_omits_discussions_and_mutation_state() {
        let response = json!({
            "recordMap": {
                "block": {
                    "page-1": {
                        "role": "editor",
                        "value": {
                            "id": "page-1",
                            "space_id": "space-1",
                            "type": "page",
                            "version": 1,
                            "properties": { "title": [["Preview"]] },
                        },
                    },
                },
            },
        });

        let page = card_page_preview_from_record_map(
            "page-1",
            response
                .get("recordMap")
                .and_then(Value::as_object)
                .expect("recordMap"),
        )
        .expect("project Quick Find page");
        assert_eq!(page.title, "Preview");
        assert!(page.discussions.is_empty());
    }

    fn partial_preview_response() -> Value {
        json!({
            "recordMap": {
                "block": {
                    "page-1": {
                        "role": "editor",
                        "value": {
                            "id": "page-1",
                            "space_id": "space-1",
                            "type": "page",
                            "version": 1,
                            "properties": { "title": [["Preview"]] },
                            "content": ["text-1", "missing-child", "unavailable-child"],
                        },
                    },
                    "text-1": {
                        "role": "editor",
                        "value": {
                            "id": "text-1",
                            "space_id": "space-1",
                            "type": "text",
                            "version": 1,
                            "properties": {
                                "title": [["See "], ["‣", [["p", "missing-reference"]]]],
                            },
                        },
                    },
                    "unavailable-child": {
                        "role": "none",
                        "value": {
                            "id": "unavailable-child",
                            "space_id": "space-1",
                            "type": "text",
                            "version": 1,
                            "properties": { "title": [["Private"]]},
                        },
                    },
                },
            },
        })
    }

    #[test]
    fn quick_find_preview_treats_omitted_initial_records_as_unavailable() {
        let response = partial_preview_response();
        let page = card_page_preview_from_record_map(
            "page-1",
            response
                .get("recordMap")
                .and_then(Value::as_object)
                .expect("recordMap"),
        )
        .expect("project partial Quick Find page");

        assert_eq!(page.blocks.len(), 3);
        assert_eq!(page.blocks[0].block_id, "text-1");
        assert!(page.blocks[1].is_opaque_unavailable());
        assert!(page.blocks[2].is_opaque_unavailable());

        let unavailable_reference_block_ids = std::collections::HashSet::new();
        let opaque_unavailable_blocks = super::super::ProvenOpaqueUnavailableBlocks::new();
        let error = card_page_from_record_map(
            "page-1",
            response
                .get("recordMap")
                .and_then(Value::as_object)
                .expect("recordMap"),
            &unavailable_reference_block_ids,
            &opaque_unavailable_blocks,
            false,
        )
        .expect_err("full page parsing must remain strict");
        assert!(error.contains("missing hydrated Notion page reference missing-reference"));
    }
}
