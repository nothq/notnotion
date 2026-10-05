use super::super::{database_view_control_property_rows, database_view_rows};
use super::{BoardSnapshot, NotionStartup, SurfaceState};
#[cfg(any(test, feature = "test-support"))]
use super::{NotionFixtureInput, NotionFixtureSource};
use crate::model::{NotionCollectionViewId, NotionWorkspaceLoad};

impl SurfaceState {
    pub(crate) fn replace_database_filter_projection(
        &mut self,
        expected_view_id: &NotionCollectionViewId,
        mut load: NotionWorkspaceLoad,
    ) -> Result<(), String> {
        let persisted_filter_state = self.board.persisted_filter_state(expected_view_id)?;
        let result_view = load
            .workspace
            .view_tabs
            .iter_mut()
            .find(|view| view.active && &view.provider_view_id == expected_view_id)
            .ok_or_else(|| projection_view_mismatch(expected_view_id))?;
        result_view.filters = Some(persisted_filter_state);
        self.notion_startup
            .replace_live_workspace(&load, &mut self.page_code_settings)?;
        self.reset_database_projection_state(load.workspace);
        Ok(())
    }

    fn reset_database_projection_state(&mut self, workspace: BoardSnapshot) {
        self.board = workspace;
        self.database_view_rows = database_view_rows(&self.board);
        self.database_view_controls.property_rows =
            database_view_control_property_rows(&self.board);
        self.columns = super::initialization::build_columns(
            &self.board,
            self.snapshot_pages.as_ref(),
            self.appearance_mode,
        );
        self.board_view
            .reset_for_projection(super::BoardViewState::new_timeline_scroll_handle(
                &self.board,
                self.viewport,
            ));
        self.date_view
            .reset_for_projection(self.board.items.clone());
        self.database_search
            .rebuild_index(&self.board, &self.columns);
    }
}

fn projection_view_mismatch(expected_view_id: &NotionCollectionViewId) -> String {
    format!(
        "Notion database filter query returned a different active view than {}",
        expected_view_id.as_str()
    )
}

fn live_workspace_required() -> String {
    "Notion database filter projection requires a live workspace".to_string()
}

impl BoardSnapshot {
    fn persisted_filter_state(
        &self,
        expected_view_id: &NotionCollectionViewId,
    ) -> Result<crate::model::DatabaseViewFilterState, String> {
        self.view_tabs
            .iter()
            .find(|view| view.active && &view.provider_view_id == expected_view_id)
            .and_then(|view| view.filters.clone())
            .ok_or_else(|| {
                format!(
                    "Notion database filter projection expected active view {}",
                    expected_view_id.as_str()
                )
            })
    }
}

impl NotionStartup {
    fn replace_live_workspace(
        &mut self,
        load: &NotionWorkspaceLoad,
        page_code_settings: &mut crate::model::CardPageCodeSettingsCapability,
    ) -> Result<(), String> {
        match self {
            NotionStartup::Ready(bootstrap) => {
                bootstrap.workspace.clone_from(&load.workspace);
                bootstrap.workspace_api.clone_from(&load.workspace_api);
                bootstrap.code_settings.clone_from(&load.code_settings);
                page_code_settings.clone_from(&load.code_settings);
                Ok(())
            }
            #[cfg(any(test, feature = "test-support"))]
            NotionStartup::Fixture(NotionFixtureInput {
                board,
                source: NotionFixtureSource::WorkspaceApi(workspace_api),
                ..
            }) => {
                board.clone_from(&load.workspace);
                workspace_api.clone_from(&load.workspace_api);
                Ok(())
            }
            NotionStartup::Loading { .. } | NotionStartup::Error { .. } => {
                Err(live_workspace_required())
            }
            #[cfg(any(test, feature = "test-support"))]
            NotionStartup::Fixture(NotionFixtureInput {
                source: NotionFixtureSource::SnapshotPages(_),
                ..
            }) => Err("Notion database filter projection requires a workspace API".to_string()),
        }
    }
}
