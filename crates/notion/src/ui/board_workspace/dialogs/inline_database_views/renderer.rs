use std::sync::Arc;

use gpui::{Context, Window};

use super::{AppearanceMode, InlineDatabaseViewMenuSelection, SurfaceState};
use crate::ui::{view_actions::ViewActionSink, IconSet, Theme, Viewport};

pub(super) struct InlineDatabaseViewMenuRenderer {
    pub(super) theme: Theme,
    pub(super) appearance_mode: AppearanceMode,
    pub(super) icons: Arc<IconSet>,
    pub(super) viewport: Viewport,
    pub(super) actions: ViewActionSink<InlineDatabaseViewMenuAction>,
}

pub(super) enum InlineDatabaseViewMenuAction {
    Dismiss,
    Highlight(InlineDatabaseViewMenuSelection),
    Activate(InlineDatabaseViewMenuSelection),
    Query(String),
    Submit,
    Escape,
    Move(isize),
}

impl SurfaceState {
    pub(super) fn inline_database_view_menu_renderer(
        &self,
        cx: &Context<Self>,
    ) -> InlineDatabaseViewMenuRenderer {
        InlineDatabaseViewMenuRenderer {
            theme: self.theme,
            appearance_mode: self.appearance_mode,
            icons: self.icons.clone(),
            viewport: self.viewport,
            actions: ViewActionSink::new(cx, handle_inline_database_view_menu_action),
        }
    }
}

fn handle_inline_database_view_menu_action(
    surface: &mut SurfaceState,
    action: InlineDatabaseViewMenuAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let chrome = &mut surface.notion_chrome;
    match action {
        InlineDatabaseViewMenuAction::Dismiss => chrome.dismiss_inline_database_view_menu(cx),
        InlineDatabaseViewMenuAction::Highlight(selection) => {
            chrome.highlight_inline_database_view_menu_selection(selection, cx);
        }
        InlineDatabaseViewMenuAction::Activate(selection) => {
            chrome.activate_inline_database_view_menu_selection(selection, cx);
        }
        InlineDatabaseViewMenuAction::Query(value) => {
            chrome.set_inline_database_view_menu_query(value, cx);
        }
        InlineDatabaseViewMenuAction::Submit => {
            chrome.activate_inline_database_view_menu_highlight(cx)
        }
        InlineDatabaseViewMenuAction::Escape => chrome.escape_inline_database_view_menu(cx),
        InlineDatabaseViewMenuAction::Move(delta) => {
            chrome.move_inline_database_view_menu_highlight(delta, cx)
        }
    }
}
