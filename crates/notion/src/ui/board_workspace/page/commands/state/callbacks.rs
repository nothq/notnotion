use std::rc::Rc;

use gpui_components::text_input::{TextInputAction, TextInputKeyAction, TextInputStateChange};

use crate::ui::{Context, SurfaceState};

pub(in crate::ui::board_workspace::page) fn page_composer_on_change(
    page_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputStateChange {
    let surface = cx.entity().downgrade();
    Rc::new(move |snapshot, _window, cx| {
        surface
            .update(cx, |surface, cx| {
                surface.dispatch_page_composer_action(
                    super::PageComposerAction::Change {
                        page_id: page_id.clone(),
                        snapshot,
                    },
                    cx,
                );
            })
            .ok();
    })
}

pub(in crate::ui::board_workspace::page) fn page_composer_on_submit(
    page_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputKeyAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |snapshot, _modifiers, _window, cx| {
        surface
            .update(cx, |surface, cx| {
                surface.dispatch_page_composer_action(
                    super::PageComposerAction::Submit {
                        page_id: page_id.clone(),
                        snapshot,
                    },
                    cx,
                );
            })
            .ok();
    })
}

pub(in crate::ui::board_workspace::page) fn page_composer_on_move(
    page_id: String,
    delta: isize,
    cx: &Context<SurfaceState>,
) -> TextInputAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |_window, cx| {
        surface
            .update(cx, |surface, cx| {
                if surface.page_editor.input.composer.page_id.as_deref() == Some(page_id.as_str()) {
                    surface.page_editor.move_page_command_selection(delta, cx);
                }
            })
            .ok();
    })
}

pub(in crate::ui::board_workspace::page) fn page_composer_on_escape(
    page_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |_window, cx| {
        surface
            .update(cx, |surface, cx| {
                surface.page_editor.escape_page_composer(&page_id, cx);
            })
            .ok();
    })
}

pub(in crate::ui::board_workspace::page) fn page_composer_on_focus(
    page_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |_window, cx| {
        surface
            .update(cx, |surface, cx| {
                surface.page_editor.focus_page_composer(&page_id, cx);
            })
            .ok();
    })
}

pub(in crate::ui::board_workspace::page) fn page_composer_on_backspace_when_empty(
    page_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |_window, cx| {
        surface
            .update(cx, |surface, cx| {
                if surface.page_editor.input.composer.page_id.as_deref() != Some(page_id.as_str()) {
                    return;
                }
                surface.page_editor.reset_page_composer();
                cx.notify();
            })
            .ok();
    })
}
