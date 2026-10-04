use std::sync::OnceLock;

use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use super::credentials::NotionDesktopSession;
use super::{NotionLiveError, NotionResourceFailure, NotionSessionFailure};

const NOTION_PRIVATE_API_BASE_URL: &str = "https://app.notion.com/api/v3";
const NOTION_WEB_ORIGIN: &str = "https://app.notion.com";
const NOTION_WEB_REFERER: &str = "https://app.notion.com/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionPrivateApiEndpoint {
    CreateCustomEmoji,
    GetActivityLog,
    GetCalendarEvents,
    GetCustomEmojisForSpace,
    GetInferenceTranscriptsForUser,
    GetNotificationLog,
    GetUnvisitedNotificationIds,
    GetPageVisitors,
    GetRecentPageVisits,
    GetSidebarSections,
    GetSpacesInitial,
    GetTeamsV2,
    GetUploadGenericFileUrl,
    GetUploadSpaceFileUrl,
    GetUserSharedPagesInSpace,
    GetUserHomePages,
    GetVisibleUsers,
    LoadCachedPageChunkV2,
    QueryCollectionInitialLoad,
    QueryCollectionInitialLoadWithAggregations,
    SaveTransactionsFanout,
    SaveTransactionsMain,
    Search,
    SyncRecordValuesMain,
    SyncRecordValuesSpace,
    SyncRecordValuesSpaceInitial,
}

impl NotionPrivateApiEndpoint {
    const fn path(self) -> &'static str {
        match self {
            Self::CreateCustomEmoji => "createCustomEmoji",
            Self::GetActivityLog => "getActivityLog",
            Self::GetCalendarEvents => "getCalendarEvents",
            Self::GetCustomEmojisForSpace => "getCustomEmojisForSpace",
            Self::GetInferenceTranscriptsForUser => "getInferenceTranscriptsForUser",
            Self::GetNotificationLog => "getNotificationLog",
            Self::GetUnvisitedNotificationIds => "getUnvisitedNotificationIds",
            Self::GetPageVisitors => "getPageVisitors",
            Self::GetRecentPageVisits => "getRecentPageVisits",
            Self::GetSidebarSections => "getSidebarSections",
            Self::GetSpacesInitial => "getSpacesInitial",
            Self::GetTeamsV2 => "getTeamsV2",
            Self::GetUploadGenericFileUrl => "getUploadGenericFileUrl",
            Self::GetUploadSpaceFileUrl => "getUploadSpaceFileUrl",
            Self::GetUserSharedPagesInSpace => "getUserSharedPagesInSpace",
            Self::GetUserHomePages => "getUserHomePages",
            Self::GetVisibleUsers => "getVisibleUsers",
            Self::LoadCachedPageChunkV2 => "loadCachedPageChunkV2",
            Self::QueryCollectionInitialLoad => "queryCollection?src=initial_load",
            Self::QueryCollectionInitialLoadWithAggregations => {
                "queryCollection?src=initial_load_with_aggregations"
            }
            Self::SaveTransactionsFanout => "saveTransactionsFanout",
            Self::SaveTransactionsMain => "saveTransactionsMain",
            Self::Search => "search",
            Self::SyncRecordValuesMain => "syncRecordValuesMain",
            Self::SyncRecordValuesSpace => "syncRecordValuesSpace",
            Self::SyncRecordValuesSpaceInitial => "syncRecordValuesSpaceInitial",
        }
    }

    const fn requires_space_header(self) -> bool {
        matches!(
            self,
            Self::CreateCustomEmoji
                | Self::GetCustomEmojisForSpace
                | Self::GetRecentPageVisits
                | Self::GetUploadGenericFileUrl
                | Self::GetUploadSpaceFileUrl
                | Self::GetUserHomePages
                | Self::GetVisibleUsers
                | Self::SaveTransactionsFanout
                | Self::SaveTransactionsMain
                | Self::SyncRecordValuesSpaceInitial
        )
    }
}

struct NotionSession {
    client: Client,
    credentials: NotionDesktopSession,
}

struct RawNotionResponse {
    status: u16,
    body: String,
}

pub(crate) struct ActiveUserNotionResponse {
    active_user_id: String,
    body: Value,
}

impl ActiveUserNotionResponse {
    pub(crate) fn into_parts(self) -> (String, Value) {
        (self.active_user_id, self.body)
    }
}

impl NotionSession {
    fn new(credentials: NotionDesktopSession) -> Result<Self, NotionLiveError> {
        let client = notion_http_client().map_err(NotionLiveError::Fatal)?;
        Ok(Self {
            client,
            credentials,
        })
    }

    fn post(
        &self,
        endpoint: NotionPrivateApiEndpoint,
        space_id: Option<&str>,
        body: &(impl Serialize + ?Sized),
    ) -> Result<RawNotionResponse, NotionLiveError> {
        let mut request = self
            .client
            .post(format!("{NOTION_PRIVATE_API_BASE_URL}/{}", endpoint.path()))
            .header("content-type", "application/json")
            .header("origin", NOTION_WEB_ORIGIN)
            .header("referer", NOTION_WEB_REFERER)
            .header("x-notion-active-user-header", self.credentials.user_id())
            .header("cookie", self.credentials.cookie_header());
        if endpoint.requires_space_header() {
            let space_id = space_id.ok_or_else(|| {
                NotionLiveError::Fatal(format!(
                    "Notion private API {} requires a workspace space ID",
                    endpoint.path()
                ))
            })?;
            request = request.header("x-notion-space-id", space_id);
        }
        let response = request.json(body).send().map_err(|error| {
            NotionLiveError::Fatal(format!(
                "Notion private API request to {} failed: {error}",
                endpoint.path()
            ))
        })?;
        let status = response.status().as_u16();
        let body = response.text().map_err(|error| {
            NotionLiveError::Fatal(format!(
                "failed to read Notion private API response: {error}"
            ))
        })?;
        Ok(RawNotionResponse { status, body })
    }
}

