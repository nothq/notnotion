use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

mod merge;
mod preview;
mod state;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod transport;

const QUICK_FIND_RECORD_LIMIT: usize = 4_096;
const QUICK_FIND_PREVIEW_REQUEST_LIMIT: usize = 512;
const QUICK_FIND_PREVIEW_MAX_WAVES: usize = 8;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct QuickFindRecordScope {
    active_user_id: String,
    space_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordKey {
    table: String,
    id: String,
}

#[derive(Clone, Copy)]
enum QuickFindUserRecordState {
    RoleOnly,
    Value { version: Option<u64> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HydratedPreviewRoot {
    version: Option<u64>,
}

type SharedRecordMap = Arc<Map<String, Value>>;

#[derive(Debug, Default)]
struct QuickFindRecordState {
    record_map: SharedRecordMap,
    recency: VecDeque<RecordKey>,
    hydrated_preview_roots: HashMap<String, HydratedPreviewRoot>,
    preview_root_generations: HashMap<String, u64>,
}

#[derive(Debug)]
pub(super) struct QuickFindRecords {
    space_id: String,
    state: Mutex<QuickFindRecordState>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitialSyncRequest {
    requests: Vec<InitialSyncRecordRequest>,
    space_pointer: InitialSyncSpacePointer,
}

#[derive(Clone, Debug, Serialize)]
struct InitialSyncRecordRequest {
    pointer: InitialSyncRecordPointer,
    version: i64,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitialSyncRecordPointer {
    table: String,
    id: String,
    space_id: String,
}

type PreviewRecordKey = (String, String, String);

#[derive(Debug)]
struct PreviewRecordClosure {
    pointers: Vec<InitialSyncRecordPointer>,
    seen: HashSet<PreviewRecordKey>,
}

#[derive(Debug, Serialize)]
struct InitialSyncSpacePointer {
    table: &'static str,
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitialSyncResponse {
    record_map: Map<String, Value>,
}
