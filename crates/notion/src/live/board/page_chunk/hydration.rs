use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::json;

use super::{
    block_record::{
        page_block_record_state, unavailable_page_block_kind, AuthoritativePageBlockUnavailable,
        CompletePageBlock, PageBlockRecordState, PageBlockRequirement,
    },
    opaque::ProvenOpaqueUnavailableBlocks,
    HydratedPageBlockScope,
};
use crate::live::board::{loaded_record_value, record_map_table, NotionPrivateApiEndpoint, Value};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

const PAGE_BLOCK_HYDRATION_MAX_WAVES: usize = 64;

mod merge;
mod opaque_blocks;
mod pending;
mod stale_edges;

use merge::merge_authoritative_blocks;
use opaque_blocks::{
    consume_proven_opaque_block, merge_opaque_proofs, page_block_scope, restart_page_traversal,
};
use pending::{MissingPageBlocks, PendingPageBlock};
use stale_edges::{refresh_unavailable_content_parents, UnavailableContentRefresh};

struct PageTraversalState {
    loaded: HashSet<String>,
    expanded: HashSet<String>,
    required_by_id: HashMap<String, PendingPageBlock>,
    unavailable_reference_block_ids: HashSet<String>,
    opaque_unavailable_blocks: ProvenOpaqueUnavailableBlocks,
    reachable_opaque_block_ids: HashSet<String>,
    collection_ids_by_space: BTreeMap<String, HashSet<String>>,
    linked_view_ids_by_space: BTreeMap<String, BTreeSet<String>>,
    user_ids: HashSet<String>,
    pending: Vec<PendingPageBlock>,
}

pub(super) fn hydrate_reachable_page_blocks(
    session: &NotionDesktopSession,
    page_id: &str,
    response: &mut Value,
) -> Result<Option<HydratedPageBlockScope>, NotionLiveError> {
    hydrate_page_root(session, page_id, response)?;
    let (root_type, space_id) = completed_page_root(page_id, response)?;
    if !matches!(root_type.as_str(), "page" | "transcription") {
        return Ok(None);
    }
    hydrate_page_scope(session, page_id, space_id, response).map(Some)
}

fn hydrate_page_root(
    session: &NotionDesktopSession,
    page_id: &str,
    response: &mut Value,
) -> Result<(), NotionLiveError> {
    for _ in 0..PAGE_BLOCK_HYDRATION_MAX_WAVES {
        if page_root_is_complete(page_id, response)? {
            return Ok(());
        }
        let request = json!({
            "requests": [{
                "pointer": { "table": "block", "id": page_id },
                "version": -1,
            }],
        });
        let hydrated = post_private_api_with_session(
            session,
            NotionPrivateApiEndpoint::SyncRecordValuesMain,
            &request,
        )?;
        let _ = merge_authoritative_blocks(
            response,
            hydrated,
            &[(page_id, PageBlockRequirement::Root)],
        )?;
    }
    if page_root_is_complete(page_id, response)? {
        return Ok(());
    }
    Err(NotionLiveError::Fatal(format!(
        "Notion page {page_id} exceeded {PAGE_BLOCK_HYDRATION_MAX_WAVES} root hydration waves"
    )))
}

fn page_root_is_complete(page_id: &str, response: &Value) -> Result<bool, String> {
    let blocks = record_map_table(response, "block")?;
    Ok(matches!(
        page_block_record_state(blocks, page_id, PageBlockRequirement::Root)?,
        PageBlockRecordState::Complete(_)
    ))
}

fn completed_page_root(page_id: &str, response: &Value) -> Result<(String, String), String> {
    let blocks = record_map_table(response, "block")?;
    match page_block_record_state(blocks, page_id, PageBlockRequirement::Root)? {
        PageBlockRecordState::Complete(root) => {
            Ok((root.block_type().to_string(), root.space_id().to_string()))
        }
        PageBlockRecordState::Unavailable => {
            Err(format!("Notion root page block {page_id} is unavailable"))
        }
        PageBlockRecordState::NoValue | PageBlockRecordState::Incomplete => Err(format!(
            "Notion did not hydrate complete root page block {page_id}"
        )),
    }
}

