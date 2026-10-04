use gpui::{point, BoxShadow};

use crate::ui::{
    alpha, div, img, px, rgb, AppearanceMode, Div, InteractiveElement, ParentElement, Styled,
};
pub(super) fn notion_search_header_control(
    icon: std::sync::Arc<gpui::RenderImage>,
    selector: &'static str,
) -> Div {
    div()
        .debug_selector(move || selector.to_string())
        .size(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(img(icon).size(px(22.0)))
}

pub(super) fn notion_search_preview_control(
    icon: std::sync::Arc<gpui::RenderImage>,
    icon_width: f32,
    selector: &'static str,
    appearance_mode: AppearanceMode,
) -> gpui::Stateful<Div> {
    div()
        .id(selector)
        .debug_selector(move || selector.to_string())
        .size(px(24.0))
        .flex_none()
        .cursor_pointer()
        .hover(move |style| {
            style
                .rounded(px(5.0))
                .bg(notion_search_neutral_tint(appearance_mode, 0.055))
        })
        .flex()
        .items_center()
        .justify_center()
        .child(img(icon).w(px(icon_width)).h(px(16.0)))
}

pub(super) fn notion_search_footer_control(
    icon: std::sync::Arc<gpui::RenderImage>,
    control_size: f32,
    icon_size: f32,
    selector: &'static str,
) -> Div {
    div()
        .debug_selector(move || selector.to_string())
        .size(px(control_size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(img(icon).size(px(icon_size)))
}

pub(super) fn notion_search_shortcut(
    key: &'static str,
    label: &'static str,
    key_color: u32,
    label_color: u32,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(px(12.0))
        .line_height(px(18.0))
        .child(div().text_color(rgb(key_color)).child(key))
        .child(div().text_color(rgb(label_color)).child(label))
}

pub(super) fn notion_search_neutral_tint(
    appearance_mode: AppearanceMode,
    opacity: f32,
) -> gpui::Hsla {
    match appearance_mode {
        AppearanceMode::Light => alpha(0x37352f, opacity),
        AppearanceMode::Dark => alpha(0xffffff, opacity),
    }
}

pub(super) fn notion_search_border_tint(appearance_mode: AppearanceMode) -> gpui::Hsla {
    match appearance_mode {
        AppearanceMode::Light => alpha(0x37352f, 0.12),
        AppearanceMode::Dark => alpha(0xfffff3, 0.082),
    }
}

pub(super) fn notion_search_loading_separator(appearance_mode: AppearanceMode) -> gpui::Hsla {
    match appearance_mode {
        AppearanceMode::Light => alpha(0x37352f, 0.08),
        AppearanceMode::Dark => rgb(0x323232).into(),
    }
}

pub(super) fn notion_search_preview_heading_text_color(
    appearance_mode: AppearanceMode,
) -> gpui::Hsla {
    rgb(match appearance_mode {
        AppearanceMode::Light => 0x37352f,
        AppearanceMode::Dark => 0xbcbab6,
    })
    .into()
}

pub(super) fn notion_search_preview_body_text_color(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(match appearance_mode {
        AppearanceMode::Light => 0x787774,
        AppearanceMode::Dark => 0xada9a3,
    })
    .into()
}

pub(super) const fn notion_search_footer_key_color(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x787774,
        AppearanceMode::Dark => 0xada9a3,
    }
}

pub(super) const fn notion_search_footer_label_color(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x9b9a97,
        AppearanceMode::Dark => 0x7d7a75,
    }
}

pub(super) fn notion_search_shadows(appearance_mode: AppearanceMode) -> Vec<BoxShadow> {
    let (deep_opacity, near_opacity) = match appearance_mode {
        AppearanceMode::Light => (0.14, 0.10),
        AppearanceMode::Dark => (0.64, 0.56),
    };
    vec![
        BoxShadow {
            color: alpha(0x191919, deep_opacity),
            offset: point(px(0.0), px(24.0)),
            blur_radius: px(48.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, near_opacity),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
            inset: false,
        },
    ]
}
