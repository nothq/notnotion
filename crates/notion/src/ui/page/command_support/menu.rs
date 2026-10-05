use super::{
    alpha, div, point, px, rgb, rgba, AnyElement, BoxShadow, Div, FluentBuilder, FontWeight,
    IntoElement, KeyDownEvent, ParentElement, Styled, Theme,
};

/// Notion's popover menu chrome, measured on Notion desktop 7.31.3: a 330 px
/// panel with a 10 px radius, 4 px inset, and a three-layer shadow whose last
/// layer is the 1 px hairline ring.
pub(crate) const COMMAND_MENU_WIDTH: f32 = 330.0;
pub(crate) const COMMAND_MENU_RADIUS: f32 = 10.0;
pub(crate) const COMMAND_MENU_INSET: f32 = 4.0;
pub(crate) const COMMAND_MENU_ROW_HEIGHT: f32 = 28.0;
pub(crate) const COMMAND_MENU_ROW_RADIUS: f32 = 6.0;
pub(crate) const COMMAND_MENU_ROW_PADDING_X: f32 = 8.0;
pub(crate) const COMMAND_MENU_ROW_GAP: f32 = 8.0;
pub(crate) const COMMAND_MENU_ICON_SIZE: f32 = 20.0;
pub(crate) const COMMAND_MENU_LABEL_SIZE: f32 = 14.0;
pub(crate) const COMMAND_MENU_LABEL_LINE_HEIGHT: f32 = 16.8;
pub(crate) const COMMAND_MENU_SUBLABEL_SIZE: f32 = 12.0;
pub(crate) const COMMAND_MENU_SECTION_TITLE_SIZE: f32 = 12.0;
pub(crate) const COMMAND_MENU_SECTION_TITLE_LINE_HEIGHT: f32 = 14.4;

pub(crate) fn command_menu_shadow(theme: Theme) -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: rgba(theme.menu_shadow_far),
            offset: point(px(0.0), px(20.0)),
            blur_radius: px(24.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: rgba(theme.menu_shadow_near),
            offset: point(px(0.0), px(5.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: rgba(theme.menu_ring),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
    ]
}

pub(crate) fn command_menu_panel_shell(theme: Theme) -> Div {
    div()
        .w(px(COMMAND_MENU_WIDTH))
        .rounded(px(COMMAND_MENU_RADIUS))
        .bg(rgb(theme.elevated_surface_bg))
        .shadow(command_menu_shadow(theme))
        .p(px(COMMAND_MENU_INSET))
}

/// A section header. The first header sits 6 px under the panel inset; later
/// ones follow a divider.
pub(crate) fn command_menu_section_header(
    theme: Theme,
    title: impl Into<gpui::SharedString>,
) -> Div {
    div()
        .mt(px(6.0))
        .mb(px(8.0))
        .pt(px(4.0))
        .px(px(COMMAND_MENU_ROW_PADDING_X))
        .text_size(px(COMMAND_MENU_SECTION_TITLE_SIZE))
        .line_height(px(COMMAND_MENU_SECTION_TITLE_LINE_HEIGHT))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(theme.menu_secondary_text))
        .child(title.into())
}

pub(crate) fn command_menu_section_title(theme: Theme, title: &'static str) -> Div {
    command_menu_section_header(theme, title)
}

pub(crate) fn command_menu_empty_state(theme: Theme, label: &'static str) -> AnyElement {
    div()
        .rounded(px(COMMAND_MENU_ROW_RADIUS))
        .px(px(COMMAND_MENU_ROW_PADDING_X))
        .py(px(6.0))
        .text_size(px(COMMAND_MENU_SUBLABEL_SIZE))
        .text_color(rgb(theme.menu_secondary_text))
        .child(label)
        .into_any_element()
}

/// The hairline between sections: 1 px, inset 8 px from the panel edges.
pub(crate) fn command_menu_divider(theme: Theme) -> Div {
    div()
        .mx(px(8.0))
        .mt(px(9.0))
        .h(px(1.0))
        .bg(rgba(theme.menu_ring))
}

pub(crate) fn command_menu_row_shell(theme: Theme, is_selected: bool) -> Div {
    div()
        .h(px(COMMAND_MENU_ROW_HEIGHT))
        .rounded(px(COMMAND_MENU_ROW_RADIUS))
        .px(px(COMMAND_MENU_ROW_PADDING_X))
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .bg(if is_selected {
            rgba(theme.command_menu_active_bg)
        } else {
            alpha(theme.elevated_surface_bg, 0.0)
        })
}

pub(crate) fn command_menu_label(theme: Theme, label: impl Into<gpui::SharedString>) -> Div {
    div()
        .text_size(px(COMMAND_MENU_LABEL_SIZE))
        .line_height(px(COMMAND_MENU_LABEL_LINE_HEIGHT))
        .text_color(rgb(theme.text_primary))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(label.into())
}

/// "— September 2, 2026": the muted dash and caption that follow a label.
pub(crate) fn command_menu_sublabel(theme: Theme, caption: impl Into<gpui::SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .text_size(px(COMMAND_MENU_SUBLABEL_SIZE))
        .line_height(px(COMMAND_MENU_LABEL_LINE_HEIGHT))
        .text_color(rgb(theme.menu_secondary_text))
        .whitespace_nowrap()
        .child(
            div()
                .mx(px(6.0))
                .text_color(rgb(theme.menu_dash))
                .child("—"),
        )
        .child(caption.into())
}

pub(crate) fn command_menu_option_body(
    theme: Theme,
    icon: AnyElement,
    label: &'static str,
    shortcut: Option<&'static str>,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(COMMAND_MENU_ROW_GAP))
        .min_w(px(0.0))
        .child(
            div()
                .size(px(COMMAND_MENU_ICON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .child(icon),
        )
        .child(command_menu_label(theme, label))
        .when_some(shortcut, |this, shortcut| {
            this.child(
                div()
                    .rounded(px(4.0))
                    .px(px(4.0))
                    .py(px(1.0))
                    .text_size(px(10.0))
                    .text_color(rgb(theme.text_hint))
                    .bg(alpha(theme.elevated_surface_bg, 0.32))
                    .child(shortcut),
            )
        })
}

pub(crate) fn command_menu_close_row_body(theme: Theme) -> Div {
    div()
        .flex()
        .items_center()
        .child(command_menu_label(theme, "Close menu"))
}

pub(crate) fn keystroke_input_text(event: &KeyDownEvent) -> Option<&str> {
    let modifiers = &event.keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    if let Some(key_char) = event.keystroke.key_char.as_deref() {
        return (!key_char.is_empty()).then_some(key_char);
    }
    match event.keystroke.key.as_str() {
        "space" => Some(" "),
        key if key.chars().count() == 1 => Some(key),
        _ => None,
    }
}
