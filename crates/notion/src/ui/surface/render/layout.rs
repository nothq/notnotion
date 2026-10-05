use crate::ui::{
    div, px, relative, AnyElement, IntoElement, ParentElement, Styled, BOARD_VIEWPORT_RIGHT_GUTTER,
    BOARD_VIEWPORT_X,
};

pub(super) fn board_title(title: String) -> AnyElement {
    div()
        .pl(px(BOARD_VIEWPORT_X))
        .pr(px(BOARD_VIEWPORT_RIGHT_GUTTER))
        .pt(px(35.0))
        .text_size(px(32.0))
        .line_height(relative(1.2))
        .font_weight(gpui::FontWeight::BOLD)
        .child(title)
        .into_any_element()
}

pub(super) fn board_controls(tabs: AnyElement, toolbar: AnyElement) -> AnyElement {
    div()
        .h(px(48.0))
        .pl(px(BOARD_VIEWPORT_X))
        .pr(px(BOARD_VIEWPORT_RIGHT_GUTTER))
        .pt(px(9.0))
        .flex()
        .items_center()
        .justify_between()
        .child(tabs)
        .child(toolbar)
        .into_any_element()
}
