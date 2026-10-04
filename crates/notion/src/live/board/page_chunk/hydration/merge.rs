use std::collections::BTreeMap;

use super::super::block_record::{
    apply_authoritative_page_block_omissions, missing_authoritative_page_block,
    normalize_authoritative_page_block_entry, AuthoritativePageBlockState,
    AuthoritativePageBlockUnavailable, PageBlockRequirement,
};
use crate::live::board::{record_map::merge_record_map, Map, Value};

pub(super) fn merge_authoritative_blocks(
    response: &mut Value,
    mut hydrated: Value,
    targets: &[(&str, PageBlockRequirement)],
) -> Result<BTreeMap<String, AuthoritativePageBlockUnavailable>, String> {
    let hydrated_record_map = hydrated
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in hydrated Notion page blocks".to_string())?;
    let outcomes = normalize_hydrated_targets(hydrated_record_map, targets)?;
    let unavailable = outcomes
        .iter()
        .filter_map(|(block_id, outcome)| {
            if let AuthoritativePageBlockState::Unavailable(unavailable) = outcome {
                Some((block_id.clone(), *unavailable))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();
    apply_hydrated_outcomes(response, hydrated_record_map, outcomes)?;
    remove_unavailable_targets(hydrated_record_map, &unavailable)?;
    let hydrated_record_map = std::mem::take(hydrated_record_map);
    let response_record_map = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap in Notion page response".to_string())?;
    merge_record_map(response_record_map, hydrated_record_map)?;
    Ok(unavailable)
}

fn remove_unavailable_targets(
    hydrated_record_map: &mut Map<String, Value>,
    unavailable: &BTreeMap<String, AuthoritativePageBlockUnavailable>,
) -> Result<(), String> {
    let blocks = hydrated_record_map
        .get_mut("block")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing authoritative Notion page block table".to_string())?;
    for block_id in unavailable.keys() {
        blocks.remove(block_id);
    }
    Ok(())
}

fn normalize_hydrated_targets(
    hydrated_record_map: &mut Map<String, Value>,
    targets: &[(&str, PageBlockRequirement)],
) -> Result<BTreeMap<String, AuthoritativePageBlockState>, String> {
    let blocks = hydrated_record_map
        .entry("block".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| {
            "recordMap.block in hydrated Notion page blocks is not an object".to_string()
        })?;
    let mut outcomes = BTreeMap::new();
    for (block_id, requirement) in targets {
        let state = match blocks.get_mut(*block_id) {
            Some(entry) => normalize_authoritative_page_block_entry(entry, block_id, *requirement)?,
            None => missing_authoritative_page_block(block_id, *requirement)?,
        };
        outcomes.insert((*block_id).to_string(), state);
    }
    Ok(outcomes)
}

fn apply_hydrated_outcomes(
    response: &mut Value,
    hydrated_record_map: &Map<String, Value>,
    outcomes: BTreeMap<String, AuthoritativePageBlockState>,
) -> Result<(), String> {
    let authoritative_blocks = hydrated_record_map
        .get("block")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing authoritative Notion page block table".to_string())?;
    let existing_blocks = response
        .get_mut("recordMap")
        .and_then(Value::as_object_mut)
        .and_then(|record_map| record_map.get_mut("block"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "missing recordMap.block in Notion page response".to_string())?;
    for (block_id, outcome) in outcomes {
        apply_hydrated_outcome(
            existing_blocks,
            &block_id,
            authoritative_blocks.get(&block_id),
            outcome,
        )?;
    }
    Ok(())
}

fn apply_hydrated_outcome(
    existing_blocks: &mut Map<String, Value>,
    block_id: &str,
    authoritative: Option<&Value>,
    outcome: AuthoritativePageBlockState,
) -> Result<(), String> {
    match outcome {
        AuthoritativePageBlockState::Unavailable(_) => {
            existing_blocks.remove(block_id);
        }
        AuthoritativePageBlockState::Complete(omitted) => {
            let authoritative = authoritative
                .ok_or_else(|| format!("missing complete authoritative Notion block {block_id}"))?;
            let Some(existing) = existing_blocks.get_mut(block_id) else {
                return Ok(());
            };
            apply_authoritative_page_block_omissions(existing, authoritative, omitted, block_id)?;
        }
    }
    Ok(())
}
