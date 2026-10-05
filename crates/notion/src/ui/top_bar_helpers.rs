use crate::ui::{alpha, div, img, px, rgb, Div, IconSet, ParentElement, Styled, Theme};
use gpui::App;

pub(crate) fn render_top_bar_menu_button_impl(
    icons: &IconSet,
    menu_opacity: f32,
    cx: &mut App,
) -> Div {
    div()
        .size(px(24.0))
        .mr(px(6.0))
        .rounded(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(icons.topbar_menu.render(cx))
                .size(px(20.0))
                .opacity(menu_opacity),
        )
}

pub(crate) fn render_top_bar_chip_impl(label: &str, color: u32, opacity: f32) -> Div {
    div()
        .h(px(24.0))
        .px(px(6.0))
        .rounded(px(6.0))
        .flex()
        .items_center()
        .text_size(px(14.0))
        .text_color(alpha(color, opacity))
        .child(label.to_string())
}

pub(crate) fn render_top_bar_private_chip_impl(
    theme: Theme,
    icons: &IconSet,
    opacity: TopBarPrivateChipOpacity,
    cx: &mut App,
) -> Div {
    div()
        .h(px(24.0))
        .px(px(6.0))
        .rounded(px(6.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(14.0))
        .text_color(rgb(theme.topbar_chip_text))
        .child(
            img(icons.topbar_lock.render(cx))
                .size(px(16.0))
                .opacity(opacity.lock),
        )
        .child(div().opacity(opacity.text).child("Private"))
        .child(
            img(icons.topbar_chip_chevron.render(cx))
                .ml(px(1.0))
                .w(px(11.0))
                .h(px(11.0))
                .opacity(opacity.chevron),
        )
}

pub(crate) fn render_top_bar_locked_chip_impl(
    theme: Theme,
    icons: &IconSet,
    lock_opacity: f32,
    text_opacity: f32,
    cx: &mut App,
) -> Div {
    div()
        .ml(px(4.0))
        .h(px(24.0))
        .px(px(6.0))
        .rounded(px(6.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .text_size(px(14.0))
        .text_color(rgb(theme.topbar_chip_text))
        .child(
            img(icons.topbar_lock.render(cx))
                .relative()
                .left(px(-1.0))
                .size(px(19.0))
                .opacity(lock_opacity),
        )
        .child(div().opacity(text_opacity).child("Locked"))
}

#[derive(Clone, Copy)]
pub(crate) struct TopBarPrivateChipOpacity {
    pub(crate) lock: f32,
    pub(crate) text: f32,
    pub(crate) chevron: f32,
}

impl TopBarPrivateChipOpacity {
    pub(crate) const fn new(lock: f32, text: f32, chevron: f32) -> Self {
        Self {
            lock,
            text,
            chevron,
        }
    }
}
