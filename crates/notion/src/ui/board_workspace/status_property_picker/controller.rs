use gpui::{App, WeakEntity};

use super::actions::StatusPropertyPickerAction;
use super::mutation::{StatusPropertyBackend, StatusPropertyMutationJob};
use crate::model::BoardSnapshot;
use crate::ui::surface::{
    DatabaseSearchState, NotionChromeState, StatusPropertyPickerSource, StatusPropertyPickerState,
};
use crate::ui::SurfaceState;

pub(super) struct StatusPropertyPickerContext<'a> {
    pub(super) board: &'a BoardSnapshot,
    pub(super) current_surface: WeakEntity<SurfaceState>,
    pub(super) page_host: Option<WeakEntity<SurfaceState>>,
    pub(super) backend: StatusPropertyBackend,
}

pub(super) enum StatusPropertyPickerEffect {
    ForwardToParent {
        parent: WeakEntity<SurfaceState>,
        picker: Box<StatusPropertyPickerState>,
    },
    SpawnMutation(Box<StatusPropertyMutationJob>),
    Error(String),
    Notify,
}

impl NotionChromeState {
    pub(super) fn reduce_status_property_picker_action(
        &mut self,
        database_search: &mut DatabaseSearchState,
        action: StatusPropertyPickerAction,
        context: StatusPropertyPickerContext<'_>,
        cx: &App,
    ) -> Vec<StatusPropertyPickerEffect> {
        match action {
            StatusPropertyPickerAction::OpenTable(command) => {
                let command = *command;
                match StatusPropertyPickerState::for_table(
                    context.board,
                    command.page_id,
                    command.property_id,
                    command.current_value,
                    command.position,
                ) {
                    Ok(Some(picker)) => {
                        self.route_status_property_picker(database_search, picker, &context)
                    }
                    Ok(None) => Vec::new(),
                    Err(error) => vec![StatusPropertyPickerEffect::Error(error)],
                }
            }
            StatusPropertyPickerAction::OpenPage(command) => {
                let command = *command;
                match StatusPropertyPickerState::for_page(
                    command.page_id,
                    command.property,
                    command.position,
                ) {
                    Ok(picker) => {
                        self.route_status_property_picker(database_search, picker, &context)
                    }
                    Err(error) => vec![StatusPropertyPickerEffect::Error(error)],
                }
            }
            StatusPropertyPickerAction::InstallHosted(picker) => {
                self.install_status_property_picker(database_search, *picker)
            }
            StatusPropertyPickerAction::Dismiss => self
                .dismiss_status_property_picker()
                .then_some(StatusPropertyPickerEffect::Notify)
                .into_iter()
                .collect(),
            StatusPropertyPickerAction::Select(option_id) => {
                self.select_status_property_option(option_id, context.backend, cx)
            }
        }
    }

    fn route_status_property_picker(
        &mut self,
        database_search: &mut DatabaseSearchState,
        mut picker: StatusPropertyPickerState,
        context: &StatusPropertyPickerContext<'_>,
    ) -> Vec<StatusPropertyPickerEffect> {
        let Some(parent) = context.page_host.clone() else {
            return self.install_status_property_picker(database_search, picker);
        };
        picker.source = StatusPropertyPickerSource::Inline(context.current_surface.clone());
        vec![StatusPropertyPickerEffect::ForwardToParent {
            parent,
            picker: Box::new(picker),
        }]
    }

    fn install_status_property_picker(
        &mut self,
        database_search: &mut DatabaseSearchState,
        picker: StatusPropertyPickerState,
    ) -> Vec<StatusPropertyPickerEffect> {
        self.toolbar_dialog = None;
        self.inline_toolbar_dialog = None;
        self.date_undated_dialog = None;
        self.inline_database_view_menu = None;
        self.ai_autofill_dialog = None;
        database_search.close();
        self.notion_search_open = false;
        self.status_property_picker = Some(picker);
        vec![StatusPropertyPickerEffect::Notify]
    }

    fn select_status_property_option(
        &mut self,
        option_id: gpui::SharedString,
        backend: StatusPropertyBackend,
        cx: &App,
    ) -> Vec<StatusPropertyPickerEffect> {
        let Some(state) = self.status_property_picker.as_ref().cloned() else {
            return Vec::new();
        };
        if state.commit_in_flight || state.current_option_id.as_ref() == Some(&option_id) {
            return Vec::new();
        }
        let backend = state.source.resolve_backend(backend, cx);
        let job = match StatusPropertyMutationJob::new(&state, &option_id, backend) {
            Ok(job) => job,
            Err(error) => return vec![StatusPropertyPickerEffect::Error(error)],
        };
        if let Some(active) = self.status_property_picker.as_mut() {
            active.commit_in_flight = true;
        }
        vec![
            StatusPropertyPickerEffect::Notify,
            StatusPropertyPickerEffect::SpawnMutation(Box::new(job)),
        ]
    }

    pub(crate) fn dismiss_status_property_picker(&mut self) -> bool {
        self.status_property_picker.take().is_some()
    }

    pub(crate) fn status_property_picker_handles_key_down(
        &self,
        event: &crate::ui::KeyDownEvent,
    ) -> bool {
        event.keystroke.key == "escape" && self.status_property_picker.is_some()
    }
}
