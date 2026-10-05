use crate::model::{NotionCollectionViewId, NotionWorkspaceLoad, NotionWorkspaceOperationFailure};
use crate::ui::board_workspace::dialogs::filter::{
    controller::DatabaseFilterController, helpers::*, prelude::*,
};

impl DatabaseFilterController<'_> {
    pub(super) fn apply_draft(&mut self) {
        let draft = self
            .state
            .draft
            .clone()
            .expect("applying a text filter requires a draft");
        let desired = database_simple_filter_from_draft(&draft);
        if let Some(path) = self.state.draft_advanced_path.clone() {
            self.apply_advanced_draft(path, desired);
            return;
        }
        self.apply_simple_draft(draft, desired);
    }

    fn apply_advanced_draft(&mut self, path: Vec<usize>, desired: Option<DatabaseSimpleFilter>) {
        let Some(filter) = desired.map(|filter| filter.filter().clone()) else {
            self.remove_advanced_node(path, DatabaseFilterHost::FullPage);
            return;
        };
        let current = self.state.effective_state(self.board);
        let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
            return;
        };
        let Some(next_group) = database_advanced_filter_replacing_node(
            group,
            &path,
            DatabaseFilterNode::Property(filter),
        ) else {
            return;
        };
        self.apply_advanced_mutation(DatabaseAdvancedFilterMutation::Set(next_group));
        self.state
            .draft
            .as_mut()
            .expect("applied advanced filter draft must remain active")
            .present_in_effective_state = true;
    }

    fn apply_simple_draft(
        &mut self,
        draft: DatabaseFilterDraft,
        desired: Option<DatabaseSimpleFilter>,
    ) {
        let mutation = match (draft.present_in_effective_state, desired) {
            (false, None) => return,
            (false, Some(filter)) => DatabaseSimpleFilterMutation::Add {
                filter,
                placement: DatabaseSimpleFilterPlacement::End,
            },
            (true, Some(filter)) => DatabaseSimpleFilterMutation::Update(filter),
            (true, None) => DatabaseSimpleFilterMutation::Remove(draft.filter_id.clone()),
        };
        let current = self
            .state
            .temporary
            .clone()
            .unwrap_or_else(|| self.state.persisted_state(self.board));
        let next = match current.applying(&DatabaseViewFilterMutation::Simple(mutation)) {
            Ok(next) => next,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        self.state.temporary = Some(next.clone());
        self.state
            .draft
            .as_mut()
            .expect("applied filter draft must remain active")
            .present_in_effective_state = database_simple_filter_from_draft(&draft).is_some();
        let revision = self.state.advance_revision();
        self.query_projection(next, revision);
    }

    pub(super) fn reset_temporary(&mut self) {
        let had_temporary = self.state.temporary.is_some();
        let persisted = self.state.persisted_state(self.board);
        self.state.reset_temporary();
        if had_temporary {
            let revision = self.state.advance_revision();
            self.query_projection(persisted, revision);
        }
    }
}

struct PendingDatabaseFilterQuery {
    filter_state: DatabaseViewFilterState,
    revision: u64,
    view_id: NotionCollectionViewId,
    session: Arc<()>,
}

struct DatabaseFilterQueryCompletion {
    revision: u64,
    view_id: NotionCollectionViewId,
    session: Arc<()>,
}

pub(super) fn schedule_database_filter_query(
    surface: &mut SurfaceState,
    filter_state: DatabaseViewFilterState,
    revision: u64,
    cx: &mut Context<SurfaceState>,
) {
    let Some(view_id) = surface.database_filter.view_id.clone() else {
        surface.print_notion_error("Notion database filtering requires an active view");
        return;
    };
    let session = surface.database_filter.session.clone();
    surface.database_filter.pending_query = Some((revision, filter_state));
    surface.database_filter.query_debounce_revision = Some(revision);
    surface.database_filter.query_failed_revision = None;
    surface.spawn_timer_task(
        (session, view_id, revision),
        std::time::Duration::from_millis(140),
        cx,
        |surface, (session, view_id, revision), cx| {
            if !Arc::ptr_eq(&surface.database_filter.session, &session)
                || surface.database_filter.view_id.as_ref() != Some(&view_id)
                || surface.database_filter.revision != revision
                || surface.database_filter.query_debounce_revision != Some(revision)
            {
                return;
            }
            surface.database_filter.query_debounce_revision = None;
            start_pending_database_filter_query(surface, cx);
        },
    );
}

