use std::sync::Arc;

use gpui::{App, Context, SharedString};
use gpui_components::spawn_background_task_for_entity;

use crate::model::{
    NotionWorkspaceApi, NotionWorkspaceOperationFailure, SetDatabaseStatusPropertyRequest,
};
use crate::ui::board_workspace::PageDocumentAction;
use crate::ui::surface::{
    NotionChromeState, StatusPropertyPickerSource, StatusPropertyPickerState,
    StatusPropertyPickerTarget,
};
use crate::ui::SurfaceState;

enum StatusPropertyRefreshRequest {
    Database {
        board_url: String,
        expected_view_id: crate::model::NotionCollectionViewId,
    },
    Page {
        page_id: String,
    },
}

pub(super) enum StatusPropertyRefresh {
    Database {
        expected_view_id: crate::model::NotionCollectionViewId,
        load: Box<crate::model::NotionWorkspaceLoad>,
    },
    Page(Box<crate::model::CardPage>),
}

struct PreparedStatusPropertyMutation {
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    request: SetDatabaseStatusPropertyRequest,
    refresh: StatusPropertyRefreshRequest,
}

pub(super) struct StatusPropertyMutationJob {
    operation: PreparedStatusPropertyMutation,
    session: Arc<()>,
    source: StatusPropertyPickerSource,
}

pub(super) struct StatusPropertyMutationCompletion {
    session: Arc<()>,
    source: StatusPropertyPickerSource,
    result: Result<StatusPropertyRefresh, NotionWorkspaceOperationFailure>,
}

type StatusPropertyMutationResult = Result<StatusPropertyRefresh, NotionWorkspaceOperationFailure>;

impl StatusPropertyMutationCompletion {
    fn into_parts(
        self,
    ) -> (
        Arc<()>,
        StatusPropertyPickerSource,
        StatusPropertyMutationResult,
    ) {
        (self.session, self.source, self.result)
    }
}

pub(super) struct StatusPropertyBackend {
    pub(super) workspace_api: Option<Arc<dyn NotionWorkspaceApi>>,
    pub(super) board_url: Option<String>,
}

impl StatusPropertyMutationJob {
    pub(super) fn new(
        state: &StatusPropertyPickerState,
        option_id: &SharedString,
        backend: StatusPropertyBackend,
    ) -> Result<Self, String> {
        Ok(Self {
            operation: PreparedStatusPropertyMutation::new(state, option_id, backend)?,
            session: state.session.clone(),
            source: state.source.clone(),
        })
    }

    pub(super) fn spawn(self, cx: &mut Context<SurfaceState>) {
        let Self {
            operation,
            session,
            source,
        } = self;
        spawn_background_task_for_entity(
            operation,
            cx,
            PreparedStatusPropertyMutation::execute,
            move |surface, result, cx| {
                surface.finish_status_property_picker_completion(
                    StatusPropertyMutationCompletion {
                        session,
                        source,
                        result,
                    },
                    cx,
                );
            },
        );
    }
}

impl SurfaceState {
    pub(super) fn finish_status_property_picker_completion(
        &mut self,
        completion: StatusPropertyMutationCompletion,
        cx: &mut Context<Self>,
    ) {
        let (session, source, result) = completion.into_parts();
        match result {
            Ok(refresh) => {
                let projection = match source {
                    StatusPropertyPickerSource::Current => match refresh {
                        StatusPropertyRefresh::Database {
                            expected_view_id,
                            load,
                        } => self
                            .replace_database_filter_projection(&expected_view_id, *load)
                            .inspect(|()| cx.notify()),
                        StatusPropertyRefresh::Page(page) => {
                            let page = *page;
                            let authority = Arc::new(page.clone());
                            let action =
                                PageDocumentAction::replace_loaded_and_authority(page, authority);
                            self.dispatch_page_document_action(action, cx);
                            Ok(())
                        }
                    },
                    StatusPropertyPickerSource::Inline(surface) => {
                        apply_inline_status_property_refresh(surface, refresh, cx)
                    }
                };
                if let Err(error) = projection {
                    self.print_notion_error(error);
                }
                let closed = self
                    .notion_chrome
                    .close_matching_status_property_picker(&session);
                if closed {
                    cx.notify();
                }
            }
            Err(error) => {
                let restarted = match source {
                    StatusPropertyPickerSource::Current => self.handle_notion_workspace_failure(
                        "status property update failed",
                        error,
                        cx,
                    ),
                    StatusPropertyPickerSource::Inline(surface) => {
                        handle_inline_status_property_failure(surface, error, cx)
                    }
                };
                let recovered = self
                    .notion_chrome
                    .finish_status_property_failure(&session, restarted);
                if recovered {
                    cx.notify();
                }
            }
        }
    }
}

