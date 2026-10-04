use std::collections::HashSet;

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;

use super::super::super::{
    optional_string_array, required_string, Map, NotionPrivateApiEndpoint, Value,
};
use super::records::{is_resolved_sidebar_block, record_value, required_bool};
use super::tree::{CollectionPointer, ParsedSidebarNode, ParsedSidebarSection};
use crate::live::{
    credentials::NotionDesktopSession, http::post_private_api_with_session, NotionLiveError,
};

pub(super) struct TeamspaceData {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) is_default: bool,
    pub(super) page_ids: Vec<String>,
}

pub(super) struct MeetingsData {
    pub(super) sidebar_section_id: String,
    pub(super) page_ids: Vec<String>,
    pub(super) blocks: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedPagesResponse {
    record_map: SharedPagesRecordMap,
}

#[derive(Deserialize)]
struct SharedPagesRecordMap {
    block: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamsResponse {
    teams: Vec<TeamSummary>,
    record_map: TeamsRecordMap,
}

#[derive(Deserialize)]
struct TeamSummary {
    id: String,
}

#[derive(Deserialize)]
struct TeamsRecordMap {
    team: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidebarSectionsResponse {
    in_gate: bool,
    sidebar_sections: Option<Vec<SidebarSectionDescriptor>>,
    record_map: Option<SidebarSectionsRecordMap>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SidebarSectionDescriptor {
    id: String,
    truncated_block_ids: Vec<String>,
    #[serde(rename = "hasMore")]
    _has_more: bool,
}

#[derive(Deserialize)]
struct SidebarSectionsRecordMap {
    sidebar_section: Map<String, Value>,
    block: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncedBlockResponse {
    record_map: SyncedBlockRecordMap,
}

#[derive(Deserialize)]
struct SyncedBlockRecordMap {
    #[serde(default)]
    block: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncedCollectionResponse {
    record_map: SyncedCollectionRecordMap,
}

#[derive(Deserialize)]
struct SyncedCollectionRecordMap {
    collection: Map<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncedAttributionResponse {
    record_map: SidebarAttributionRecords,
}

#[derive(Default, Deserialize)]
pub(super) struct SidebarAttributionRecords {
    #[serde(default, rename = "notion_user")]
    pub(super) users: Map<String, Value>,
    #[serde(default, rename = "bot")]
    pub(super) bots: Map<String, Value>,
}

pub(super) fn load_shared_page_blocks(
    session: &NotionDesktopSession,
    space_id: &str,
) -> Result<Map<String, Value>, NotionLiveError> {
    let response = post_typed::<SharedPagesResponse>(
        session,
        NotionPrivateApiEndpoint::GetUserSharedPagesInSpace,
        &json!({ "spaceId": space_id }),
    )?;
    Ok(response.record_map.block)
}

pub(super) fn load_teamspaces(
    session: &NotionDesktopSession,
    space_id: &str,
    user_id: &str,
    joined_team_ids: &[String],
) -> Result<Vec<TeamspaceData>, NotionLiveError> {
    let response = post_typed::<TeamsResponse>(
        session,
        NotionPrivateApiEndpoint::GetTeamsV2,
        &json!({
            "spaceId": space_id,
            "teamTypes": ["Joined"],
            "includeMembershipSummary": false,
            "targetUserId": user_id,
        }),
    )?;
    joined_team_ids
        .iter()
        .map(|team_id| parse_teamspace(&response, team_id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(NotionLiveError::Fatal)
}

fn parse_teamspace(response: &TeamsResponse, team_id: &str) -> Result<TeamspaceData, String> {
    if !response.teams.iter().any(|team| team.id == team_id) {
        return Err(format!("missing joined Notion team summary {team_id}"));
    }
    let team = record_value(&response.record_map.team, team_id, "team")?;
    Ok(TeamspaceData {
        id: team_id.to_string(),
        name: required_string(team, "name")?.to_string(),
        is_default: required_bool(team, "is_default")?,
        page_ids: optional_string_array(team, "team_pages")?,
    })
}

pub(in crate::live::board::load) fn load_missing_sidebar_blocks(
    session: &NotionDesktopSession,
    space_id: &str,
    page_ids: &[String],
    available_blocks: &Map<String, Value>,
) -> Result<Map<String, Value>, NotionLiveError> {
    let mut seen_page_ids = HashSet::new();
    let missing_page_ids = page_ids
        .iter()
        .filter(|block_id| seen_page_ids.insert(block_id.as_str()))
        .filter(|block_id| !is_resolved_sidebar_block(available_blocks, block_id))
        .collect::<Vec<_>>();
    if missing_page_ids.is_empty() {
        return Ok(Map::new());
    }
    let requests = missing_page_ids
        .iter()
        .map(|block_id| {
            json!({
                "pointer": { "table": "block", "id": block_id, "spaceId": space_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let response = post_typed::<SyncedBlockResponse>(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({ "requests": requests }),
    )?;
    Ok(response.record_map.block)
}

pub(super) fn load_sidebar_attribution_records(
    session: &NotionDesktopSession,
    space_id: &str,
    pointers: &[(String, String)],
) -> Result<SidebarAttributionRecords, NotionLiveError> {
    load_sidebar_attribution_records_with(space_id, pointers, |body| {
        post_typed::<SyncedAttributionResponse>(
            session,
            NotionPrivateApiEndpoint::SyncRecordValuesMain,
            body,
        )
    })
}

fn load_sidebar_attribution_records_with(
    space_id: &str,
    pointers: &[(String, String)],
    load: impl FnOnce(&Value) -> Result<SyncedAttributionResponse, NotionLiveError>,
) -> Result<SidebarAttributionRecords, NotionLiveError> {
    if pointers.is_empty() {
        return Ok(SidebarAttributionRecords::default());
    }
    let requests = pointers
        .iter()
        .map(|(table, id)| {
            json!({
                "pointer": { "table": table, "id": id, "spaceId": space_id },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    match load(&json!({ "requests": requests })) {
        Ok(response) => Ok(response.record_map),
        Err(NotionLiveError::Session(error)) => Err(NotionLiveError::Session(error)),
        Err(error) => {
            println!("notnotion Quick Find omitted optional attribution metadata: {error}");
            Ok(SidebarAttributionRecords::default())
        }
    }
}

pub(super) fn load_meetings(
    session: &NotionDesktopSession,
    space_id: &str,
) -> Result<Option<MeetingsData>, NotionLiveError> {
    let response = post_typed::<SidebarSectionsResponse>(
        session,
        NotionPrivateApiEndpoint::GetSidebarSections,
        &json!({ "spaceId": space_id }),
    )?;
    if !response.in_gate {
        return Ok(None);
    }
    let sidebar_sections = response
        .sidebar_sections
        .ok_or_else(|| "missing gated Notion sidebar sections".to_string())?;
    let record_map = response
        .record_map
        .ok_or_else(|| "missing gated Notion sidebar section records".to_string())?;
    let mut meetings = None;
    for descriptor in sidebar_sections {
        let section = record_value(
            &record_map.sidebar_section,
            &descriptor.id,
            "sidebar_section",
        )?;
        if required_string(section, "section_type")? != "myMeetings" {
            continue;
        }
        if meetings
            .replace((descriptor.id, descriptor.truncated_block_ids))
            .is_some()
        {
            return Err(NotionLiveError::Fatal(
                "duplicate Notion myMeetings sidebar section".to_string(),
            ));
        }
    }
    let Some((sidebar_section_id, page_ids)) = meetings else {
        return Ok(None);
    };
    Ok(Some(MeetingsData {
        sidebar_section_id,
        page_ids,
        blocks: record_map.block,
    }))
}

pub(super) fn load_sidebar_collections(
    session: &NotionDesktopSession,
    sections: &[ParsedSidebarSection],
) -> Result<Map<String, Value>, NotionLiveError> {
    let mut pointers = Vec::new();
    for section in sections {
        section.collect_collection_pointers(&mut pointers);
    }
    load_collection_pointers(session, pointers)
}

pub(super) fn load_sidebar_node_collections(
    session: &NotionDesktopSession,
    node: &ParsedSidebarNode,
) -> Result<Map<String, Value>, NotionLiveError> {
    let mut pointers = Vec::new();
    node.collect_child_collection_pointers(&mut pointers);
    load_collection_pointers(session, pointers)
}

pub(super) fn load_sidebar_collections_by_ids(
    session: &NotionDesktopSession,
    space_id: &str,
    collection_ids: &[String],
) -> Result<Map<String, Value>, NotionLiveError> {
    let mut seen_collection_ids = HashSet::new();
    let requests = collection_ids
        .iter()
        .filter(|collection_id| seen_collection_ids.insert(collection_id.as_str()))
        .map(|collection_id| {
            json!({
                "pointer": {
                    "id": collection_id,
                    "table": "collection",
                    "spaceId": space_id,
                },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    load_collection_requests(session, requests)
}

fn load_collection_pointers(
    session: &NotionDesktopSession,
    mut pointers: Vec<&CollectionPointer>,
) -> Result<Map<String, Value>, NotionLiveError> {
    let mut seen_collection_ids = HashSet::new();
    pointers.retain(|pointer| seen_collection_ids.insert(pointer.id().to_string()));
    let requests = pointers
        .iter()
        .map(|pointer| json!({ "pointer": pointer, "version": -1 }))
        .collect::<Vec<_>>();
    load_collection_requests(session, requests)
}

fn load_collection_requests(
    session: &NotionDesktopSession,
    requests: Vec<Value>,
) -> Result<Map<String, Value>, NotionLiveError> {
    if requests.is_empty() {
        return Ok(Map::new());
    }
    let response = post_typed::<SyncedCollectionResponse>(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({ "requests": requests }),
    )?;
    Ok(response.record_map.collection)
}

fn post_typed<T: DeserializeOwned>(
    session: &NotionDesktopSession,
    endpoint: NotionPrivateApiEndpoint,
    body: &Value,
) -> Result<T, NotionLiveError> {
    let response = post_private_api_with_session(session, endpoint, body)?;
    serde_json::from_value(response).map_err(|error| {
        NotionLiveError::Fatal(format!(
            "invalid Notion private API {endpoint:?} response: {error}"
        ))
    })
}

#[cfg(test)]
mod tests;
