use crate::ui::board_workspace::PageEditSession;

use std::rc::Rc;

use super::super::input::{PageInputAction, PageNavigationInputAction};
use super::super::navigation::PageBlockNavigation;
use super::super::{
    Context, SurfaceState, TextInputKeyAction, TextInputKeyPreAction,
    TextInputVerticalBoundaryAction,
};
use crate::ui::view_actions::ViewActionSink;

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_navigation(
    block_id: String,
    navigation: PageBlockNavigation,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputKeyAction {
    Rc::new(move |snapshot, modifiers, window, cx| {
        let actions = actions.clone();
        let block_id = block_id.clone();
        cx.defer_in(window, move |input, window, cx| {
            if !input.focus_handle_clone().is_focused(window) {
                return;
            }
            actions.emit(
                PageInputAction::Navigation(PageNavigationInputAction::Navigate {
                    block_id,
                    navigation,
                    snapshot,
                    window_x: None,
                    extend: modifiers.shift,
                }),
                window,
                cx,
            );
        });
    })
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_horizontal_selection_collapse(
    block_id: String,
    navigation: PageBlockNavigation,
    cx: &Context<SurfaceState>,
) -> TextInputKeyPreAction {
    let surface = cx.entity().downgrade();
    Rc::new(move |snapshot, modifiers, window, cx| {
        if modifiers.shift {
            return false;
        }
        let collapse = surface
            .update(cx, |surface, cx| {
                let transition = {
                    let mut edit =
                        PageEditSession::new(&mut surface.page_editor, &surface.page_documents);
                    let result = edit.collapse_cross_block_text_selection(
                        &block_id,
                        snapshot.selection.end,
                        navigation,
                        cx,
                    );
                    edit.finish(result)
                };
                surface.apply_page_edit_transition(transition, cx)
            })
            .ok()
            .flatten();
        let Some(collapse) = collapse else {
            return false;
        };
        cx.defer_in(window, move |input, window, cx| {
            if !input.focus_handle_clone().is_focused(window) {
                return;
            }
            collapse.apply(input, window, cx);
        });
        true
    })
}

pub(in crate::ui::board_workspace::page::editor) fn page_block_on_vertical_navigation(
    block_id: String,
    navigation: PageBlockNavigation,
    actions: ViewActionSink<PageInputAction>,
) -> TextInputVerticalBoundaryAction {
    Rc::new(move |boundary, modifiers, window, cx| {
        let actions = actions.clone();
        let block_id = block_id.clone();
        cx.defer_in(window, move |input, window, cx| {
            if !input.focus_handle_clone().is_focused(window) {
                return;
            }
            actions.emit(
                PageInputAction::Navigation(PageNavigationInputAction::Navigate {
                    block_id,
                    navigation,
                    snapshot: boundary.snapshot,
                    window_x: Some(boundary.window_x),
                    extend: modifiers.shift,
                }),
                window,
                cx,
            );
        });
        true
    })
}