fn apply_inline_status_property_refresh(
    surface: gpui::WeakEntity<SurfaceState>,
    refresh: StatusPropertyRefresh,
    cx: &mut Context<SurfaceState>,
) -> Result<(), String> {
    surface
        .update(cx, move |surface, cx| match refresh {
            StatusPropertyRefresh::Database {
                expected_view_id,
                load,
            } => surface
                .replace_database_filter_projection(&expected_view_id, *load)
                .inspect(|()| cx.notify()),
            StatusPropertyRefresh::Page(page) => {
                let page = *page;
                surface.dispatch_page_document_action(
                    PageDocumentAction::replace_loaded_and_authority(page.clone(), Arc::new(page)),
                    cx,
                );
                Ok(())
            }
        })
        .unwrap_or_else(|_| Err("status projection source is no longer mounted".to_string()))
}

fn handle_inline_status_property_failure(
    surface: gpui::WeakEntity<SurfaceState>,
    error: NotionWorkspaceOperationFailure,
    cx: &mut Context<SurfaceState>,
) -> bool {
    surface
        .update(cx, move |surface, cx| {
            surface.handle_notion_workspace_failure("status property update failed", error, cx)
        })
        .unwrap_or(true)
}

impl NotionChromeState {
    fn close_matching_status_property_picker(&mut self, session: &Arc<()>) -> bool {
        let matches = self
            .status_property_picker
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(&active.session, session));
        if matches {
            self.status_property_picker = None;
        }
        matches
    }

    fn finish_status_property_failure(&mut self, session: &Arc<()>, restarted: bool) -> bool {
        if restarted {
            return self.close_matching_status_property_picker(session);
        }
        let Some(active) = self
            .status_property_picker
            .as_mut()
            .filter(|active| Arc::ptr_eq(&active.session, session))
        else {
            return false;
        };
        active.commit_in_flight = false;
        true
    }
}

impl StatusPropertyPickerSource {
    pub(super) fn resolve_backend(
        &self,
        current: StatusPropertyBackend,
        cx: &App,
    ) -> StatusPropertyBackend {
        match self {
            Self::Current => current,
            Self::Inline(surface) => {
                let source = surface.upgrade();
                StatusPropertyBackend {
                    workspace_api: source
                        .as_ref()
                        .and_then(|source| source.read(cx).notion_startup.workspace_api()),
                    board_url: source
                        .as_ref()
                        .and_then(|source| source.read(cx).notion_startup.board_url()),
                }
            }
        }
    }
}

impl PreparedStatusPropertyMutation {
    fn new(
        state: &StatusPropertyPickerState,
        option_id: &SharedString,
        backend: StatusPropertyBackend,
    ) -> Result<Self, String> {
        let request = status_property_mutation_request(state, option_id)?;
        let workspace_api = backend
            .workspace_api
            .ok_or_else(|| "status editing requires a live workspace API".to_string())?;
        let refresh = status_property_refresh_request(state, backend.board_url)?;
        Ok(Self {
            workspace_api,
            request,
            refresh,
        })
    }

    fn execute(self) -> Result<StatusPropertyRefresh, NotionWorkspaceOperationFailure> {
        self.workspace_api
            .set_database_status_property(self.request)?;
        load_status_property_refresh(self.workspace_api, self.refresh)
    }
}

fn status_property_mutation_request(
    state: &StatusPropertyPickerState,
    option_id: &SharedString,
) -> Result<SetDatabaseStatusPropertyRequest, String> {
    match &state.target {
        StatusPropertyPickerTarget::DatabaseView {
            page_id,
            expected_view_id,
        } => SetDatabaseStatusPropertyRequest::for_database_view(
            page_id.to_string(),
            expected_view_id.clone(),
            &state.property,
            option_id,
        ),
        StatusPropertyPickerTarget::LoadedPage { page_id } => {
            SetDatabaseStatusPropertyRequest::for_loaded_page(
                page_id.to_string(),
                &state.property,
                option_id,
            )
        }
    }
}

fn status_property_refresh_request(
    state: &StatusPropertyPickerState,
    board_url: Option<String>,
) -> Result<StatusPropertyRefreshRequest, String> {
    match &state.target {
        StatusPropertyPickerTarget::DatabaseView {
            expected_view_id, ..
        } => Ok(StatusPropertyRefreshRequest::Database {
            board_url: board_url
                .ok_or_else(|| "status editing requires a live database route".to_string())?,
            expected_view_id: expected_view_id.clone(),
        }),
        StatusPropertyPickerTarget::LoadedPage { page_id } => {
            Ok(StatusPropertyRefreshRequest::Page {
                page_id: page_id.to_string(),
            })
        }
    }
}

fn load_status_property_refresh(
    workspace_api: Arc<dyn NotionWorkspaceApi>,
    refresh: StatusPropertyRefreshRequest,
) -> Result<StatusPropertyRefresh, NotionWorkspaceOperationFailure> {
    match refresh {
        StatusPropertyRefreshRequest::Database {
            board_url,
            expected_view_id,
        } => workspace_api.load_notion_workspace(&board_url).map(|load| {
            StatusPropertyRefresh::Database {
                expected_view_id,
                load: Box::new(load),
            }
        }),
        StatusPropertyRefreshRequest::Page { page_id } => workspace_api
            .load_card_page(&page_id)
            .map(Box::new)
            .map(StatusPropertyRefresh::Page),
    }
}
