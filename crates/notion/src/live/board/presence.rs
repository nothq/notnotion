use std::collections::HashSet;

use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    record_map_table, required_string, unwrap_record_value, NotionPrivateApiEndpoint,
    PagePresenceSnapshot, UserContext,
};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};
use crate::model::PagePresenceProfile;

const MAX_VISIBLE_OTHER_USERS: usize = 4;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageVisitorsResponse {
    page_visits: Vec<PageVisit>,
    total_count: u32,
}

#[derive(Deserialize)]
struct PageVisit {
    user_id: String,
    visited_at: u64,
    #[serde(default)]
    is_hidden: bool,
}

pub(super) fn load_page_presence(
    session: &NotionDesktopSession,
    block_id: &str,
    space_id: &str,
    user_context: &UserContext,
) -> Result<PagePresenceSnapshot, NotionLiveError> {
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::GetPageVisitors,
        &json!({
            "block": { "id": block_id, "spaceId": space_id },
            "limit": 32,
        }),
    )?;
    let visitors = serde_json::from_value::<PageVisitorsResponse>(response)
        .map_err(|error| format!("invalid getPageVisitors response: {error}"))?;
    let visitor_ids = ordered_visitor_ids(visitors.page_visits, &user_context.profile.user_id);
    let visitor_profiles = load_visitor_profiles(session, space_id, &visitor_ids)?;
    let mut profiles = Vec::with_capacity(MAX_VISIBLE_OTHER_USERS + 1);
    profiles.push(user_context.profile.clone());
    profiles.extend(visitor_profiles.into_iter().take(MAX_VISIBLE_OTHER_USERS));
    let visible_other_count = profiles.len().saturating_sub(1) as u32;
    let adjusted_total = visitors.total_count.saturating_sub(1);
    Ok(PagePresenceSnapshot {
        profiles,
        overflow_count: adjusted_total.saturating_sub(visible_other_count),
    })
}

pub(super) fn page_presence_profile(value: &Value) -> Result<PagePresenceProfile, String> {
    Ok(PagePresenceProfile {
        user_id: required_string(value, "id")?.to_string(),
        name: required_string(value, "name")?.to_string(),
        profile_photo: value
            .get("profile_photo")
            .and_then(Value::as_str)
            .map(str::to_string),
        avatar: None,
    })
}

fn ordered_visitor_ids(visits: Vec<PageVisit>, current_user_id: &str) -> Vec<String> {
    let mut visits = visits
        .into_iter()
        .filter(|visit| !visit.is_hidden && visit.user_id != current_user_id)
        .collect::<Vec<_>>();
    visits.sort_by_key(|visit| std::cmp::Reverse(visit.visited_at));
    let mut seen = HashSet::new();
    visits
        .into_iter()
        .filter_map(|visit| seen.insert(visit.user_id.clone()).then_some(visit.user_id))
        .collect()
}

fn load_visitor_profiles(
    session: &NotionDesktopSession,
    space_id: &str,
    visitor_ids: &[String],
) -> Result<Vec<PagePresenceProfile>, NotionLiveError> {
    if visitor_ids.is_empty() {
        return Ok(Vec::new());
    }
    let requests = visitor_ids
        .iter()
        .map(|user_id| {
            json!({
                "pointer": { "table": "notion_user", "id": user_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": space_id },
        }),
    )?;
    let users = record_map_table(&response, "notion_user")?;
    Ok(visitor_ids
        .iter()
        .filter_map(|user_id| users.get(user_id).and_then(unwrap_record_value))
        .filter_map(|user| page_presence_profile(user).ok())
        .collect())
}