fn hydrate_page_scope(
    session: &NotionDesktopSession,
    page_id: &str,
    space_id: String,
    response: &mut Value,
) -> Result<HydratedPageBlockScope, NotionLiveError> {
    let mut traversal = PageTraversalState {
        loaded: HashSet::new(),
        expanded: HashSet::new(),
        required_by_id: HashMap::new(),
        unavailable_reference_block_ids: HashSet::new(),
        opaque_unavailable_blocks: ProvenOpaqueUnavailableBlocks::new(),
        reachable_opaque_block_ids: HashSet::new(),
        collection_ids_by_space: BTreeMap::new(),
        linked_view_ids_by_space: BTreeMap::new(),
        user_ids: HashSet::new(),
        pending: vec![PendingPageBlock::root(page_id, &space_id)],
    };
    for _ in 0..PAGE_BLOCK_HYDRATION_MAX_WAVES {
        let missing = collect_missing_page_blocks(page_id, response, &mut traversal)?;
        if missing.is_empty() {
            return Ok(page_block_scope(page_id, space_id, traversal));
        }
        hydrate_missing_page_blocks(session, missing, response, &mut traversal)?;
    }
    let missing = collect_missing_page_blocks(page_id, response, &mut traversal)?;
    if missing.is_empty() {
        return Ok(page_block_scope(page_id, space_id, traversal));
    }
    Err(NotionLiveError::Fatal(format!(
        "Notion page {page_id} exceeded {PAGE_BLOCK_HYDRATION_MAX_WAVES} missing-block hydration waves"
    )))
}

fn collect_missing_page_blocks(
    page_id: &str,
    response: &Value,
    traversal: &mut PageTraversalState,
) -> Result<MissingPageBlocks, String> {
    let blocks = record_map_table(response, "block")?;
    let mut missing = MissingPageBlocks::default();
    while let Some(pending_block) = traversal.pending.pop() {
        let has_record_value = blocks
            .get(&pending_block.block_id)
            .and_then(loaded_record_value)
            .is_some();
        if consume_proven_opaque_block(&pending_block, has_record_value, traversal)? {
            continue;
        }
        if traversal.loaded.contains(&pending_block.block_id)
            && (!pending_block.follows_content()
                || traversal.expanded.contains(&pending_block.block_id))
        {
            merge_required_provenance(&pending_block, traversal)?;
            continue;
        }
        let requirement = pending_block.requirement();
        match page_block_record_state(blocks, &pending_block.block_id, requirement)? {
            PageBlockRecordState::NoValue | PageBlockRecordState::Incomplete => {
                missing.insert(pending_block)?;
            }
            PageBlockRecordState::Unavailable => {
                let entry = blocks.get(&pending_block.block_id).ok_or_else(|| {
                    format!(
                        "unavailable Notion block {} is missing its record entry",
                        pending_block.block_id
                    )
                })?;
                let unavailable = unavailable_page_block_kind(entry, &pending_block.block_id)?;
                record_unavailable_page_block(pending_block, unavailable, traversal, &mut missing)?;
            }
            PageBlockRecordState::Complete(block) => {
                append_complete_page_block(page_id, *block, pending_block, traversal)?;
            }
        }
    }
    Ok(missing)
}

fn record_unavailable_page_block(
    pending_block: PendingPageBlock,
    unavailable: AuthoritativePageBlockUnavailable,
    traversal: &mut PageTraversalState,
    missing: &mut MissingPageBlocks,
) -> Result<(), String> {
    if pending_block.is_reference_target() {
        let block_id = pending_block.block_id;
        traversal.loaded.insert(block_id.clone());
        traversal.unavailable_reference_block_ids.insert(block_id);
        return Ok(());
    }
    if pending_block.is_root() {
        return Err(format!(
            "Notion root page block {} is unavailable",
            pending_block.block_id
        ));
    }
    merge_required_provenance(&pending_block, traversal)?;
    missing.insert_unavailable_content(pending_block, unavailable)
}

