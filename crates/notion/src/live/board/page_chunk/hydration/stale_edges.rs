use std::collections::{BTreeMap, HashMap};

use super::super::block_record::{
    page_block_record_state, AuthoritativePageBlockUnavailable, PageBlockRecordState,
    PageBlockRequirement,
};
use super::super::opaque::{
    insert_proven_opaque_block, ProvenOpaqueUnavailableBlock, ProvenOpaqueUnavailableBlocks,
};
use super::{
    hydrate_page_block_wave,
    pending::{PendingPageBlock, UnavailableContentEdgesBySpace},
};
use crate::live::board::{record_map_table, Value};
use crate::live::{credentials::NotionDesktopSession, NotionLiveError};

type ParentRequirementsBySpace = BTreeMap<String, BTreeMap<String, PageBlockRequirement>>;
type UnavailableParentsBySpace =
    BTreeMap<String, BTreeMap<String, AuthoritativePageBlockUnavailable>>;

pub(super) enum UnavailableContentRefresh {
    NotNeeded,
    Restart {
        opaque_blocks: ProvenOpaqueUnavailableBlocks,
    },
}

pub(super) fn refresh_unavailable_content_parents(
    session: &NotionDesktopSession,
    edges_by_space: &UnavailableContentEdgesBySpace,
    response: &mut Value,
    required_by_id: &HashMap<String, PendingPageBlock>,
) -> Result<UnavailableContentRefresh, NotionLiveError> {
    if edges_by_space.is_empty() {
        return Ok(UnavailableContentRefresh::NotNeeded);
    }
    let mut pending_edges_by_space = edges_by_space.clone();
    let mut opaque_blocks = ProvenOpaqueUnavailableBlocks::new();
    for _ancestor_wave in 1..=super::PAGE_BLOCK_HYDRATION_MAX_WAVES {
        let parent_requirements_by_space =
            parent_requirements_by_space(&pending_edges_by_space, required_by_id)?;
        let unavailable_parents_by_space =
            refresh_parent_wave(session, &parent_requirements_by_space, response)?;
        verify_available_parent_edges(
            &pending_edges_by_space,
            &parent_requirements_by_space,
            &unavailable_parents_by_space,
            response,
            &mut opaque_blocks,
        )?;
        let next_edges_by_space =
            unavailable_parent_edges(unavailable_parents_by_space, required_by_id)?;
        if next_edges_by_space.is_empty() {
            return Ok(UnavailableContentRefresh::Restart { opaque_blocks });
        }
        pending_edges_by_space = next_edges_by_space;
    }
    Err(NotionLiveError::Fatal(format!(
        "Notion page exceeded {} unavailable content-edge ancestor waves",
        super::PAGE_BLOCK_HYDRATION_MAX_WAVES
    )))
}

fn parent_requirements_by_space(
    edges_by_space: &UnavailableContentEdgesBySpace,
    required_by_id: &HashMap<String, PendingPageBlock>,
) -> Result<ParentRequirementsBySpace, NotionLiveError> {
    let mut requirements_by_space = BTreeMap::new();
    for (space_id, edges_by_parent) in edges_by_space {
        let requirements = requirements_by_space
            .entry(space_id.clone())
            .or_insert_with(BTreeMap::new);
        for parent_block_id in edges_by_parent.keys() {
            let pending_parent = required_by_id.get(parent_block_id).ok_or_else(|| {
                NotionLiveError::Fatal(format!(
                    "Notion content-edge parent {parent_block_id} was not traversed"
                ))
            })?;
            if pending_parent.space_id != *space_id {
                return Err(NotionLiveError::Fatal(format!(
                    "Notion content-edge parent {parent_block_id} was expected in space {}, found {space_id}",
                    pending_parent.space_id
                )));
            }
            requirements.insert(parent_block_id.clone(), pending_parent.requirement());
        }
    }
    Ok(requirements_by_space)
}

fn refresh_parent_wave(
    session: &NotionDesktopSession,
    parent_requirements_by_space: &ParentRequirementsBySpace,
    response: &mut Value,
) -> Result<UnavailableParentsBySpace, NotionLiveError> {
    let mut unavailable_by_space = BTreeMap::new();
    for (space_id, parent_requirements) in parent_requirements_by_space {
        let unavailable =
            hydrate_page_block_wave(session, space_id, parent_requirements, response)?;
        if !unavailable.is_empty() {
            unavailable_by_space.insert(space_id.clone(), unavailable);
        }
    }
    Ok(unavailable_by_space)
}

