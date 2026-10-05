use std::rc::Rc;

use gpui_components::text_input::{
    TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

use super::DateUndatedAction;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{alpha, px, rgb, Theme};

pub(super) fn date_undated_search_input_props(
    theme: Theme,
    actions: ViewActionSink<DateUndatedAction>,
) -> TextInputProps {
    TextInputProps::single_line("")
        .placeholder("Search for a page…")
        .request_focus(true)
        .style(TextInputStyle {
            height: px(28.0),
            min_height: px(28.0),
            padding_x: px(6.0),
            padding_y: px(4.0),
            radius: px(6.0),
            background: alpha(theme.text_primary, 0.035),
            border: alpha(theme.text_primary, 0.0),
            focused_border: rgb(0x2383e2).into(),
            text: rgb(theme.text_primary).into(),
            placeholder: rgb(theme.text_hint).into(),
            selection: alpha(0x2383e2, 0.28),
            caret: rgb(theme.text_primary).into(),
            font_size: px(14.0),
            line_height: px(20.0),
            font_family: None,
        })
        .accessibility("notion-date-undated-search", "Search for a page…")
        .on_change(date_undated_search_on_change(actions.clone()))
        .on_escape(date_undated_search_on_escape(actions))
}

fn date_undated_search_on_change(actions: ViewActionSink<DateUndatedAction>) -> TextInputChange {
    Rc::new(move |value, window, cx| {
        actions.emit(DateUndatedAction::SearchChanged(value), window, cx);
    })
}

fn date_undated_search_on_escape(actions: ViewActionSink<DateUndatedAction>) -> TextInputAction {
    Rc::new(move |window, cx| actions.emit(DateUndatedAction::Dismiss, window, cx))
}
