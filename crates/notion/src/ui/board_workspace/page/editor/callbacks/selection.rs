use std::rc::Rc;

use gpui_components::text_input::{TextInputPointerSelection, TextInputPointerSelectionChange};

use super::super::input::{PageInputAction, PageNavigationInputAction};
use super::super::{TextInputAction, TextInputKeyAction, TextInputStateChange};
use crate::ui::view_actions::ViewActionSink;

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_select_all_again(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, _modifiers, window, cx| {
        actions.emit(
            PageInputAction::Navigation(PageNavigationInputAction::SelectAll {
                block_id: block_id.clone(),
                cursor: snapshot.cursor,
            }),
            window,
            cx,
        );
    })
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_selection_change(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputStateChange {
    Rc::new(move |snapshot, window, cx| {
        let actions = actions.clone();
        let block_id = block_id.clone();
        cx.defer_in(window, move |input, window, cx| {
            // Selection notifications are deferred until after GPUI has
            // reconciled focus. A block can be removed from the visible
            // projection in that same turn, so an old input must not restore
            // a hidden selection into the editor state.
            if !input.focus_handle_clone().is_focused(window) {
                return;
            }
            actions.emit(
                PageInputAction::Navigation(PageNavigationInputAction::SelectionChanged {
                    block_id,
                    snapshot,
                }),
                window,
                cx,
            );
        });
    })
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_pointer_selection(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputPointerSelectionChange {
    Rc::new(move |selection: TextInputPointerSelection, window, cx| {
        let actions = actions.clone();
        let block_id = block_id.clone();
        cx.defer_in(window, move |input, window, cx| {
            if !input.focus_handle_clone().is_focused(window) {
                return;
            }
            actions.emit(
                PageInputAction::Navigation(PageNavigationInputAction::PointerSelectionChanged {
                    block_id,
                    selection,
                }),
                window,
                cx,
            );
        });
    })
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_focus(
    block_id: String,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputAction {
    Rc::new(move |window, cx| {
        let actions = actions.clone();
        let block_id = block_id.clone();
        cx.defer_in(window, move |input, window, cx| {
            // Focus callbacks are deferred because the input's focus handle
            // is committed at the end of the current effect cycle. Only the
            // still-focused input may claim the active block.
            if !input.focus_handle_clone().is_focused(window) {
                return;
            }
            actions.emit(
                PageInputAction::Navigation(PageNavigationInputAction::Focused(block_id)),
                window,
                cx,
            );
        });
    })
}
