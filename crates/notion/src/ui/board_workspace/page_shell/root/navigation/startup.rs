use crate::model::NotionLaunchRoute;
use crate::ui::surface::{NotionNavigationHistoryUpdate, NotionStartup};
use crate::ui::{Arc, BoardSnapshot, NotionWorkspaceApi, NotionWorkspaceBootstrap};
#[cfg(any(test, feature = "test-support"))]
use crate::ui::{NotionFixtureInput, NotionFixtureSource};

impl NotionStartup {
    /// Records the shown page in history when a navigation leaves it.
    pub(crate) fn push_history_update(&self) -> NotionNavigationHistoryUpdate {
        self.ready_route()
            .map(|source| NotionNavigationHistoryUpdate::Push { source })
            .unwrap_or(NotionNavigationHistoryUpdate::Untracked)
    }

    pub(super) fn ready_route(&self) -> Option<NotionLaunchRoute> {
        match self {
            NotionStartup::Ready(bootstrap) => Some(bootstrap.route.clone()),
            NotionStartup::Loading { .. } | NotionStartup::Error { .. } => None,
            #[cfg(any(test, feature = "test-support"))]
            NotionStartup::Fixture(_) => None,
        }
    }

    pub(super) fn for_navigation(
        &self,
        board: BoardSnapshot,
        workspace_api: Arc<dyn NotionWorkspaceApi>,
        code_settings: crate::model::CardPageCodeSettingsCapability,
        route: NotionLaunchRoute,
    ) -> NotionStartup {
        match self {
            #[cfg(any(test, feature = "test-support"))]
            NotionStartup::Fixture(NotionFixtureInput {
                source: NotionFixtureSource::WorkspaceApi(_),
                ..
            }) => NotionStartup::Fixture(NotionFixtureInput {
                board,
                source: NotionFixtureSource::WorkspaceApi(workspace_api),
            }),
            NotionStartup::Ready(_) => NotionStartup::Ready(NotionWorkspaceBootstrap {
                route,
                workspace: board,
                workspace_api,
                code_settings,
                previous_state_disposition:
                    crate::model::NotionPreviousStateDisposition::PreserveCompatible,
            }),
            #[cfg(any(test, feature = "test-support"))]
            NotionStartup::Fixture(NotionFixtureInput {
                source: NotionFixtureSource::SnapshotPages(_),
                ..
            }) => {
                panic!("Notion navigation requires a ready workspace API")
            }
            NotionStartup::Loading { .. } | NotionStartup::Error { .. } => {
                panic!("Notion navigation requires a ready workspace API")
            }
        }
    }
}
