use super::{alpha, div, px, rgb, AnyElement, IntoElement, ParentElement, Styled};

pub(super) fn timeline_today_icon() -> AnyElement {
    div()
        .size(px(32.0))
        .flex_none()
        .rounded_full()
        .bg(alpha(0x2783de, 0.32))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(17.0))
        .text_color(rgb(0x5e9fe8))
        .child("✓")
        .into_any_element()
}

pub(super) fn timeline_reminder_icon() -> AnyElement {
    div()
        .size(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(16.0))
                .rounded_full()
                .border_1()
                .border_color(rgb(0x5e9fe8))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(9.0))
                .text_color(rgb(0x5e9fe8))
                .child("•"),
        )
        .into_any_element()
}

pub(super) fn timeline_upgrade_icon() -> AnyElement {
    div()
        .size(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(16.0))
                .h(px(12.0))
                .rounded(px(2.0))
                .border_1()
                .border_color(rgb(0x5e9fe8))
                .border_t_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(8.0))
                .text_color(rgb(0x5e9fe8))
                .child("—"),
        )
        .into_any_element()
}