fn append_complete_page_block(
    page_id: &str,
    block: CompletePageBlock<'_>,
    pending_block: PendingPageBlock,
    traversal: &mut PageTraversalState,
) -> Result<(), String> {
    validate_complete_page_block(&block, &pending_block)?;
    merge_required_provenance(&pending_block, traversal)?;
    traversal
        .unavailable_reference_block_ids
        .remove(&pending_block.block_id);
    traversal.loaded.insert(pending_block.block_id.clone());
    for pointer in block.collection_pointers() {
        traversal
            .collection_ids_by_space
            .entry(pointer.space_id.to_string())
            .or_default()
            .insert(pointer.id.to_string());
    }
    if !block.linked_view_ids().is_empty() {
        traversal
            .linked_view_ids_by_space
            .entry(block.space_id().to_string())
            .or_default()
            .extend(block.linked_view_ids().iter().map(|id| (*id).to_string()));
    }
    traversal
        .user_ids
        .extend(block.property_user_ids().iter().map(|id| (*id).to_string()));
    traversal.pending.extend(
        block
            .property_block_ids()
            .iter()
            .map(|id| PendingPageBlock::reference_target(id, block.space_id())),
    );
    if let Some(reference) = block.reference() {
        traversal.pending.push(PendingPageBlock::reference_target(
            reference.id,
            reference.space_id,
        ));
    }
    if !pending_block.follows_content() {
        return Ok(());
    }
    traversal.expanded.insert(pending_block.block_id.clone());
    if pending_block.block_id != page_id && page_block_starts_separate_scope(block.block_type()) {
        return Ok(());
    }
    traversal
        .pending
        .extend(block.content_ids().iter().map(|child_id| {
            PendingPageBlock::content_child(child_id, block.space_id(), &pending_block.block_id)
        }));
    Ok(())
}

fn validate_complete_page_block(
    block: &CompletePageBlock<'_>,
    pending_block: &PendingPageBlock,
) -> Result<(), String> {
    if block.space_id() != pending_block.space_id {
        return Err(format!(
            "Notion page expected block {} in space {}, found {}",
            pending_block.block_id,
            pending_block.space_id,
            block.space_id()
        ));
    }
    if let Some(parent_block_id) = pending_block.content_parent_id() {
        if block.parent_table() != "block" || block.parent_id() != parent_block_id {
            return Err(format!(
                "Notion content edge {parent_block_id} -> {} resolves to parent {} {}",
                pending_block.block_id,
                block.parent_table(),
                block.parent_id()
            ));
        }
    }
    Ok(())
}

fn merge_required_provenance(
    pending_block: &PendingPageBlock,
    traversal: &mut PageTraversalState,
) -> Result<(), String> {
    if !pending_block.follows_content() {
        return Ok(());
    }
    match traversal
        .required_by_id
        .entry(pending_block.block_id.clone())
    {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(pending_block.clone());
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            entry.get_mut().merge_provenance(pending_block.clone())?;
        }
    }
    Ok(())
}

fn hydrate_missing_page_blocks(
    session: &NotionDesktopSession,
    mut missing: MissingPageBlocks,
    response: &mut Value,
    traversal: &mut PageTraversalState,
) -> Result<(), NotionLiveError> {
    let missing_by_space = std::mem::take(&mut missing.by_space);
    for (space_id, targets) in missing_by_space {
        let requirements = targets
            .iter()
            .map(|(block_id, pending)| (block_id.clone(), pending.requirement()))
            .collect::<BTreeMap<_, _>>();
        let unavailable = hydrate_page_block_wave(session, &space_id, &requirements, response)?;
        for (block_id, pending) in targets {
            if let Some(unavailable) = unavailable.get(&block_id).copied() {
                record_unavailable_page_block(pending, unavailable, traversal, &mut missing)?;
            } else {
                traversal.pending.push(pending);
            }
        }
    }
    let refreshed = refresh_unavailable_content_parents(
        session,
        &missing.unavailable_content_edges_by_space,
        response,
        &traversal.required_by_id,
    )?;
    if let UnavailableContentRefresh::Restart { opaque_blocks } = refreshed {
        merge_opaque_proofs(traversal, opaque_blocks)?;
        restart_page_traversal(traversal)?;
    }
    Ok(())
}

fn hydrate_page_block_wave(
    session: &NotionDesktopSession,
    space_id: &str,
    block_targets: &BTreeMap<String, PageBlockRequirement>,
    response: &mut Value,
) -> Result<BTreeMap<String, AuthoritativePageBlockUnavailable>, NotionLiveError> {
    let requests = block_targets
        .keys()
        .map(|block_id| {
            json!({
                "pointer": { "table": "block", "id": block_id, "spaceId": space_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let hydrated = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesSpace,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    let targets = block_targets
        .iter()
        .map(|(block_id, requirement)| (block_id.as_str(), *requirement))
        .collect::<Vec<_>>();
    merge_authoritative_blocks(response, hydrated, &targets).map_err(NotionLiveError::Fatal)
}

fn page_block_starts_separate_scope(block_type: &str) -> bool {
    matches!(
        block_type,
        "page" | "link_to_page" | "alias" | "collection_view" | "collection_view_page"
    )
}
