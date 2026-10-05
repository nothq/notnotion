use super::super::super::database_view_controls::{
    database_view_controls_renderer, DatabaseViewControlPlacement,
};
use super::super::filter::{database_filter_renderer, DatabaseFilterHost};
use super::{
    alpha, div, inline_toolbar_renderer, AnyElement, Bounds, Context,
    DatabaseFilterDialogPlacement, InlineToolbarAction, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Pixels, Styled, SurfaceState, ToolbarDialogKind,
};
use crate::ui::surface::NotionChromeState;
use crate::ui::view_actions::ViewActionSink;
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

impl SurfaceState {
    pub(crate) fn render_inline_toolbar_dialog_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = self
            .notion_chrome
            .inline_toolbar_dialog
            .as_ref()
            .expect("inline toolbar overlay requires dialog state")
            .clone();
        let surface = state
            .surface
            .upgrade()
            .expect("inline toolbar dialog surface must outlive its overlay");
        let parent_actions = ViewActionSink::new(cx, handle_inline_toolbar_action);
        let dialog = surface.update(cx, |surface, cx| {
            surface.render_inline_toolbar_dialog(
                state.dialog,
                InlineToolbarDialogGeometry {
                    anchor: state.anchor,
                    viewport_width: self.viewport.app_width(),
                    viewport_height: self.viewport.app_height(),
                },
                parent_actions,
                cx,
            )
        });

        div()
            .id("notion-inline-toolbar-overlay")
            .absolute()
            .inset_0()
            .child(dismissible_backdrop(
                div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
                BackdropDismissal::new(|this: &mut Self, _, _, cx| {
                    this.notion_chrome.dismiss_inline_toolbar_dialog(cx);
                }),
                cx,
            ))
            .child(dialog)
            .into_any_element()
    }
}

fn handle_inline_toolbar_action(
    surface: &mut SurfaceState,
    action: InlineToolbarAction,
    _window: &mut gpui::Window,
    cx: &mut Context<SurfaceState>,
) {
    let InlineToolbarAction::Dismiss = action;
    surface.notion_chrome.dismiss_inline_toolbar_dialog(cx);
}

impl NotionChromeState {
    pub(crate) fn handle_inline_toolbar_dialog_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<SurfaceState>,
    ) -> bool {
        let Some(state) = self.inline_toolbar_dialog.as_ref().cloned() else {
            return false;
        };
        if state.dialog == ToolbarDialogKind::Filter {
            let handled = state
                .surface
                .update(cx, |surface, cx| {
                    surface.handle_database_filter_key_down(event, cx)
                })
                .unwrap_or(false);
            if handled {
                return true;
            }
        }
        if event.keystroke.key != "escape" {
            return false;
        }
        self.dismiss_inline_toolbar_dialog(cx);
        true
    }

    pub(crate) fn dismiss_inline_toolbar_dialog(&mut self, cx: &mut Context<SurfaceState>) {
        self.inline_toolbar_dialog = None;
        cx.notify();
    }
}

impl SurfaceState {
    fn render_inline_toolbar_dialog(
        &self,
        dialog: ToolbarDialogKind,
        geometry: InlineToolbarDialogGeometry,
        parent_actions: ViewActionSink<InlineToolbarAction>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let InlineToolbarDialogGeometry {
            anchor,
            viewport_width,
            viewport_height,
        } = geometry;
        match dialog {
            ToolbarDialogKind::Filter => {
                database_filter_renderer(self, DatabaseFilterHost::Inline, cx)
                    .render_database_filter_dialog(
                        DatabaseFilterDialogPlacement::Inline {
                            toolbar_anchor: anchor,
                            viewport_width,
                            viewport_height,
                        },
                        cx,
                    )
            }
            ToolbarDialogKind::Sort => database_view_controls_renderer(self, cx)
                .render_database_sort_controls(DatabaseViewControlPlacement::Inline {
                    anchor,
                    viewport_width,
                    viewport_height,
                }),
            ToolbarDialogKind::Automations => inline_toolbar_renderer(self, parent_actions.clone())
                .render_inline_automation_dialog(viewport_width),
            ToolbarDialogKind::Properties => database_view_controls_renderer(self, cx)
                .render_database_properties_controls(DatabaseViewControlPlacement::Inline {
                    anchor,
                    viewport_width,
                    viewport_height,
                }),
            ToolbarDialogKind::Templates => inline_toolbar_renderer(self, parent_actions)
                .render_inline_templates_dialog(anchor, viewport_width),
            ToolbarDialogKind::Actions => {
                panic!("inline toolbar Actions dialog requires an explicit table action anchor")
            }
        }
    }
}

#[derive(Clone, Copy)]
struct InlineToolbarDialogGeometry {
    anchor: Bounds<Pixels>,
    viewport_width: f32,
    viewport_height: f32,
}