pub(super) fn start_pending_database_filter_query(
    surface: &mut SurfaceState,
    cx: &mut Context<SurfaceState>,
) {
    let pending = match surface.database_filter.take_pending_query() {
        Ok(Some(pending)) => pending,
        Ok(None) => return,
        Err((revision, error)) => {
            surface.database_filter.query_failed_revision = Some(revision);
            surface.print_notion_error(error);
            return;
        }
    };
    let Some(workspace_api) = surface.notion_startup.workspace_api() else {
        surface.database_filter.query_failed_revision = Some(pending.revision);
        surface.print_notion_error("Notion database filtering requires a live workspace");
        return;
    };
    let request =
        DatabaseViewFilterQueryRequest::new(pending.view_id.clone(), pending.filter_state);
    let completion = DatabaseFilterQueryCompletion {
        revision: pending.revision,
        view_id: pending.view_id,
        session: pending.session,
    };
    surface.database_filter.query_in_flight_revision = Some(completion.revision);
    surface.spawn_background_task(
        request,
        cx,
        move |request| workspace_api.query_database_view_filter(request),
        move |surface, result, cx| {
            finish_database_filter_query(surface, completion, result, cx);
        },
    );
}

fn finish_database_filter_query(
    surface: &mut SurfaceState,
    completion: DatabaseFilterQueryCompletion,
    result: Result<NotionWorkspaceLoad, NotionWorkspaceOperationFailure>,
    cx: &mut Context<SurfaceState>,
) {
    if !Arc::ptr_eq(&surface.database_filter.session, &completion.session)
        || surface.database_filter.view_id.as_ref() != Some(&completion.view_id)
    {
        return;
    }
    if surface.database_filter.query_in_flight_revision == Some(completion.revision) {
        surface.database_filter.query_in_flight_revision = None;
    }
    if surface.database_filter.revision == completion.revision {
        match result {
            Ok(load) => match surface.replace_database_filter_projection(&completion.view_id, load)
            {
                Ok(()) => {
                    surface.database_filter.applied_revision = completion.revision;
                    surface.database_filter.query_failed_revision = None;
                }
                Err(error) => {
                    surface.database_filter.query_failed_revision = Some(completion.revision);
                    surface.print_notion_error(error);
                }
            },
            Err(error) => {
                if surface.handle_notion_workspace_failure(
                    "database filter query failed",
                    error,
                    cx,
                ) {
                    return;
                }
                surface.database_filter.query_failed_revision = Some(completion.revision);
            }
        }
    }
    start_pending_database_filter_query(surface, cx);
    cx.notify();
}

/// A pending query's revision and why it cannot run.
type BlockedFilterQuery = (u64, &'static str);

impl DatabaseFilterUiState {
    fn take_pending_query(
        &mut self,
    ) -> Result<Option<PendingDatabaseFilterQuery>, BlockedFilterQuery> {
        if self.save_in_flight
            || self.query_in_flight_revision.is_some()
            || self.query_debounce_revision.is_some()
        {
            return Ok(None);
        }
        let Some((revision, filter_state)) = self.pending_query.take() else {
            return Ok(None);
        };
        if self.revision != revision {
            return Ok(None);
        }
        let Some(view_id) = self.view_id.clone() else {
            return Err((
                revision,
                "Notion database filtering requires an active view",
            ));
        };
        Ok(Some(PendingDatabaseFilterQuery {
            filter_state,
            revision,
            view_id,
            session: self.session.clone(),
        }))
    }
}
