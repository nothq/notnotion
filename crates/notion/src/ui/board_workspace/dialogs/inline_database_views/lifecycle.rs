use super::renderer::{InlineDatabaseViewMenuAction, InlineDatabaseViewMenuRenderer};
use super::{
    alpha, div, inline_database_view_menu_layout, AnyElement, Bounds, Context, FluentBuilder,
    InlineDatabaseViewMenuSelection, InlineDatabaseViewMenuState, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Pixels, Styled, SurfaceState, ViewTab, Window,
};
use crate::ui::surface::{InlineDatabaseViewMenuOwner, NotionChromeState};
use gpui_components::backdrop::dismissible_backdrop_with_handler;

impl SurfaceState {
    pub(crate) fn render_inline_database_view_menu_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = self
            .notion_chrome
            .inline_database_view_menu
            .as_ref()
            .expect("inline database view menu overlay requires menu state")
            .clone();
        self.inline_database_view_menu_renderer(cx)
            .render_overlay(state, cx)
    }

    pub(crate) fn toggle_inline_database_view_menu(
        &mut self,
        anchor: Bounds<Pixels>,
        owner: InlineDatabaseViewMenuOwner,
        view_tabs: std::sync::Arc<[ViewTab]>,
        cx: &mut Context<Self>,
    ) {
        let InlineDatabaseViewMenuOwner {
            database_block_id,
            inline_view,
        } = owner;
        let already_open = self
            .notion_chrome
            .inline_database_view_menu
            .as_ref()
            .is_some_and(|state| state.database_block_id == database_block_id);
        if already_open {
            self.notion_chrome.dismiss_inline_database_view_menu(cx);
            return;
        }
        let highlighted = view_tabs
            .iter()
            .find(|tab| tab.active)
            .map(|tab| InlineDatabaseViewMenuSelection::View(tab.provider_view_id.clone()))
            .expect("inline database view menu requires one active provider view");
        let input = self
            .inline_database_view_menu_renderer(cx)
            .new_inline_database_view_menu_input(&database_block_id, cx);
        self.notion_chrome.inline_database_view_menu = Some(InlineDatabaseViewMenuState {
            anchor,
            database_block_id,
            inline_view,
            view_tabs,
            query: "".into(),
            highlighted,
            input,
            new_view_picker_open: false,
        });
        self.notion_chrome.inline_toolbar_dialog = None;
        self.notion_chrome.toolbar_dialog = None;
        self.notion_chrome.ai_autofill_dialog = None;
        self.notion_chrome.notion_search_open = false;
        self.notion_chrome.notion_ai_open = false;
        cx.notify();
    }
}

impl NotionChromeState {
    pub(crate) fn dismiss_inline_database_view_menu(&mut self, cx: &mut Context<SurfaceState>) {
        if self.inline_database_view_menu.take().is_some() {
            cx.notify();
        }
    }

    pub(crate) fn handle_inline_database_view_menu_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<SurfaceState>,
    ) -> bool {
        if self.inline_database_view_menu.is_none() || event.keystroke.modifiers.modified() {
            return false;
        }
        let handled = match event.keystroke.key.as_str() {
            "up" | "arrowup" => {
                self.move_inline_database_view_menu_highlight(-1, cx);
                true
            }
            "down" | "arrowdown" => {
                self.move_inline_database_view_menu_highlight(1, cx);
                true
            }
            "home" => {
                self.move_inline_database_view_menu_highlight_to_edge(false, cx);
                true
            }
            "end" => {
                self.move_inline_database_view_menu_highlight_to_edge(true, cx);
                true
            }
            "enter" => {
                self.activate_inline_database_view_menu_highlight(cx);
                true
            }
            "escape" => {
                self.escape_inline_database_view_menu(cx);
                true
            }
            _ => false,
        };
        if handled {
            window.prevent_default();
            cx.stop_propagation();
        }
        handled
    }
}

impl InlineDatabaseViewMenuRenderer {
    pub(super) fn render_overlay(
        &self,
        state: InlineDatabaseViewMenuState,
        cx: &mut gpui::App,
    ) -> AnyElement {
        let layout = inline_database_view_menu_layout(
            &state,
            self.viewport.app_width(),
            self.viewport.app_height(),
        );
        let overlay_id = format!(
            "notion-inline-database-view-menu-overlay-{}",
            state.database_block_id
        );
        div()
            .id(overlay_id)
            .absolute()
            .inset_0()
            .child(dismissible_backdrop_with_handler(
                div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
                self.actions
                    .listener(|_, _, _| InlineDatabaseViewMenuAction::Dismiss),
            ))
            .child(self.render_inline_database_view_menu_dialog(&state, layout, cx))
            .when(state.new_view_picker_open, |overlay| {
                overlay.child(self.render_inline_database_new_view_picker(&state, layout))
            })
            .into_any_element()
    }
}
