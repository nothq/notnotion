use std::sync::Arc;

use gpui::{img, px, rgb, App, Div, Image, IntoElement, ParentElement, Styled};

use crate::ui::{
    alpha, calendar_property_icon, checkbox_property_icon, div, list_property_icon,
    number_property_icon, person_property_icon, relation_property_icon, render_svg_image,
    status_property_icon, time_property_icon, AppearanceMode, FontWeight, PropertyPickerIconKind,
    Theme,
};

pub(crate) fn render_property_picker_option_icon(
    theme: Theme,
    appearance_mode: AppearanceMode,
    icon_kind: PropertyPickerIconKind,
    selected: bool,
    cx: &mut App,
) -> Div {
    let icon_fill = if selected {
        theme.text_primary
    } else {
        theme.text_secondary
    };

    match icon_kind {
        PropertyPickerIconKind::Text => render_text_icon(appearance_mode, icon_fill),
        PropertyPickerIconKind::Number => render_svg_icon(number_property_icon(icon_fill), cx),
        PropertyPickerIconKind::Checkbox => render_svg_icon(checkbox_property_icon(icon_fill), cx),
        PropertyPickerIconKind::Formula => render_glyph_icon(appearance_mode, "∑", icon_fill),
        PropertyPickerIconKind::Rollup => render_glyph_icon(appearance_mode, "↗", icon_fill),
        PropertyPickerIconKind::Time => render_svg_icon(time_property_icon(icon_fill), cx),
        PropertyPickerIconKind::Person => render_svg_icon(person_property_icon(icon_fill), cx),
        PropertyPickerIconKind::Relation => render_svg_icon(relation_property_icon(icon_fill), cx),
        PropertyPickerIconKind::Date => render_svg_icon(calendar_property_icon(icon_fill), cx),
        PropertyPickerIconKind::List => render_svg_icon(list_property_icon(icon_fill), cx),
        PropertyPickerIconKind::Status => render_svg_icon(status_property_icon(icon_fill), cx),
    }
}

fn render_text_icon(appearance_mode: AppearanceMode, icon_fill: u32) -> Div {
    icon_box(
        matches!(appearance_mode, AppearanceMode::Light),
        div()
            .text_size(px(11.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(icon_fill))
            .child("Aa"),
    )
}

fn render_glyph_icon(appearance_mode: AppearanceMode, glyph: &'static str, icon_fill: u32) -> Div {
    icon_box(
        matches!(appearance_mode, AppearanceMode::Light),
        div()
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(icon_fill))
            .child(glyph),
    )
}

fn render_svg_icon(icon: Arc<Image>, cx: &mut App) -> Div {
    icon_box(false, img(render_svg_image(icon, cx)).size(px(12.0)))
}

fn icon_box(show_background: bool, child: impl IntoElement) -> Div {
    div()
        .w(px(24.0))
        .h(px(16.0))
        .mr(px(2.0))
        .rounded(px(4.0))
        .bg(if show_background {
            alpha(0xffffff, 1.0)
        } else {
            alpha(0xffffff, 0.0)
        })
        .flex()
        .items_center()
        .justify_center()
        .child(child)
}
