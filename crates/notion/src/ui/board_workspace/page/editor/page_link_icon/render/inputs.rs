use std::{collections::VecDeque, rc::Rc, sync::Arc};

use gpui::{App, AppContext};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

use crate::ui::{
    alpha,
    board_workspace::page::editor::page_link_icon::{
        picker::PageLinkIconPickerUi, PageLinkIconAction, PageLinkIconPickerAction,
        PageLinkIconUploadAction,
    },
    notion_named_icon_matches, px, rgb,
    view_actions::ViewActionSink,
    NotionNamedIconSlug, Theme,
};

use super::{page_link_picker_emoji_item_count, page_link_picker_named_icon_item_count};

pub(in crate::ui::board_workspace::page::editor::page_link_icon) fn new_page_link_icon_picker_ui(
    theme: Theme,
    actions: ViewActionSink<PageLinkIconAction>,
    recent_named_icons: &VecDeque<NotionNamedIconSlug>,
    cx: &mut App,
) -> PageLinkIconPickerUi {
    let search_input = page_link_icon_picker_search_input(theme, actions.clone(), cx);
    let upload_name_input = page_link_icon_picker_upload_name_input(theme, actions.clone(), cx);
    let emoji_list_state = gpui::ListState::new(
        page_link_picker_emoji_item_count("", &[]),
        gpui::ListAlignment::Top,
        gpui::px(64.0),
    );
    emoji_list_state.set_scroll_handler(move |event, window, cx| {
        actions.emit(
            PageLinkIconAction::Picker(PageLinkIconPickerAction::SyncCategoryFromScroll(
                event.visible_range.start,
            )),
            window,
            cx,
        );
    });
    let named_icon_matches = notion_named_icon_matches("");
    let recent_named_icons: Arc<[NotionNamedIconSlug]> = recent_named_icons
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .into();
    let named_icon_list_state = gpui::ListState::new(
        page_link_picker_named_icon_item_count(
            named_icon_matches.len(),
            recent_named_icons.len(),
            true,
        ),
        gpui::ListAlignment::Top,
        gpui::px(32.0),
    );
    PageLinkIconPickerUi {
        search_input,
        upload_name_input,
        emoji_list_state,
        named_icon_list_state,
        named_icon_matches,
        recent_named_icons,
    }
}

fn page_link_icon_picker_search_input(
    theme: Theme,
    actions: ViewActionSink<PageLinkIconAction>,
    cx: &mut App,
) -> gpui::Entity<TextInput> {
    let change_actions = actions.clone();
    let on_change: TextInputChange = Rc::new(move |value, window, cx| {
        change_actions.emit(
            PageLinkIconAction::Picker(PageLinkIconPickerAction::SetQuery(value)),
            window,
            cx,
        );
    });
    let on_escape: TextInputAction = Rc::new(move |window, cx| {
        actions.emit(
            PageLinkIconAction::Picker(PageLinkIconPickerAction::Dismiss),
            window,
            cx,
        );
    });
    let props = TextInputProps::single_line("")
        .placeholder("Filter…")
        .request_focus(true)
        .style(page_link_icon_input_style(theme))
        .accessibility("notion-page-link-icon-picker-search", "Filter icons")
        .on_change(on_change)
        .on_escape(on_escape);
    cx.new(|cx| TextInput::new(props, cx))
}

fn page_link_icon_picker_upload_name_input(
    theme: Theme,
    actions: ViewActionSink<PageLinkIconAction>,
    cx: &mut App,
) -> gpui::Entity<TextInput> {
    let change_actions = actions.clone();
    let on_change: TextInputChange = Rc::new(move |value, window, cx| {
        change_actions.emit(
            PageLinkIconAction::Upload(PageLinkIconUploadAction::SetUploadName(value)),
            window,
            cx,
        );
    });
    let on_escape: TextInputAction = Rc::new(move |window, cx| {
        actions.emit(
            PageLinkIconAction::Picker(PageLinkIconPickerAction::Dismiss),
            window,
            cx,
        );
    });
    let props = TextInputProps::single_line("")
        .placeholder("have-fun-with-it")
        .request_focus(false)
        .style(page_link_icon_input_style(theme))
        .accessibility("notion-page-link-icon-upload-name", "Emoji name")
        .on_change(on_change)
        .on_escape(on_escape);
    cx.new(|cx| TextInput::new(props, cx))
}

fn page_link_icon_input_style(theme: Theme) -> TextInputStyle {
    TextInputStyle {
        height: px(20.0),
        min_height: px(20.0),
        padding_x: px(0.0),
        padding_y: px(0.0),
        radius: px(0.0),
        background: gpui::Hsla::from(rgb(0x000000)).opacity(0.0),
        border: gpui::Hsla::from(rgb(0x000000)).opacity(0.0),
        focused_border: gpui::Hsla::from(rgb(0x000000)).opacity(0.0),
        text: rgb(theme.text_primary).into(),
        placeholder: rgb(theme.text_muted).into(),
        selection: alpha(0x2383e2, 0.28),
        caret: rgb(theme.text_primary).into(),
        font_size: px(14.0),
        line_height: px(20.0),
        font_family: None,
    }
}
