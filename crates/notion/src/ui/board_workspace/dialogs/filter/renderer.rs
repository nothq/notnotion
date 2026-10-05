use crate::model::BoardSnapshot;
use crate::ui::board_workspace::PageShellIconRenderer;
use crate::ui::surface::DatabaseFilterUiState;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{AppearanceMode, Theme};
use gpui::{App, ClickEvent, KeyDownEvent, Window};

use super::types::{DatabaseFilterAction, DatabaseFilterHost};

#[derive(Clone)]
pub(super) struct DatabaseFilterRenderResources {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) page_icons: PageShellIconRenderer,
    pub(super) workspace_editable: bool,
    pub(super) host: DatabaseFilterHost,
    pub(super) actions: ViewActionSink<DatabaseFilterAction>,
}

pub(in crate::ui::board_workspace::dialogs) struct DatabaseFilterRenderer<'a> {
    pub(super) state: &'a DatabaseFilterUiState,
    pub(super) board: &'a BoardSnapshot,
    pub(super) resources: DatabaseFilterRenderResources,
}

impl<'a> DatabaseFilterRenderer<'a> {
    pub(super) fn new(
        state: &'a DatabaseFilterUiState,
        board: &'a BoardSnapshot,
        resources: DatabaseFilterRenderResources,
    ) -> Self {
        Self {
            state,
            board,
            resources,
        }
    }
}

impl DatabaseFilterRenderResources {
    pub(super) fn filter_click_listener(
        &self,
        action: DatabaseFilterAction,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        self.actions.listener(move |_: &ClickEvent, _, cx| {
            cx.stop_propagation();
            action.clone()
        })
    }

    pub(super) fn filter_key_listener(
        &self,
        action: DatabaseFilterAction,
    ) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
        let actions = self.actions.clone();
        move |event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            actions.emit(action.clone(), window, cx);
        }
    }
}
