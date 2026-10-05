use std::collections::HashSet;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{
    block_value, combined_block_records, load_missing_sidebar_blocks, page_shell_icon,
    record_map_table, required_string, title_property_allow_empty, unwrap_record_value,
    DatabaseFilterRelationPage, LiveWorkspaceContext, LoadDatabaseFilterUsersResult,
    NotionFilterPageId, NotionPrivateApiEndpoint, SearchDatabaseFilterRelationPagesRequest,
    SearchDatabaseFilterRelationPagesResult,
};
use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_in_space_with_session, post_private_api_with_session},
    NotionLiveError,
};
use crate::model::{NotionUserId, NotionWorkspaceUser};

const DATABASE_FILTER_RELATION_SEARCH_LIMIT: u32 = 20;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisibleUsersResponse {
    #[serde(alias = "users")]
    visible_users: Vec<VisibleUserWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisibleUserWire {
    user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelationSearchResponse {
    results: Vec<RelationSearchResultWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelationSearchResultWire {
    id: String,
}

pub(crate) fn load_database_filter_users(
    session: &NotionDesktopSession,
    context: &LiveWorkspaceContext,
) -> Result<LoadDatabaseFilterUsersResult, NotionLiveError> {
    Ok(LoadDatabaseFilterUsersResult {
        users: load_visible_workspace_users(session, context)?,
    })
}

pub(crate) fn load_visible_workspace_users(
    session: &NotionDesktopSession,
    context: &LiveWorkspaceContext,
) -> Result<Vec<NotionWorkspaceUser>, NotionLiveError> {
    let cache = context.workspace_cache()?;
    let visible_response = post_private_api_in_space_with_session(
        session,
        NotionPrivateApiEndpoint::GetVisibleUsers,
        &cache.space_id,
        &json!({
            "spaceId": cache.space_id.as_str(),
            "supportsEdgeCache": true,
            "earlyReturnForEdgeCache": true,
        }),
    )?;
    let visible_response = serde_json::from_value::<VisibleUsersResponse>(visible_response)
        .map_err(|error| format!("invalid Notion getVisibleUsers response: {error}"))?;
    let user_ids = parse_visible_user_ids(visible_response.visible_users)?;
    let requests = user_ids
        .iter()
        .map(|user_id| {
            json!({
                "pointer": { "table": "notion_user", "id": user_id.as_str() },
                "version": -1,
            })
        })
        .collect::<Vec<_>>();
    let hydrated_response = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::SyncRecordValuesMain,
        &json!({
            "requests": requests,
            "spacePointer": { "table": "space", "id": cache.space_id.as_str() },
        }),
    )?;
    let hydrated_users = record_map_table(&hydrated_response, "notion_user")?;
    let users = user_ids
        .into_iter()
        .map(|user_id| workspace_user(user_id, hydrated_users, cache.user_context.user_id.as_str()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(users)
}

pub(crate) fn search_database_filter_relation_pages(
    session: &NotionDesktopSession,
    context: &LiveWorkspaceContext,
    request: SearchDatabaseFilterRelationPagesRequest,
) -> Result<SearchDatabaseFilterRelationPagesResult, NotionLiveError> {
    let cache = context.workspace_cache()?;
    let (collection_id, query, selected_page_ids) = request.into_parts();
    let response: Value = post_private_api_with_session(
        session,
        NotionPrivateApiEndpoint::Search,
        &json!({
            "type": "BlocksInCollection",
            "query": query.as_str(),
            "spaceId": cache.space_id.as_str(),
            "collectionId": collection_id.as_str(),
            "limit": DATABASE_FILTER_RELATION_SEARCH_LIMIT,
            "filters": relation_search_filters(),
            "sort": { "field": "relevance" },
            "source": "relation_menu",
        }),
    )?;
    let decoded = RelationSearchResponse::deserialize(&response)
        .map_err(|error| format!("invalid Notion relation search response: {error}"))?;
    let empty_blocks = Map::new();
    let response_blocks = match response
        .get("recordMap")
        .and_then(|record_map| record_map.get("block"))
        .and_then(Value::as_object)
    {
        Some(blocks) => blocks,
        None => &empty_blocks,
    };
    let (results, result_ids) = relation_search_results(decoded.results, selected_page_ids);
    let hydrated_blocks =
        load_missing_sidebar_blocks(session, &cache.space_id, &result_ids, response_blocks)?;
    let blocks = combined_block_records([response_blocks, &hydrated_blocks])?;
    let mut page_ids = HashSet::with_capacity(results.len());
    let pages = results
        .into_iter()
        .map(|result| {
            database_filter_relation_page(
                result,
                &blocks,
                &cache.space_id,
                &collection_id,
                &mut page_ids,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SearchDatabaseFilterRelationPagesResult { pages })
}

fn relation_search_results(
    mut results: Vec<RelationSearchResultWire>,
    selected_page_ids: Vec<NotionFilterPageId>,
) -> (Vec<RelationSearchResultWire>, Vec<String>) {
    let mut result_ids = results
        .iter()
        .map(|result| result.id.clone())
        .collect::<Vec<_>>();
    let mut result_id_set = result_ids.iter().cloned().collect::<HashSet<_>>();
    for page_id in selected_page_ids {
        let page_id = page_id.as_str();
        if result_id_set.insert(page_id.to_string()) {
            result_ids.push(page_id.to_string());
            results.push(RelationSearchResultWire {
                id: page_id.to_string(),
            });
        }
    }
    (results, result_ids)
}

fn parse_visible_user_ids(users: Vec<VisibleUserWire>) -> Result<Vec<NotionUserId>, String> {
    if users.is_empty() {
        return Err("Notion getVisibleUsers returned no visible users".to_string());
    }
    let mut seen = HashSet::with_capacity(users.len());
    users
        .into_iter()
        .map(|user| {
            let user_id = NotionUserId::try_from(user.user_id).map_err(str::to_string)?;
            if !seen.insert(user_id.clone()) {
                return Err(format!(
                    "Notion getVisibleUsers returned duplicate user {}",
                    user_id.as_str()
                ));
            }
            Ok(user_id)
        })
        .collect()
}

fn workspace_user(
    user_id: NotionUserId,
    hydrated_users: &Map<String, Value>,
    current_user_id: &str,
) -> Result<NotionWorkspaceUser, String> {
    let user = hydrated_users
        .get(user_id.as_str())
        .and_then(unwrap_record_value)
        .ok_or_else(|| {
            format!(
                "Notion user hydration omitted visible user {}",
                user_id.as_str()
            )
        })?;
    let hydrated_user_id = required_string(user, "id")?;
    if hydrated_user_id != user_id.as_str() {
        return Err(format!(
            "Notion user hydration returned user {hydrated_user_id} for requested user {}",
            user_id.as_str()
        ));
    }
    let name = required_string(user, "name")?;
    if name.trim().is_empty() {
        return Err(format!(
            "Notion visible user {} has an empty name",
            user_id.as_str()
        ));
    }
    Ok(NotionWorkspaceUser {
        is_current_user: user_id.as_str() == current_user_id,
        user_id,
        name: name.to_string(),
        profile_photo: optional_profile_photo(user)?,
    })
}

fn optional_profile_photo(user: &Value) -> Result<Option<String>, String> {
    match user.get("profile_photo") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(profile_photo)) => Ok(Some(profile_photo.clone())),
        Some(_) => Err("Notion user profile_photo must be a string or null".to_string()),
    }
}

fn relation_search_filters() -> Value {
    json!({
        "isDeletedOnly": false,
        "excludeTemplates": true,
        "navigableBlockContentOnly": true,
        "requireEditPermissions": false,
        "includePublicPagesWithoutExplicitAccess": true,
        "ancestors": [],
        "createdBy": [],
        "editedBy": [],
        "lastEditedTime": {},
        "createdTime": {},
        "inTeams": [],
        "excludeSurrogateCollections": false,
        "excludedParentCollectionIds": [],
    })
}

fn database_filter_relation_page(
    result: RelationSearchResultWire,
    blocks: &Map<String, Value>,
    expected_space_id: &str,
    expected_collection_id: &str,
    page_ids: &mut HashSet<NotionFilterPageId>,
) -> Result<DatabaseFilterRelationPage, String> {
    let page_id = NotionFilterPageId::try_from(result.id.clone()).map_err(str::to_string)?;
    if !page_ids.insert(page_id.clone()) {
        return Err(format!(
            "Notion relation search returned duplicate page {}",
            page_id.as_str()
        ));
    }
    let block = block_value(blocks, page_id.as_str())?;
    if required_string(block, "space_id")? != expected_space_id {
        return Err(format!(
            "Notion relation page {} belongs to another space",
            page_id.as_str()
        ));
    }
    if required_string(block, "parent_table")? != "collection"
        || required_string(block, "parent_id")? != expected_collection_id
    {
        return Err(format!(
            "Notion relation page {} does not belong to collection {expected_collection_id}",
            page_id.as_str()
        ));
    }
    let title = title_property_allow_empty(block)?.ok_or_else(|| {
        format!(
            "Notion relation page {} is missing its title property",
            page_id.as_str()
        )
    })?;
    Ok(DatabaseFilterRelationPage {
        page_id,
        title,
        icon: page_shell_icon(block, "page"),
    })
}
