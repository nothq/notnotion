use super::super::super::{
    block_value, format_edited_date_label, normalize_uuid, record_value_state, required_string,
    BoardTarget, LiveWorkspaceSearchContext, Map, RecordValueState, Value,
};
use super::api::{
    load_missing_sidebar_blocks, load_sidebar_attribution_records, load_sidebar_collections_by_ids,
};
use super::records::{combined_block_records, is_available_sidebar_root};
use super::search::{
    combined_search_attribution_records, combined_search_collection_records,
    missing_attribution_pointers, record_entry_by_id, resolve_search_breadcrumb,
    search_result_attribution_display_name, search_result_breadcrumb,
    search_result_has_required_collection, shape_page_shell_search_result, AttributionRecords,
    SearchResultPage,
};
use crate::live::{
    credentials::NotionDesktopSession,
    recent_pages::{load_recent_page_visits, NotionRecentPageVisit},
    NotionLiveError,
};
use crate::model::{
    LoadRecentPagesRequest, LoadRecentPagesResult, PageShellEditedAt, RecentPageResult,
};

struct HydratedRecentBlocks {
    records: Map<String, Value>,
    cache_records: Map<String, Value>,
    collection_ids: Vec<String>,
}

struct HydratedRecentCollections {
    records: Map<String, Value>,
    cache_records: Map<String, Value>,
}

struct HydratedRecentAttribution {
    users: Map<String, Value>,
    bots: Map<String, Value>,
    cache_users: Map<String, Value>,
    cache_bots: Map<String, Value>,
}

struct RecentPageShapeContext<'a> {
    space_id: &'a str,
    blocks: &'a Map<String, Value>,
    collections: &'a Map<String, Value>,
    teams: Option<&'a Map<String, Value>>,
    attribution: &'a HydratedRecentAttribution,
    search_context: &'a LiveWorkspaceSearchContext,
    board_target: &'a BoardTarget,
}

pub(crate) fn load_recent_pages(
    session: &NotionDesktopSession,
    search_context: &LiveWorkspaceSearchContext,
    request: LoadRecentPagesRequest,
) -> Result<LoadRecentPagesResult, NotionLiveError> {
    let board_target = BoardTarget::parse(&request.current_board_url)?;
    let space_id = search_context.space_id.as_str();
    let visits = load_recent_page_visits(session, space_id, request.limit)?;
    let page_ids = visits
        .iter()
        .map(|visit| visit.page_id.clone().into_string())
        .collect::<Vec<_>>();
    let cached_record_map = search_context.quick_find_records.record_map_snapshot()?;
    let empty_blocks = Map::new();
    let cached_blocks = cached_record_map
        .get("block")
        .and_then(Value::as_object)
        .unwrap_or(&empty_blocks);
    let blocks = hydrate_recent_blocks(session, space_id, &page_ids, cached_blocks)?;
    let empty_collections = Map::new();
    let cached_collections = cached_record_map
        .get("collection")
        .and_then(Value::as_object)
        .unwrap_or(&empty_collections);
    let collections = hydrate_recent_collections(
        session,
        space_id,
        &blocks.collection_ids,
        cached_collections,
    )?;
    let attribution = hydrate_recent_attribution(
        session,
        &page_ids,
        &blocks.records,
        AttributionRecords {
            users: cached_record_map
                .get("notion_user")
                .and_then(Value::as_object),
            bots: cached_record_map.get("bot").and_then(Value::as_object),
        },
        search_context,
    )?;
    let results = shape_recent_pages(
        visits,
        RecentPageShapeContext {
            space_id,
            blocks: &blocks.records,
            collections: &collections.records,
            teams: cached_record_map.get("team").and_then(Value::as_object),
            attribution: &attribution,
            search_context,
            board_target: &board_target,
        },
    )?;
    drop(cached_record_map);
    cache_recent_records(search_context, blocks, collections, attribution)?;
    Ok(LoadRecentPagesResult { results })
}

fn cache_recent_records(
    search_context: &LiveWorkspaceSearchContext,
    blocks: HydratedRecentBlocks,
    collections: HydratedRecentCollections,
    attribution: HydratedRecentAttribution,
) -> Result<(), NotionLiveError> {
    search_context.quick_find_records.merge_tables([
        ("block", blocks.cache_records),
        ("collection", collections.cache_records),
        ("notion_user", attribution.cache_users),
        ("bot", attribution.cache_bots),
    ])?;
    Ok(())
}

fn hydrate_recent_blocks(
    session: &NotionDesktopSession,
    space_id: &str,
    page_ids: &[String],
    cached_blocks: &Map<String, Value>,
) -> Result<HydratedRecentBlocks, NotionLiveError> {
    let hydrated_blocks = load_missing_sidebar_blocks(session, space_id, page_ids, cached_blocks)?;
    let cache_blocks = hydrated_blocks.clone();
    let blocks = combined_block_records([cached_blocks, &hydrated_blocks])?;
    let mut collection_ids = Vec::new();
    for page_id in page_ids {
        if !is_available_sidebar_root(&blocks, page_id) {
            continue;
        }
        let block = block_value(&blocks, page_id)?;
        if let Some(collection_id) = block.get("collection_id").and_then(Value::as_str) {
            collection_ids.push(collection_id.to_string());
        }
    }
    Ok(HydratedRecentBlocks {
        records: blocks,
        cache_records: cache_blocks,
        collection_ids,
    })
}

