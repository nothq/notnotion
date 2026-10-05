use std::rc::Rc;

use super::document_edit::CrossBlockPageLineBreak;
use super::input::{PageInputAction, PageInputHostAction, PageTextInputAction};
use super::PageEditSession;
use super::{
    Context, SurfaceState, TextInputAction, TextInputKeyAction, TextInputKeyPreAction,
    TextInputLayoutChange, TextInputStateChange,
};
use crate::ui::view_actions::ViewActionSink;
use gpui::Window;

mod editing;
mod navigation;

use editing::PageInputDispatch;
mod selection;
mod simple_table;

pub(super) use navigation::{
    page_block_on_horizontal_selection_collapse, page_block_on_navigation,
    page_block_on_vertical_navigation,
};
pub(super) use selection::{
    page_block_on_focus, page_block_on_pointer_selection, page_block_on_select_all_again,
    page_block_on_selection_change,
};
pub(super) use simple_table::{
    simple_table_on_change, simple_table_on_history, simple_table_on_layout_change,
    simple_table_on_navigation, simple_table_on_pre_mutation, simple_table_on_selection_change,
    simple_table_on_tab, simple_table_on_vertical_navigation, PageSimpleTableInputContext,
};

pub(super) fn page_input_action_sink(
    cx: &Context<SurfaceState>,
) -> ViewActionSink<PageInputAction> {
    ViewActionSink::new(cx, handle_page_input_action)
}

pub(super) fn page_block_on_change(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputStateChange {
    Rc::new(move |snapshot, window, cx| {
        actions.emit(
            PageInputAction::Text(PageTextInputAction::BlockChanged {
                block_id: block_id.clone(),
                snapshot,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_title_on_change(
    page_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputStateChange {
    let surface = cx.entity().downgrade();
    Rc::new(move |snapshot, _window, cx| {
        surface
            .update(cx, |surface, cx| {
                let transition = {
                    let mut edit =
                        PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                    edit.apply_page_title_change(&page_id, snapshot);
                    edit.finish(())
                };
                surface.apply_page_edit_transition(transition, cx);
            })
            .ok();
    })
}

pub(super) fn page_title_on_submit(page_id: String, cx: &Context<SurfaceState>) -> TextInputAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |_window, cx| {
        surface
            .update(cx, |surface, cx| {
                let transition = {
                    let mut edit =
                        PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                    edit.focus_first_page_block(&page_id);
                    edit.finish(())
                };
                surface.apply_page_edit_transition(transition, cx);
            })
            .ok();
    })
}

pub(super) fn page_title_on_focus(cx: &Context<SurfaceState>) -> TextInputAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |_window, cx| {
        surface
            .update(cx, |surface, cx| surface.page_editor.focus_page_title(cx))
            .ok();
    })
}

pub(super) fn page_block_on_submit(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, _modifiers, window, cx| {
        actions.emit(
            PageInputAction::Host(PageInputHostAction::Submit {
                block_id: block_id.clone(),
                snapshot,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_block_on_cross_selection_line_break(
    block_id: String,
    cx: &Context<SurfaceState>,
) -> TextInputKeyPreAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |snapshot, modifiers, _window, cx| {
        let line_break = if modifiers.shift {
            CrossBlockPageLineBreak::ShiftEnter
        } else {
            CrossBlockPageLineBreak::Enter
        };
        surface
            .update(cx, |surface, cx| {
                let transition = {
                    let mut edit =
                        PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                    match edit.prepare_cross_block_page_line_break(&block_id, &snapshot, line_break)
                    {
                        Ok(Some(prepared)) => {
                            prepared.apply(&mut edit);
                            Ok(edit.finish(true))
                        }
                        Ok(None) => Ok(edit.finish(false)),
                        Err(error) => Err(error),
                    }
                };
                match transition {
                    Ok(transition) => surface.apply_page_edit_transition(transition, cx),
                    Err(error) => {
                        surface.print_notion_error(error);
                        cx.notify();
                        true
                    }
                }
            })
            .unwrap_or(false)
    })
}

pub(super) fn page_slash_menu_move(
    block_id: String,
    delta: isize,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(
            PageInputAction::Host(PageInputHostAction::MoveSlash {
                block_id: block_id.clone(),
                delta,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_slash_menu_close(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(
            PageInputAction::Host(PageInputHostAction::CloseSlash(block_id.clone())),
            window,
            cx,
        );
    })
}

pub(super) fn page_block_on_backspace(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, _, window, cx| {
        actions.emit(
            PageInputAction::Text(PageTextInputAction::MergeBackward {
                block_id: block_id.clone(),
                snapshot,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_block_on_delete(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, _, window, cx| {
        actions.emit(
            PageInputAction::Text(PageTextInputAction::MergeForward {
                block_id: block_id.clone(),
                snapshot,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_block_on_tab(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, modifiers, window, cx| {
        actions.emit(
            PageInputAction::Text(PageTextInputAction::Indent {
                block_id: block_id.clone(),
                outdent: modifiers.shift,
                snapshot,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_block_on_history(
    block_id: String,
    redo: bool,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, _, window, cx| {
        actions.emit(
            PageInputAction::Text(PageTextInputAction::History {
                block_id: block_id.clone(),
                redo,
                cursor: snapshot.cursor,
            }),
            window,
            cx,
        );
    })
}

pub(super) fn page_block_on_keyboard_move(
    block_id: String,
    delta: isize,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, _, window, cx| {
        if !snapshot.is_composing {
            actions.emit(
                PageInputAction::Host(PageInputHostAction::MoveBlock {
                    block_id: block_id.clone(),
                    delta,
                    cursor: snapshot.cursor,
                }),
                window,
                cx,
            );
        }
    })
}

pub(super) fn page_block_on_layout_change(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputLayoutChange {
    Rc::new(move |window, cx| {
        actions.emit(
            PageInputAction::Navigation(super::input::PageNavigationInputAction::LayoutChanged(
                block_id.clone(),
            )),
            window,
            cx,
        );
    })
}

fn handle_page_input_action(
    surface: &mut SurfaceState,
    action: PageInputAction,
    window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    let transition = {
        let mut edit = PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
        let dispatch = edit.apply_input_action(action, cx);
        edit.finish(dispatch)
    };
    match surface.apply_page_edit_transition(transition, cx) {
        PageInputDispatch::Edited(focus) => {
            if let Some(input) = focus {
                window.focus(&input.read(cx).focus_handle_clone(), cx);
            }
        }
        PageInputDispatch::Host(action) => handle_page_input_host_action(surface, action, cx),
    }
}

fn handle_page_input_host_action(
    surface: &mut SurfaceState,
    action: PageInputHostAction,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        PageInputHostAction::Submit { block_id, snapshot } => {
            surface.page_editor.clear_page_composer_handoff(&block_id);
            surface.submit_page_block(&block_id, snapshot, cx);
        }
        PageInputHostAction::MoveSlash { block_id, delta } => {
            if surface
                .page_editor
                .move_native_page_slash_selection(&block_id, delta)
            {
                cx.notify();
            }
        }
        PageInputHostAction::CloseSlash(block_id) => {
            if surface.page_editor.close_native_page_slash_menu(&block_id) {
                cx.notify();
            }
        }
        PageInputHostAction::MoveBlock {
            block_id,
            delta,
            cursor,
        } => surface.move_page_block_by_keyboard(&block_id, delta, cursor, cx),
    }
}
