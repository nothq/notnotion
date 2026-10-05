use std::collections::HashSet;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::super::super::super::{
    normalize_uuid, optional_record_map_table, record_map::merge_record_table, record_value_state,
    BoardTarget, LiveWorkspaceSearchContext, NotionPrivateApiEndpoint, RecordValueState,
};
use super::super::api::{
    load_missing_sidebar_blocks, load_sidebar_attribution_records, load_sidebar_collections_by_ids,
};
use super::super::records::combined_block_records;
use super::query::search_request_body;
use super::result::{
    search_collection_state, search_result_attribution_pointer, search_result_block,
    search_result_collection_id, shape_search_results, SearchRecordMaps,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use crate::model::{SearchWorkspaceRequest, SearchWorkspaceResult};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchResponse {
    pub(super) total: u32,
    pub(super) results: Vec<SearchResultWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SearchResultWire {
    pub(super) id: String,
    pub(super) space_id: String,
    pub(super) collection_id: Option<String>,
    #[serde(default)]
    pub(super) highlight: Option<SearchHighlightWire>,
}

#[derive(Deserialize)]
pub(super) struct SearchHighlightWire {
    #[serde(default)]
    pub(super) text: Option<String>,
}

struct HydratedSearchRecords {
    blocks: Map<String, Value>,
    collections: Map<String, Value>,
    teams: Option<Map<String, Value>>,
    users: Map<String, Value>,
    bots: Map<String, Value>,
    cache_blocks: Map<String, Value>,
    cache_collections: Map<String, Value>,
    cache_users: Map<String, Value>,
    cache_bots: Map<String, Value>,
}

struct HydratedSearchAttribution {
    users: Map<String, Value>,
    bots: Map<String, Value>,
    cache_users: Map<String, Value>,
    cache_bots: Map<String, Value>,
}

type CombinedAndHydratedRecords = (Map<String, Value>, Map<String, Value>);

/// The user and bot records that attribute pages to their editors.
pub(in super::super) struct AttributionRecords<'a> {
    pub(in super::super) users: Option<&'a Map<String, Value>>,
    pub(in super::super) bots: Option<&'a Map<String, Value>>,
}

pub(crate) fn search_workspace(
    session: &NotionDesktopSession,
    search_context: &LiveWorkspaceSearchContext,
    request: SearchWorkspaceRequest,
) -> Result<SearchWorkspaceResult, NotionLiveError> {
    let board_target = BoardTarget::parse(&request.current_board_url)?;
    let body = search_request_body(&search_context.space_id, &request);
    let response: Value =
        post_private_api_with_session(session, NotionPrivateApiEndpoint::Search, &body)?;
    let decoded = serde_json::from_value::<SearchResponse>(response.clone())
        .map_err(|error| format!("invalid Notion private API search response: {error}"))?;
    let records = hydrate_search_records(session, search_context, &decoded, &response)?;
    let consumed_result_count = u32::try_from(decoded.results.len())
        .map_err(|_| "Notion search result count does not fit in a u32".to_string())?;
    let results = shape_search_results(
        &decoded.results,
        search_context,
        &board_target,
        SearchRecordMaps {
            blocks: &records.blocks,
            collections: &records.collections,
            teams: records.teams.as_ref(),
            users: Some(&records.users),
            bots: Some(&records.bots),
        },
    )?;
    cache_search_records(search_context, &response, records)?;
    Ok(SearchWorkspaceResult {
        total: decoded.total,
        consumed_result_count,
        results,
    })
}

fn hydrate_search_records(
    session: &NotionDesktopSession,
    search_context: &LiveWorkspaceSearchContext,
    decoded: &SearchResponse,
    response: &Value,
) -> Result<HydratedSearchRecords, NotionLiveError> {
    let empty_blocks = Map::new();
    let response_blocks = optional_record_map_table(response, "block").unwrap_or(&empty_blocks);
    let (blocks, cache_blocks) =
        hydrate_search_blocks(session, search_context, decoded, response_blocks)?;
    let response_collections = optional_record_map_table(response, "collection");
    let (collections, cache_collections) = hydrate_search_collections(
        session,
        search_context,
        decoded,
        &blocks,
        response_collections,
    )?;
    let attribution = hydrate_search_attribution(
        session,
        search_context,
        decoded,
        &blocks,
        AttributionRecords {
            users: optional_record_map_table(response, "notion_user"),
            bots: optional_record_map_table(response, "bot"),
        },
    )?;
    Ok(HydratedSearchRecords {
        blocks,
        collections,
        teams: optional_record_map_table(response, "team").cloned(),
        users: attribution.users,
        bots: attribution.bots,
        cache_blocks,
        cache_collections,
        cache_users: attribution.cache_users,
        cache_bots: attribution.cache_bots,
    })
}

fn hydrate_search_blocks(
    session: &NotionDesktopSession,
    search_context: &LiveWorkspaceSearchContext,
    decoded: &SearchResponse,
    response_blocks: &Map<String, Value>,
) -> Result<CombinedAndHydratedRecords, NotionLiveError> {
    let result_ids = decoded
        .results
        .iter()
        .map(|result| result.id.clone())
        .collect::<Vec<_>>();
    let hydrated = load_missing_sidebar_blocks(
        session,
        &search_context.space_id,
        &result_ids,
        response_blocks,
    )?;
    let combined = combined_block_records([response_blocks, &hydrated])?;
    Ok((combined, hydrated))
}

fn hydrate_search_collections(
    session: &NotionDesktopSession,
    search_context: &LiveWorkspaceSearchContext,
    decoded: &SearchResponse,
    blocks: &Map<String, Value>,
    response_collections: Option<&Map<String, Value>>,
) -> Result<CombinedAndHydratedRecords, NotionLiveError> {
    let missing = missing_search_collection_ids(&decoded.results, blocks, response_collections)?;
    let hydrated = load_sidebar_collections_by_ids(session, &search_context.space_id, &missing)?;
    let combined = combined_search_collection_records(response_collections, hydrated.clone())?;
    Ok((combined, hydrated))
}

fn hydrate_search_attribution(
    session: &NotionDesktopSession,
    search_context: &LiveWorkspaceSearchContext,
    decoded: &SearchResponse,
    blocks: &Map<String, Value>,
    response_records: AttributionRecords<'_>,
) -> Result<HydratedSearchAttribution, NotionLiveError> {
    let AttributionRecords {
        users: response_users,
        bots: response_bots,
    } = response_records;
    let attribution_blocks = decoded
        .results
        .iter()
        .map(|result| {
            search_result_block(result, blocks).map(|block| block.map(|(_, block)| block))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let missing = missing_attribution_pointers(
        attribution_blocks.into_iter().flatten(),
        response_users,
        response_bots,
        search_context,
    );
    let hydrated = load_sidebar_attribution_records(session, &search_context.space_id, &missing)?;
    Ok(HydratedSearchAttribution {
        users: combined_search_attribution_records("notion_user", response_users, &hydrated.users),
        bots: combined_search_attribution_records("bot", response_bots, &hydrated.bots),
        cache_users: hydrated.users,
        cache_bots: hydrated.bots,
    })
}

fn cache_search_records(
    search_context: &LiveWorkspaceSearchContext,
    response: &Value,
    records: HydratedSearchRecords,
) -> Result<(), NotionLiveError> {
    if response.get("recordMap").is_some() {
        search_context.quick_find_records.merge_response(response)?;
    }
    search_context.quick_find_records.merge_tables([
        ("block", records.cache_blocks),
        ("collection", records.cache_collections),
        ("notion_user", records.cache_users),
        ("bot", records.cache_bots),
    ])?;
    Ok(())
}

pub(super) fn missing_search_collection_ids(
    results: &[SearchResultWire],
    blocks: &Map<String, Value>,
    response_collections: Option<&Map<String, Value>>,
) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut missing = Vec::new();
    for result in results {
        let Some((_, block)) = search_result_block(result, blocks)? else {
            continue;
        };
        let Some(collection_id) = search_result_collection_id(result, block) else {
            continue;
        };
        if !seen.insert(normalize_uuid(&collection_id))
            || !matches!(
                search_collection_state(response_collections, &collection_id),
                RecordValueState::Incomplete
            )
        {
            continue;
        }
        missing.push(collection_id);
    }
    Ok(missing)
}

pub(in super::super) fn combined_search_collection_records(
    response_collections: Option<&Map<String, Value>>,
    hydrated_collections: Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let mut collections = response_collections.cloned().unwrap_or_default();
    merge_record_table("collection", &mut collections, hydrated_collections)?;
    Ok(collections)
}

pub(in super::super) fn combined_search_attribution_records(
    table: &str,
    response_records: Option<&Map<String, Value>>,
    hydrated_records: &Map<String, Value>,
) -> Map<String, Value> {
    let response_records = response_records.cloned().unwrap_or_default();
    let mut records = response_records.clone();
    if let Err(error) = merge_record_table(table, &mut records, hydrated_records.clone()) {
        println!("notnotion Quick Find omitted invalid optional {table} attribution: {error}");
        return response_records;
    }
    records
}

pub(in super::super) fn missing_attribution_pointers<'a>(
    blocks: impl IntoIterator<Item = &'a Value>,
    users: Option<&Map<String, Value>>,
    bots: Option<&Map<String, Value>>,
    search_context: &LiveWorkspaceSearchContext,
) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut missing = Vec::new();
    for block in blocks {
        let Some((table, id)) = search_result_attribution_pointer(block) else {
            continue;
        };
        if table == "notion_user"
            && normalize_uuid(id) == normalize_uuid(&search_context.active_user_id)
        {
            continue;
        }
        let records = match table {
            "notion_user" => users,
            "bot" => bots,
            _ => continue,
        };
        let key = (table.to_string(), normalize_uuid(id));
        if !seen.insert(key)
            || matches!(
                record_value_state(records.and_then(|records| record_entry_by_id(records, id))),
                RecordValueState::Present(_)
            )
        {
            continue;
        }
        missing.push((table.to_string(), id.to_string()));
    }
    missing
}

fn record_entry_by_id<'a>(records: &'a Map<String, Value>, record_id: &str) -> Option<&'a Value> {
    super::result::record_entry_by_id(records, record_id)
}