fn hydrate_recent_collections(
    session: &NotionDesktopSession,
    space_id: &str,
    collection_ids: &[String],
    cached_collections: &Map<String, Value>,
) -> Result<HydratedRecentCollections, NotionLiveError> {
    let missing_collection_ids = collection_ids
        .iter()
        .filter(|collection_id| {
            matches!(
                record_value_state(record_entry_by_id(cached_collections, collection_id)),
                RecordValueState::Incomplete
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    let hydrated_collections =
        load_sidebar_collections_by_ids(session, space_id, &missing_collection_ids)?;
    let cache_collections = hydrated_collections.clone();
    let collections =
        combined_search_collection_records(Some(cached_collections), hydrated_collections)?;
    Ok(HydratedRecentCollections {
        records: collections,
        cache_records: cache_collections,
    })
}

fn hydrate_recent_attribution(
    session: &NotionDesktopSession,
    page_ids: &[String],
    blocks: &Map<String, Value>,
    cached_records: AttributionRecords<'_>,
    search_context: &LiveWorkspaceSearchContext,
) -> Result<HydratedRecentAttribution, NotionLiveError> {
    let AttributionRecords {
        users: cached_users,
        bots: cached_bots,
    } = cached_records;
    let space_id = search_context.space_id.as_str();
    let attribution_blocks = page_ids
        .iter()
        .filter(|page_id| is_available_sidebar_root(blocks, page_id))
        .map(|page_id| block_value(blocks, page_id))
        .collect::<Result<Vec<_>, _>>()?;
    let missing_attribution_pointers = missing_attribution_pointers(
        attribution_blocks,
        cached_users,
        cached_bots,
        search_context,
    );
    let hydrated_attribution =
        load_sidebar_attribution_records(session, space_id, &missing_attribution_pointers)?;
    let cache_users = hydrated_attribution.users.clone();
    let cache_bots = hydrated_attribution.bots.clone();
    let users = combined_search_attribution_records(
        "notion_user",
        cached_users,
        &hydrated_attribution.users,
    );
    let bots = combined_search_attribution_records("bot", cached_bots, &hydrated_attribution.bots);
    Ok(HydratedRecentAttribution {
        users,
        bots,
        cache_users,
        cache_bots,
    })
}

fn shape_recent_pages(
    visits: Vec<NotionRecentPageVisit>,
    context: RecentPageShapeContext<'_>,
) -> Result<Vec<RecentPageResult>, NotionLiveError> {
    let mut results = Vec::with_capacity(visits.len());
    for visit in visits {
        if let Some(result) = shape_recent_page(visit, &context)? {
            results.push(result);
        }
    }
    Ok(results)
}

fn shape_recent_page(
    visit: NotionRecentPageVisit,
    context: &RecentPageShapeContext<'_>,
) -> Result<Option<RecentPageResult>, NotionLiveError> {
    let page_id = visit.page_id.into_string();
    if !is_available_sidebar_root(context.blocks, &page_id) {
        return Ok(None);
    }
    let block = block_value(context.blocks, &page_id)?;
    if required_string(block, "space_id")? != context.space_id {
        return Err(NotionLiveError::Fatal(
            "Notion recent pages returned a result from a different space".to_string(),
        ));
    }
    let collection_id = block.get("collection_id").and_then(Value::as_str);
    if !search_result_has_required_collection(block, collection_id, context.collections)? {
        return Ok(None);
    }
    let record_breadcrumb = search_result_breadcrumb(
        block,
        context.blocks,
        Some(context.collections),
        context.teams,
    )?;
    let breadcrumb = resolve_search_breadcrumb(
        record_breadcrumb,
        context.search_context,
        &normalize_uuid(&page_id),
    );
    let mut page = shape_page_shell_search_result(
        SearchResultPage {
            block_id: &page_id,
            collection_id,
            block,
        },
        context.board_target,
        Some(context.collections),
        breadcrumb,
    )?;
    page.editor_display_name = search_result_attribution_display_name(
        block,
        context.search_context,
        Some(&context.attribution.users),
        Some(&context.attribution.bots),
    );
    page.edited_at = block
        .get("last_edited_time")
        .and_then(Value::as_u64)
        .map(|unix_millis| PageShellEditedAt {
            unix_millis,
            date_label: format_edited_date_label(unix_millis, &context.search_context.time_zone),
        });
    page.edited_label = None;
    Ok(Some(RecentPageResult {
        page,
        visited_at_unix_millis: visit.visited_at_unix_millis,
    }))
}
