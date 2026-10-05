use serde::{Deserialize, Serialize};

use super::workspace::{select_active_workspace, ActiveNotionWorkspace};
use super::ValidatedNotionSession;
use crate::live::{
    credentials::NotionDesktopSession,
    http::{post_private_api_in_space_with_session, NotionPrivateApiEndpoint},
    recent_pages::{load_recent_page_ids, NotionPageId},
    snapshot_cache::load_cached_notion_route,
    NotionLiveError,
};
use crate::model::{NotionBoardUrl, NotionCacheFailure, NotionLaunchRoute, NotionRouteSource};

const RECENT_PAGE_LIMIT: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotionRouteCandidateKind {
    Explicit,
    Cached,
    Recent,
    Home,
}

impl std::fmt::Display for NotionRouteCandidateKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Explicit => "explicit",
            Self::Cached => "cached",
            Self::Recent => "recent",
            Self::Home => "home",
        })
    }
}

pub(crate) struct NotionRouteCandidate {
    pub(crate) kind: NotionRouteCandidateKind,
    pub(crate) route: NotionLaunchRoute,
}

pub(crate) enum NotionCachedRouteCandidate {
    Miss,
    Candidate(NotionRouteCandidate),
    Failed(NotionCacheFailure),
}

pub(crate) fn explicit_route_candidate(route: NotionLaunchRoute) -> NotionRouteCandidate {
    NotionRouteCandidate {
        kind: NotionRouteCandidateKind::Explicit,
        route,
    }
}

pub(crate) fn cached_route_candidate(session: &NotionDesktopSession) -> NotionCachedRouteCandidate {
    match load_cached_notion_route(session, &NotionRouteSource::LastOpened) {
        Ok(None) => NotionCachedRouteCandidate::Miss,
        Ok(Some(route)) => NotionCachedRouteCandidate::Candidate(NotionRouteCandidate {
            kind: NotionRouteCandidateKind::Cached,
            route,
        }),
        Err(error) => NotionCachedRouteCandidate::Failed(error),
    }
}

pub(crate) struct NotionFallbackRouteResolver<'a> {
    session: &'a NotionDesktopSession,
    workspace: ActiveNotionWorkspace,
}

impl<'a> NotionFallbackRouteResolver<'a> {
    pub(crate) fn new(
        session: &'a NotionDesktopSession,
        validated: &ValidatedNotionSession,
    ) -> Result<Self, NotionLiveError> {
        Ok(Self {
            session,
            workspace: select_active_workspace(session, validated)?,
        })
    }

    pub(crate) fn recent(&self) -> Result<Option<NotionRouteCandidate>, NotionLiveError> {
        load_recent_page_ids(self.session, &self.workspace.space_id.0, RECENT_PAGE_LIMIT)?
            .into_iter()
            .next()
            .map(|page_id| route_candidate(NotionRouteCandidateKind::Recent, page_id))
            .transpose()
    }

    pub(crate) fn home(&self) -> Result<NotionRouteCandidate, NotionLiveError> {
        let response = post_private_api_in_space_with_session::<_, UserHomePagesResponse>(
            self.session,
            NotionPrivateApiEndpoint::GetUserHomePages,
            &self.workspace.space_id.0,
            &UserHomePagesRequest {
                space_id: &self.workspace.space_id.0,
                space_view_id: &self.workspace.space_view_id.0,
            },
        )?;
        route_candidate(NotionRouteCandidateKind::Home, response.home_page_id)
    }
}

fn route_candidate(
    kind: NotionRouteCandidateKind,
    page_id: NotionPageId,
) -> Result<NotionRouteCandidate, NotionLiveError> {
    let board_url =
        NotionBoardUrl::try_from(format!("https://www.notion.so/{}", page_id.compact()))
            .map_err(NotionLiveError::Fatal)?;
    Ok(NotionRouteCandidate {
        kind,
        route: NotionLaunchRoute::board(board_url),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserHomePagesRequest<'a> {
    space_id: &'a str,
    space_view_id: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserHomePagesResponse {
    home_page_id: NotionPageId,
}