fn notion_http_client() -> Result<Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .build()
                .map_err(|error| format!("failed to build reqwest client: {error}"))
        })
        .clone()
}

impl RawNotionResponse {
    fn into_json<Response: DeserializeOwned>(
        self,
        endpoint: NotionPrivateApiEndpoint,
    ) -> Result<Response, NotionLiveError> {
        if let Some(failure) = notion_resource_failure(endpoint, self.status, &self.body) {
            return Err(NotionLiveError::Unavailable(failure));
        }
        if matches!(self.status, 401 | 403) {
            return Err(NotionLiveError::Session(NotionSessionFailure::Rejected {
                status: self.status,
            }));
        }
        if !(200..300).contains(&self.status) {
            return Err(NotionLiveError::Fatal(response_failure_message(
                endpoint,
                self.status,
                &self.body,
            )));
        }
        serde_json::from_str(&self.body).map_err(|error| {
            NotionLiveError::Fatal(format!(
                "failed to decode Notion private API {} response as JSON: {error}",
                endpoint.path()
            ))
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NotionApiErrorCode {
    ObjectNotFound,
    RestrictedResource,
    #[default]
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct NotionApiErrorEnvelope {
    #[serde(default)]
    code: NotionApiErrorCode,
    #[serde(default)]
    message: Option<String>,
}

fn notion_resource_failure(
    endpoint: NotionPrivateApiEndpoint,
    status: u16,
    body: &str,
) -> Option<NotionResourceFailure> {
    if endpoint != NotionPrivateApiEndpoint::LoadCachedPageChunkV2 {
        return None;
    }
    let error = serde_json::from_str::<NotionApiErrorEnvelope>(body).ok()?;
    let diagnostic = response_failure_message(endpoint, status, body);
    match error.code {
        NotionApiErrorCode::ObjectNotFound => Some(NotionResourceFailure::NotFound { diagnostic }),
        NotionApiErrorCode::RestrictedResource => {
            Some(NotionResourceFailure::Restricted { diagnostic })
        }
        NotionApiErrorCode::Other => None,
    }
}

fn response_failure_message(endpoint: NotionPrivateApiEndpoint, status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<NotionApiErrorEnvelope>(body)
        .ok()
        .and_then(|error| error.message);
    detail.map_or_else(
        || {
            format!(
                "Notion private API {} returned HTTP {status}",
                endpoint.path()
            )
        },
        |detail| {
            format!(
                "Notion private API {} returned HTTP {status}: {detail}",
                endpoint.path()
            )
        },
    )
}

pub(crate) fn post_private_api_with_session<Request, Response>(
    credentials: &NotionDesktopSession,
    endpoint: NotionPrivateApiEndpoint,
    body: &Request,
) -> Result<Response, NotionLiveError>
where
    Request: Serialize + ?Sized,
    Response: DeserializeOwned,
{
    post_private_api_with_session_and_space(credentials, endpoint, None, body)
}

pub(crate) fn post_private_api_in_space_with_session<Request, Response>(
    credentials: &NotionDesktopSession,
    endpoint: NotionPrivateApiEndpoint,
    space_id: &str,
    body: &Request,
) -> Result<Response, NotionLiveError>
where
    Request: Serialize + ?Sized,
    Response: DeserializeOwned,
{
    post_private_api_with_session_and_space(credentials, endpoint, Some(space_id), body)
}

fn post_private_api_with_session_and_space<Request, Response>(
    credentials: &NotionDesktopSession,
    endpoint: NotionPrivateApiEndpoint,
    space_id: Option<&str>,
    body: &Request,
) -> Result<Response, NotionLiveError>
where
    Request: Serialize + ?Sized,
    Response: DeserializeOwned,
{
    NotionSession::new(credentials.clone())?
        .post(endpoint, space_id, body)?
        .into_json(endpoint)
}

pub(crate) fn post_private_api_for_active_user_with_session(
    credentials: &NotionDesktopSession,
    endpoint: NotionPrivateApiEndpoint,
    body: &Value,
) -> Result<ActiveUserNotionResponse, NotionLiveError> {
    let response = NotionSession::new(credentials.clone())?.post(endpoint, None, body)?;
    active_user_response(credentials.clone(), response, endpoint)
}

fn active_user_response(
    credentials: NotionDesktopSession,
    response: RawNotionResponse,
    endpoint: NotionPrivateApiEndpoint,
) -> Result<ActiveUserNotionResponse, NotionLiveError> {
    Ok(ActiveUserNotionResponse {
        active_user_id: credentials.user_id().to_string(),
        body: response.into_json(endpoint)?,
    })
}

#[cfg(test)]
mod tests {
    use super::NotionPrivateApiEndpoint;

    #[test]
    fn initial_record_sync_requires_the_workspace_header() {
        assert!(NotionPrivateApiEndpoint::SyncRecordValuesSpaceInitial.requires_space_header());
    }
}
