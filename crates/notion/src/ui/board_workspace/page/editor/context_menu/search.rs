use std::rc::Rc;

use gpui::{AppContext, Context};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputKeyAction, TextInputProps, TextInputStyle,
};

use super::super::{alpha, px, rgb, rgba, SurfaceState, Theme};
use super::actions::PageBlockContextMenuAction;
use crate::ui::view_actions::ViewActionSink;

struct PageBlockMenuSearchCallbacks {
    change: TextInputChange,
    submit: TextInputAction,
    escape: TextInputAction,
    up: TextInputAction,
    down: TextInputAction,
    left: TextInputKeyAction,
    right: TextInputKeyAction,
    backspace: TextInputAction,
    delete: TextInputKeyAction,
}

pub(super) fn new_page_block_context_menu_search_input(
    theme: Theme,
    actions: ViewActionSink<PageBlockContextMenuAction>,
    cx: &mut Context<SurfaceState>,
) -> gpui::Entity<TextInput> {
    let callbacks = page_block_menu_search_callbacks(actions);
    let props = TextInputProps::single_line("")
        .placeholder("Search actions…")
        .bordered(false)
        .style(page_block_context_menu_search_input_style(theme))
        .accessibility("notion-block-menu-search-input", "Search actions")
        .on_change(callbacks.change)
        .on_submit(callbacks.submit)
        .on_escape(callbacks.escape)
        .on_up(callbacks.up)
        .on_down(callbacks.down)
        .on_left_at_start(callbacks.left)
        .on_right_at_end(callbacks.right)
        .on_backspace_when_empty(callbacks.backspace)
        .on_delete_at_end(callbacks.delete);
    cx.new(|cx| TextInput::new(props, cx))
}

pub(super) fn new_page_code_language_search_input(
    theme: Theme,
    actions: ViewActionSink<PageBlockContextMenuAction>,
    cx: &mut Context<SurfaceState>,
) -> gpui::Entity<TextInput> {
    let props = TextInputProps::single_line("")
        .placeholder("Search for a language…")
        .bordered(false)
        .style(page_code_language_search_input_style(theme))
        .accessibility("notion-code-language-search-input", "Search code languages")
        .on_change(page_code_language_change(actions.clone()))
        .on_submit(page_block_menu_submit(actions.clone()))
        .on_escape(page_block_menu_escape(actions.clone()))
        .on_up(page_block_menu_move(actions.clone(), -1))
        .on_down(page_block_menu_move(actions.clone(), 1))
        .on_left_at_start(page_block_menu_left(actions));
    cx.new(|cx| TextInput::new(props, cx))
}

fn page_code_language_search_input_style(theme: Theme) -> TextInputStyle {
    TextInputStyle {
        height: px(28.0),
        min_height: px(28.0),
        padding_x: px(6.0),
        padding_y: px(4.0),
        radius: px(6.0),
        background: alpha(0x422303, 0.03),
        border: alpha(0x2383e2, 0.0),
        focused_border: alpha(0x2383e2, 0.0),
        text: rgb(theme.text_primary).into(),
        placeholder: rgb(theme.text_muted).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(14.0),
        line_height: px(20.0),
        font_family: None,
    }
}

fn page_block_context_menu_search_input_style(theme: Theme) -> TextInputStyle {
    TextInputStyle {
        height: px(28.0),
        min_height: px(28.0),
        padding_x: px(6.0),
        padding_y: px(4.0),
        radius: px(6.0),
        background: rgba(theme.dialog_input_bg),
        border: rgba(theme.surface_border),
        focused_border: rgba(theme.surface_border),
        text: rgb(theme.text_primary).into(),
        placeholder: rgb(theme.text_muted).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(14.0),
        line_height: px(20.0),
        font_family: None,
    }
}

pub(super) fn page_block_menu_label_matches(label: &str, query: &str) -> bool {
    query.is_empty()
        || label
            .as_bytes()
            .windows(query.len())
            .any(|window| window.eq_ignore_ascii_case(query.as_bytes()))
}

pub(super) fn page_code_language_indices(query: &str) -> std::sync::Arc<[usize]> {
    let query = query.trim();
    super::super::PAGE_CODE_LANGUAGES
        .iter()
        .enumerate()
        .filter_map(|(index, language)| {
            page_block_menu_label_matches(language, query).then_some(index)
        })
        .collect::<Vec<_>>()
        .into()
}

fn page_block_menu_search_callbacks(
    actions: ViewActionSink<PageBlockContextMenuAction>,
) -> PageBlockMenuSearchCallbacks {
    PageBlockMenuSearchCallbacks {
        change: page_block_menu_change(actions.clone()),
        submit: page_block_menu_submit(actions.clone()),
        escape: page_block_menu_escape(actions.clone()),
        up: page_block_menu_move(actions.clone(), -1),
        down: page_block_menu_move(actions.clone(), 1),
        left: page_block_menu_left(actions.clone()),
        right: page_block_menu_right(actions.clone()),
        backspace: page_block_menu_delete(actions.clone()),
        delete: page_block_menu_delete_at_end(actions),
    }
}

fn page_block_menu_change(actions: ViewActionSink<PageBlockContextMenuAction>) -> TextInputChange {
    Rc::new(move |value, window, cx| {
        actions.emit(PageBlockContextMenuAction::SetRootQuery(value), window, cx);
    })
}

fn page_code_language_change(
    actions: ViewActionSink<PageBlockContextMenuAction>,
) -> TextInputChange {
    Rc::new(move |value, window, cx| {
        actions.emit(
            PageBlockContextMenuAction::SetCodeLanguageQuery(value),
            window,
            cx,
        );
    })
}

fn page_block_menu_submit(actions: ViewActionSink<PageBlockContextMenuAction>) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(PageBlockContextMenuAction::Submit, window, cx);
    })
}

fn page_block_menu_escape(actions: ViewActionSink<PageBlockContextMenuAction>) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(PageBlockContextMenuAction::Dismiss, window, cx);
    })
}

fn page_block_menu_move(
    actions: ViewActionSink<PageBlockContextMenuAction>,
    delta: isize,
) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(PageBlockContextMenuAction::MoveSelection(delta), window, cx);
    })
}

fn page_block_menu_left(actions: ViewActionSink<PageBlockContextMenuAction>) -> TextInputKeyAction {
    Rc::new(move |_snapshot, _modifiers, window, cx| {
        actions.emit(PageBlockContextMenuAction::CloseSubmenu, window, cx);
    })
}

fn page_block_menu_right(
    actions: ViewActionSink<PageBlockContextMenuAction>,
) -> TextInputKeyAction {
    Rc::new(move |_snapshot, _modifiers, window, cx| {
        actions.emit(PageBlockContextMenuAction::OpenSelectedSubmenu, window, cx);
    })
}

fn page_block_menu_delete(actions: ViewActionSink<PageBlockContextMenuAction>) -> TextInputAction {
    Rc::new(move |window, cx| {
        actions.emit(PageBlockContextMenuAction::DeleteTarget, window, cx);
    })
}

fn page_block_menu_delete_at_end(
    actions: ViewActionSink<PageBlockContextMenuAction>,
) -> TextInputKeyAction {
    Rc::new(move |_snapshot, modifiers, window, cx| {
        if modifiers.platform
            || modifiers.control
            || modifiers.alt
            || modifiers.function
            || modifiers.shift
        {
            return;
        }
        actions.emit(PageBlockContextMenuAction::DeleteTarget, window, cx);
    })
}
