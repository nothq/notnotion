use std::{sync::Arc, time::Duration};

use gpui::Context;

use crate::{
    model::{NotionFilterPageId, SearchDatabaseFilterRelationPagesRequest},
    ui::{surface::DatabaseFilterUiState, SurfaceState},
};

const RELATION_SEARCH_FAILURE: &str = "database relation search failed";

pub(in crate::ui::board_workspace::dialogs::filter) fn execute_database_filter_relation_search(
    surface: &mut SurfaceState,
    cx: &mut Context<SurfaceState>,
) {
    let (session, revision) = surface.database_filter.begin_relation_search();
    surface.spawn_timer_task(
        (session, revision),
        Duration::from_millis(140),
        cx,
        |surface, (session, revision), cx| {
            if !Arc::ptr_eq(&surface.database_filter.session, &session)
                || surface.database_filter.relation_search_revision != revision
            {
                return;
            }
            let Some(request) = surface
                .database_filter
                .database_filter_relation_search_request(revision)
            else {
                return;
            };
            let Some(workspace_api) = surface.notion_startup.workspace_api() else {
                return;
            };
            let session = surface.database_filter.session.clone();
            surface.database_filter.relation_search_in_flight_revision = Some(revision);
            surface.spawn_background_task(
                request,
                cx,
                move |request| workspace_api.search_database_filter_relation_pages(request),
                move |surface, result, cx| {
                    if !Arc::ptr_eq(&surface.database_filter.session, &session) {
                        return;
                    }
                    surface.database_filter.finish_relation_search(revision);
                    if surface.database_filter.relation_search_revision == revision {
                        match result {
                            Ok(result) => {
                                surface.database_filter.relation_pages = result.pages.into()
                            }
                            Err(error) => {
                                if surface.handle_notion_workspace_failure(
                                    RELATION_SEARCH_FAILURE,
                                    error,
                                    cx,
                                ) {
                                    return;
                                }
                            }
                        }
                    }
                    cx.notify();
                },
            );
        },
    );
}

impl DatabaseFilterUiState {
    fn begin_relation_search(&mut self) -> (Arc<()>, u64) {
        self.relation_search_revision = self
            .relation_search_revision
            .checked_add(1)
            .expect("Notion relation filter search revision exhausted");
        (self.session.clone(), self.relation_search_revision)
    }

    fn finish_relation_search(&mut self, revision: u64) {
        if self.relation_search_in_flight_revision == Some(revision) {
            self.relation_search_in_flight_revision = None;
        }
    }

    pub(in crate::ui::board_workspace::dialogs::filter) fn database_filter_relation_search_request(
        &self,
        revision: u64,
    ) -> Option<SearchDatabaseFilterRelationPagesRequest> {
        let draft = self.draft.as_ref()?;
        if draft.property.filter_type() != "relation" || self.relation_search_revision != revision {
            return None;
        }
        let collection_id = draft
            .property
            .relation_collection_id
            .clone()
            .expect("loaded Notion relation properties have a collection pointer");
        let selected_page_ids = draft
            .selected_values
            .iter()
            .cloned()
            .map(|page_id| {
                NotionFilterPageId::try_from(page_id)
                    .expect("loaded relation filter page IDs are non-empty")
            })
            .collect();
        Some(
            SearchDatabaseFilterRelationPagesRequest::new(
                collection_id,
                self.value_query.clone(),
                selected_page_ids,
            )
            .expect("loaded Notion relation collection IDs are non-empty"),
        )
    }
}