fn verify_available_parent_edges(
    edges_by_space: &UnavailableContentEdgesBySpace,
    requirements_by_space: &ParentRequirementsBySpace,
    unavailable_by_space: &UnavailableParentsBySpace,
    response: &Value,
    opaque_blocks: &mut ProvenOpaqueUnavailableBlocks,
) -> Result<(), NotionLiveError> {
    let blocks = record_map_table(response, "block")?;
    for (space_id, edges_by_parent) in edges_by_space {
        let parent_requirements = requirements_by_space.get(space_id).ok_or_else(|| {
            NotionLiveError::Fatal(format!(
                "Notion content-edge parent refresh lost space {space_id}"
            ))
        })?;
        let unavailable_parents = unavailable_by_space.get(space_id);
        for (parent_block_id, unavailable_child_ids) in edges_by_parent {
            if unavailable_parents.is_some_and(|parents| parents.contains_key(parent_block_id)) {
                continue;
            }
            let requirement = parent_requirements
                .get(parent_block_id)
                .copied()
                .ok_or_else(|| {
                    NotionLiveError::Fatal(format!(
                        "Notion content-edge parent {parent_block_id} lost its refresh requirement"
                    ))
                })?;
            verify_parent_edges(
                blocks,
                ParentContentEdges {
                    space_id,
                    parent_block_id,
                    unavailable_child_ids,
                    requirement,
                },
                opaque_blocks,
            )?;
        }
    }
    Ok(())
}

/// A refreshed parent's content edges to children Notion reported unavailable.
struct ParentContentEdges<'a> {
    space_id: &'a str,
    parent_block_id: &'a str,
    unavailable_child_ids: &'a BTreeMap<String, AuthoritativePageBlockUnavailable>,
    requirement: PageBlockRequirement,
}

fn verify_parent_edges(
    blocks: &crate::live::board::Map<String, Value>,
    edges: ParentContentEdges<'_>,
    opaque_blocks: &mut ProvenOpaqueUnavailableBlocks,
) -> Result<(), NotionLiveError> {
    let ParentContentEdges {
        space_id,
        parent_block_id,
        unavailable_child_ids,
        requirement,
    } = edges;
    let parent = match page_block_record_state(blocks, parent_block_id, requirement)? {
        PageBlockRecordState::Complete(parent) => parent,
        PageBlockRecordState::Unavailable => {
            return Err(NotionLiveError::Fatal(format!(
                "Notion parent {parent_block_id} was unavailable without a typed refresh outcome"
            )));
        }
        PageBlockRecordState::NoValue | PageBlockRecordState::Incomplete => {
            return Err(NotionLiveError::Fatal(format!(
                "Notion did not hydrate complete content-edge parent {parent_block_id}"
            )));
        }
    };
    for (child_id, unavailable) in unavailable_child_ids {
        if parent.content_ids().contains(&child_id.as_str()) {
            record_retained_unavailable_child(
                child_id,
                parent_block_id,
                space_id,
                *unavailable,
                opaque_blocks,
            )?;
        }
    }
    Ok(())
}

fn record_retained_unavailable_child(
    child_id: &str,
    parent_block_id: &str,
    space_id: &str,
    unavailable: AuthoritativePageBlockUnavailable,
    opaque_blocks: &mut ProvenOpaqueUnavailableBlocks,
) -> Result<(), NotionLiveError> {
    let state = match unavailable {
        AuthoritativePageBlockUnavailable::ExplicitRoleNone => {
            let proof = ProvenOpaqueUnavailableBlock::from_authoritative_content_edge(
                child_id,
                parent_block_id,
                space_id,
            );
            return insert_proven_opaque_block(opaque_blocks, proof)
                .map_err(NotionLiveError::Fatal);
        }
        AuthoritativePageBlockUnavailable::Deleted => "a deleted record",
        AuthoritativePageBlockUnavailable::Missing => "no record entry",
        AuthoritativePageBlockUnavailable::Fragment => "an incomplete record fragment",
    };
    Err(retained_unavailable_edge_error(
        child_id,
        parent_block_id,
        state,
    ))
}

fn unavailable_parent_edges(
    unavailable_by_space: UnavailableParentsBySpace,
    required_by_id: &HashMap<String, PendingPageBlock>,
) -> Result<UnavailableContentEdgesBySpace, NotionLiveError> {
    let mut edges_by_space = UnavailableContentEdgesBySpace::new();
    for (space_id, unavailable_parent_ids) in unavailable_by_space {
        for (parent_block_id, unavailable) in unavailable_parent_ids {
            let pending_parent = required_by_id.get(&parent_block_id).ok_or_else(|| {
                NotionLiveError::Fatal(format!(
                    "unavailable Notion parent {parent_block_id} lost its content-edge provenance"
                ))
            })?;
            let ancestor_id = pending_parent.content_parent_id().ok_or_else(|| {
                NotionLiveError::Fatal(format!(
                    "required Notion root block {parent_block_id} is unavailable"
                ))
            })?;
            edges_by_space
                .entry(space_id.clone())
                .or_default()
                .entry(ancestor_id.to_string())
                .or_default()
                .insert(parent_block_id, unavailable);
        }
    }
    Ok(edges_by_space)
}

fn retained_unavailable_edge_error(
    child_id: &str,
    parent_block_id: &str,
    state: &str,
) -> NotionLiveError {
    NotionLiveError::Fatal(format!(
        "required Notion block {child_id} returned {state} while authoritative parent {parent_block_id} still references it"
    ))
}
