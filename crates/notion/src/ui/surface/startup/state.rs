#[cfg(any(test, feature = "test-support"))]
use super::super::{NotionFixtureInput, NotionFixtureSource};
use crate::model::{CardPage, NotionLaunchRoute};

use super::super::NotionStartup;

type NotionBootstrapApiRef = crate::ui::Arc<dyn crate::model::NotionBootstrapApi>;
type NotionStartupRequests = (Option<NotionBootstrapApiRef>, Option<NotionBootstrapApiRef>);

impl NotionStartup {
    pub(crate) fn workspace_api(
        &self,
    ) -> Option<crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>> {
        match self {
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(NotionFixtureInput {
                source: NotionFixtureSource::WorkspaceApi(workspace_api),
                ..
            }) => Some(workspace_api.clone()),
            Self::Ready(bootstrap) => Some(bootstrap.workspace_api.clone()),
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(NotionFixtureInput {
                source: NotionFixtureSource::SnapshotPages(_),
                ..
            }) => None,
            Self::Loading { .. } | Self::Error { .. } => None,
        }
    }

    pub(crate) fn navigation_api(
        &self,
    ) -> Result<crate::ui::Arc<dyn crate::ui::NotionWorkspaceApi>, String> {
        self.workspace_api().ok_or_else(|| {
            match self {
                Self::Loading { .. } => "Notion is still loading its workspace",
                Self::Error { .. } => "Notion could not load its workspace",
                _ => "this Notion workspace cannot open pages",
            }
            .to_string()
        })
    }

    pub(crate) fn board_url(&self) -> Option<String> {
        match self {
            Self::Ready(bootstrap) => Some(bootstrap.route.board_url().as_str().to_string()),
            Self::Loading {
                cached: Some(cached),
                ..
            }
            | Self::Error {
                cached: Some(cached),
                ..
            } => Some(cached.route.board_url().as_str().to_string()),
            Self::Loading { cached: None, .. } | Self::Error { cached: None, .. } => None,
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(_) => None,
        }
    }

    pub(crate) fn cached_workspace_visible(&self) -> bool {
        matches!(
            self,
            Self::Loading {
                cached: Some(_),
                ..
            } | Self::Error {
                cached: Some(_),
                ..
            }
        )
    }

    pub(super) fn route(&self) -> Option<&NotionLaunchRoute> {
        match self {
            Self::Ready(bootstrap) => Some(&bootstrap.route),
            Self::Loading {
                cached: Some(cached),
                ..
            }
            | Self::Error {
                cached: Some(cached),
                ..
            } => Some(&cached.route),
            Self::Loading { cached: None, .. } | Self::Error { cached: None, .. } => None,
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(_) => None,
        }
    }

    pub(super) fn take_requests(&mut self) -> NotionStartupRequests {
        match self {
            Self::Loading {
                bootstrap_api,
                request_started,
                cache_request_started,
                ..
            } => {
                let cache_api = if *cache_request_started {
                    None
                } else {
                    *cache_request_started = true;
                    Some(bootstrap_api.clone())
                };
                let bootstrap_api = if *request_started {
                    None
                } else {
                    *request_started = true;
                    Some(bootstrap_api.clone())
                };
                (cache_api, bootstrap_api)
            }
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(_) => (None, None),
            Self::Ready(_) | Self::Error { .. } => (None, None),
        }
    }

    pub(super) fn cached_page_edit_handoff(
        &self,
        edited_page: Option<&CardPage>,
    ) -> Option<(CardPage, CardPage)> {
        let cached_page = match self {
            Self::Loading {
                cached: Some(cached),
                ..
            }
            | Self::Error {
                cached: Some(cached),
                ..
            } => cached.workspace.page_content.as_ref()?,
            Self::Ready(_)
            | Self::Loading { cached: None, .. }
            | Self::Error { cached: None, .. } => return None,
            #[cfg(any(test, feature = "test-support"))]
            Self::Fixture(_) => return None,
        };
        let edited_page = edited_page?;
        (cached_page.block_id == edited_page.block_id)
            .then(|| (cached_page.clone(), edited_page.clone()))
    }
}
