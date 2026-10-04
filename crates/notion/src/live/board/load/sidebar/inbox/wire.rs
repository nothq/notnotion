use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationLogRequest<'a> {
    pub(super) space_id: &'a str,
    pub(super) size: u32,
    #[serde(rename = "type")]
    pub(super) notification_type: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NotificationLogResponse {
    pub(super) notification_ids: Vec<String>,
    pub(super) record_map: Map<String, Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActivityLogRequest<'a> {
    pub(super) space_id: &'a str,
    pub(super) limit: u32,
    pub(super) activity_types: Vec<&'static str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActivityLogResponse {
    pub(super) activity_ids: Vec<String>,
    pub(super) record_map: Map<String, Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UnvisitedNotificationIdsRequest<'a> {
    pub(super) space_id: &'a str,
    pub(super) timestamp: u64,
    #[serde(rename = "type")]
    pub(super) notification_type: &'static str,
    pub(super) size: u32,
    pub(super) read_filter: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UnvisitedNotificationIdsResponse {
    pub(super) notification_ids: Vec<String>,
}
