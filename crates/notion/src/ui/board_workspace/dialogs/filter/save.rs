use crate::model::{NotionCollectionViewId, ViewTab};
use crate::ui::board_workspace::dialogs::filter::{
    apply::start_pending_database_filter_query, execute_database_filter_effect, prelude::*,
};

const FILTER_SAVE_FAILURE: &str = "database filter save failed";

struct PendingDatabaseFilterSave {
    request: DatabaseViewFilterSaveRequest,
    filter_state: DatabaseViewFilterState,
    context: DatabaseFilterSaveContext,
}

struct DatabaseFilterSaveContext {
    session: Arc<()>,
    view_id: NotionCollectionViewId,
    saved_revision: u64,
    host: DatabaseFilterHost,
}

struct DatabaseFilterSaveController<'a> {
    state: &'a mut DatabaseFilterUiState,
    view_tabs: &'a mut [ViewTab],
}

pub(super) fn execute_database_filter_save(
    surface: &mut SurfaceState,
    host: DatabaseFilterHost,
    cx: &mut Context<SurfaceState>,
) {
    let Some(workspace_api) = surface.notion_startup.workspace_api() else {
        return;
    };
    let Some(pending) = surface.database_filter.begin_save(host) else {
        return;
    };
    let context = pending.context;
    surface.spawn_background_task(
        (pending.request, pending.filter_state),
        cx,
        move |(request, filter_state)| {
            workspace_api
                .save_database_view_filter_state(request)
                .map(|()| filter_state)
        },
        move |surface, result, cx| {
            if !Arc::ptr_eq(&surface.database_filter.session, &context.session)
                || surface.database_filter.view_id.as_ref() != Some(&context.view_id)
            {
                return;
            }
            surface.database_filter.save_in_flight = false;
            let effect = match result {
                Ok(filter_state) => match (DatabaseFilterSaveController {
                    state: &mut surface.database_filter,
                    view_tabs: &mut surface.board.view_tabs,
                })
                .complete(&context, filter_state)
                {
                    Ok(effect) => effect,
                    Err(error) => {
                        surface.print_notion_error(error);
                        cx.notify();
                        return;
                    }
                },
                Err(error) => {
                    if surface.handle_notion_workspace_failure(FILTER_SAVE_FAILURE, error, cx) {
                        return;
                    }
                    None
                }
            };
            if let Some(effect) = effect {
                execute_database_filter_effect(surface, effect, cx);
            }
            start_pending_database_filter_query(surface, cx);
            cx.notify();
        },
    );
    cx.notify();
}

impl DatabaseFilterUiState {
    fn begin_save(&mut self, host: DatabaseFilterHost) -> Option<PendingDatabaseFilterSave> {
        if self.save_in_flight
            || self.applied_revision != self.revision
            || self.query_in_flight_revision == Some(self.revision)
        {
            return None;
        }
        let filter_state = self.temporary.clone()?;
        let view_id = self.view_id.clone()?;
        let request = DatabaseViewFilterSaveRequest::new(view_id.clone(), filter_state.clone());
        let context = DatabaseFilterSaveContext {
            session: self.session.clone(),
            view_id,
            saved_revision: self.revision,
            host,
        };
        self.save_in_flight = true;
        Some(PendingDatabaseFilterSave {
            request,
            filter_state,
            context,
        })
    }

    fn reset_saved_draft(&mut self) {
        self.temporary = None;
        self.draft = None;
        self.stage = DatabaseFilterDialogStage::PropertyPicker;
        self.property_input.borrow_mut().take();
        self.value_input.borrow_mut().take();
    }
}

impl DatabaseFilterSaveController<'_> {
    fn complete(
        &mut self,
        context: &DatabaseFilterSaveContext,
        filter_state: DatabaseViewFilterState,
    ) -> Result<Option<DatabaseFilterEffect>, &'static str> {
        let Some(index) = self
            .view_tabs
            .iter()
            .position(|view| view.active && view.provider_view_id == context.view_id)
        else {
            return Err("Saved Notion filter view is no longer active");
        };
        self.view_tabs[index].filters = Some(filter_state.clone());
        let saved_state_is_current = self.state.revision == context.saved_revision
            && self.state.temporary.as_ref() == Some(&filter_state);
        if saved_state_is_current {
            self.state.reset_saved_draft();
            return Ok(Some(DatabaseFilterEffect::Dismiss {
                host: context.host,
                close_local: true,
            }));
        }
        if self.state.temporary.is_none() {
            let revision = self.state.advance_revision();
            return Ok(Some(DatabaseFilterEffect::QueryProjection {
                filter_state,
                revision,
            }));
        }
        Ok(None)
    }
}

impl DatabaseFilterUiState {
    pub(crate) fn bar_is_visible(&self, board: &crate::ui::BoardSnapshot) -> bool {
        let state = self.effective_state(board);
        let has_filters = match state.simple() {
            DatabaseSimpleFiltersState::Entries(filters) => !filters.is_empty(),
            DatabaseSimpleFiltersState::Unsupported => true,
        } || !matches!(
            state.advanced(),
            crate::model::DatabaseAdvancedFilterState::None
        );
        has_filters || self.draft.is_some()
    }
}
