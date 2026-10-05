use super::super::{
    block_value, optional_record_map_table, page_state::PageMutationState, required_u64,
    NotionPrivateApiEndpoint,
};
use super::wire::SaveOperation;
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};

const PAGE_MUTATION_COMMIT_TIMEOUT: Duration = Duration::from_secs(3);
const PAGE_MUTATION_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub(super) struct BlockCommitExpectation {
    pub(super) expected_version: u64,
    pub(super) overlay_record: bool,
}

pub(super) fn record_value_mut(entry: &mut Value) -> Option<&mut Map<String, Value>> {
    let value = entry.get_mut("value")?;
    let record = if value.get("value").is_some() {
        value.get_mut("value")?
    } else {
        value
    };
    record.as_object_mut()
}

pub(super) fn block_commit_expectations(
    state: &PageMutationState,
    operations: &[SaveOperation],
) -> HashMap<String, BlockCommitExpectation> {
    let mut commit_block_ids = Vec::new();
    for operation in operations {
        operation.append_commit_block_ids(&mut commit_block_ids);
    }
    let mut expectations = HashMap::new();
    for block_id in commit_block_ids {
        let baseline = state
            .blocks
            .get(&block_id)
            .map(|block| block.version)
            .unwrap_or(0);
        expectations
            .entry(block_id)
            .or_insert(BlockCommitExpectation {
                expected_version: baseline
                    .checked_add(1)
                    .expect("Notion block versions must not overflow"),
                overlay_record: false,
            });
    }
    let mut overlay_block_ids = Vec::new();
    for operation in operations {
        operation.append_record_overlay_block_ids(&mut overlay_block_ids);
    }
    for block_id in overlay_block_ids {
        if let Some(expectation) = expectations.get_mut(&block_id) {
            expectation.overlay_record = true;
        }
    }
    expectations
}

pub(super) fn wait_for_committed_block_records(
    session: &NotionDesktopSession,
    space_id: &str,
    expectations: &HashMap<String, BlockCommitExpectation>,
) -> Result<Map<String, Value>, NotionLiveError> {
    let deadline = Instant::now() + PAGE_MUTATION_COMMIT_TIMEOUT;
    let mut pending = expectations.keys().cloned().collect::<HashSet<_>>();
    let mut committed = Map::new();
    while !pending.is_empty() {
        let response = sync_record_values(session, space_id, expectations, &pending)?;
        collect_committed_records(&response, expectations, &mut pending, &mut committed)?;
        if pending.is_empty() {
            return Ok(committed);
        }
        if Instant::now() >= deadline {
            return Err(NotionLiveError::Fatal(commit_timeout_error(
                &pending,
                expectations,
            )));
        }
        thread::sleep(PAGE_MUTATION_POLL_INTERVAL);
    }
    Ok(committed)
}

fn sync_record_values(
    session: &NotionDesktopSession,
    space_id: &str,
    expectations: &HashMap<String, BlockCommitExpectation>,
    pending: &HashSet<String>,
) -> Result<Value, NotionLiveError> {
    let requests = pending
        .iter()
        .map(|block_id| {
            let expected_version = expectations
                .get(block_id)
                .expect("pending commit block must have a version expectation")
                .expected_version;
            json!({
                "pointer": { "table": "block", "id": block_id, "spaceId": space_id },
                "version": expected_version - 1,
            })
        })
        .collect::<Vec<_>>();
    post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesSpace,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )
}

fn collect_committed_records(
    response: &Value,
    expectations: &HashMap<String, BlockCommitExpectation>,
    pending: &mut HashSet<String>,
    committed: &mut Map<String, Value>,
) -> Result<(), String> {
    let Some(blocks) = optional_record_map_table(response, "block") else {
        return Ok(());
    };
    for block_id in pending.clone() {
        let Some(entry) = blocks.get(&block_id) else {
            continue;
        };
        let version = required_u64(block_value(blocks, &block_id)?, "version")?;
        let expected_version = expectations
            .get(&block_id)
            .expect("returned commit block must have a version expectation")
            .expected_version;
        if version >= expected_version {
            committed.insert(block_id.clone(), entry.clone());
            pending.remove(&block_id);
        }
    }
    Ok(())
}

fn commit_timeout_error(
    pending: &HashSet<String>,
    expectations: &HashMap<String, BlockCommitExpectation>,
) -> String {
    let mut missing = pending
        .iter()
        .map(|block_id| {
            format!(
                "{block_id}@{}",
                expectations
                    .get(block_id)
                    .expect("pending commit block must have a version expectation")
                    .expected_version
            )
        })
        .collect::<Vec<_>>();
    missing.sort();
    format!(
        "Notion did not expose committed block versions before the mutation deadline: {}",
        missing.join(", ")
    )
}
