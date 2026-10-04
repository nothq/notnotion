use app_model::AppearanceMode;
use gpui::{rgb, Hsla};

use crate::model::{CardPageBlockColor, CardPageBlockColorValue, CardPageBlockKind};

pub(crate) fn page_block_visual_color(
    kind: CardPageBlockKind,
    color: CardPageBlockColor,
) -> CardPageBlockColor {
    if page_block_kind_supports_color(kind) {
        color
    } else {
        CardPageBlockColor::default()
    }
}

pub(crate) const fn page_block_kind_supports_color(kind: CardPageBlockKind) -> bool {
    matches!(
        kind,
        CardPageBlockKind::Text
            | CardPageBlockKind::SubHeader
            | CardPageBlockKind::SubSubHeader
            | CardPageBlockKind::Heading3
            | CardPageBlockKind::Heading4
            | CardPageBlockKind::BulletedList
            | CardPageBlockKind::NumberedList
            | CardPageBlockKind::ToDoList
            | CardPageBlockKind::ToggleList
            | CardPageBlockKind::Callout
            | CardPageBlockKind::Quote
    )
}

pub(crate) fn page_block_foreground(
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
) -> Hsla {
    match color {
        CardPageBlockColor::Text(value) => rgb(page_block_text_color_hex(value, appearance_mode)),
        CardPageBlockColor::Background(_) => rgb(page_block_primary_hex(appearance_mode)),
    }
    .into()
}

pub(crate) fn page_block_background(
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
) -> Option<Hsla> {
    let CardPageBlockColor::Background(value) = color else {
        return None;
    };
    page_block_background_hex(value, appearance_mode).map(|hex| rgb(hex).into())
}

pub(crate) fn page_callout_background(
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
) -> Option<Hsla> {
    let CardPageBlockColor::Background(value) = color else {
        return None;
    };
    page_callout_background_hex(value, appearance_mode).map(|hex| rgb(hex).into())
}

pub(crate) fn page_callout_border(
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
) -> Hsla {
    let has_background = match color {
        CardPageBlockColor::Background(value) => {
            page_callout_background_hex(value, appearance_mode).is_some()
        }
        CardPageBlockColor::Text(_) => false,
    };
    if has_background {
        return Hsla::from(rgb(0x000000)).opacity(0.0);
    }
    match appearance_mode {
        AppearanceMode::Light => Hsla::from(rgb(0x1c1301)).opacity(0.11),
        AppearanceMode::Dark => Hsla::from(rgb(0xffffeb)).opacity(0.10),
    }
}

pub(crate) const fn page_callout_background_hex(
    value: CardPageBlockColorValue,
    appearance_mode: AppearanceMode,
) -> Option<u32> {
    match (appearance_mode, value) {
        (_, CardPageBlockColorValue::Default) => None,
        (AppearanceMode::Light, CardPageBlockColorValue::Gray) => Some(0xf9f8f7),
        (AppearanceMode::Light, CardPageBlockColorValue::Brown) => Some(0xfaf8f6),
        (AppearanceMode::Light, CardPageBlockColorValue::Orange) => Some(0xfcf7f4),
        (AppearanceMode::Light, CardPageBlockColorValue::Yellow) => Some(0xfcfaef),
        (AppearanceMode::Light, CardPageBlockColorValue::Green) => Some(0xf6f9f7),
        (AppearanceMode::Light, CardPageBlockColorValue::Blue) => Some(0xf3f9fd),
        (AppearanceMode::Light, CardPageBlockColorValue::Purple) => Some(0xfaf7fc),
        (AppearanceMode::Light, CardPageBlockColorValue::Pink) => Some(0xfcf7f9),
        (AppearanceMode::Light, CardPageBlockColorValue::Red) => Some(0xfdf6f6),
        (AppearanceMode::Dark, CardPageBlockColorValue::Gray) => Some(0x202020),
        (AppearanceMode::Dark, CardPageBlockColorValue::Brown) => Some(0x25201d),
        (AppearanceMode::Dark, CardPageBlockColorValue::Orange) => Some(0x231e1b),
        (AppearanceMode::Dark, CardPageBlockColorValue::Yellow) => Some(0x23221a),
        (AppearanceMode::Dark, CardPageBlockColorValue::Green) => Some(0x1b211d),
        (AppearanceMode::Dark, CardPageBlockColorValue::Blue) => Some(0x1a2027),
        (AppearanceMode::Dark, CardPageBlockColorValue::Purple) => Some(0x221d25),
        (AppearanceMode::Dark, CardPageBlockColorValue::Pink) => Some(0x261c20),
        (AppearanceMode::Dark, CardPageBlockColorValue::Red) => Some(0x241d1d),
    }
}

pub(crate) fn page_to_do_border_color(
    color: CardPageBlockColor,
    appearance_mode: AppearanceMode,
    dark_default: u32,
) -> Hsla {
    match color {
        CardPageBlockColor::Text(CardPageBlockColorValue::Default)
        | CardPageBlockColor::Background(_) => rgb(match appearance_mode {
            AppearanceMode::Light => 0x383836,
            AppearanceMode::Dark => dark_default,
        })
        .into(),
        CardPageBlockColor::Text(_) => page_block_foreground(color, appearance_mode),
    }
}

pub(crate) const fn page_block_primary_hex(appearance_mode: AppearanceMode) -> u32 {
    match appearance_mode {
        AppearanceMode::Light => 0x2c2c2b,
        AppearanceMode::Dark => 0xf0efed,
    }
}

pub(crate) const fn page_block_text_color_hex(
    value: CardPageBlockColorValue,
    appearance_mode: AppearanceMode,
) -> u32 {
    match value {
        CardPageBlockColorValue::Default => page_block_primary_hex(appearance_mode),
        CardPageBlockColorValue::Gray => 0x7d7a75,
        CardPageBlockColorValue::Brown => 0x9f765a,
        CardPageBlockColorValue::Orange => 0xd27b2d,
        CardPageBlockColorValue::Yellow => 0xcb9434,
        CardPageBlockColorValue::Green => 0x50946e,
        CardPageBlockColorValue::Blue => 0x387dc9,
        CardPageBlockColorValue::Purple => 0x9a6bb4,
        CardPageBlockColorValue::Pink => 0xc14c8a,
        CardPageBlockColorValue::Red => 0xcf5148,
    }
}

pub(crate) const fn page_block_background_hex(
    value: CardPageBlockColorValue,
    appearance_mode: AppearanceMode,
) -> Option<u32> {
    match (appearance_mode, value) {
        (_, CardPageBlockColorValue::Default) => None,
        (AppearanceMode::Light, CardPageBlockColorValue::Gray) => Some(0xf0efed),
        (AppearanceMode::Light, CardPageBlockColorValue::Brown) => Some(0xf5ede9),
        (AppearanceMode::Light, CardPageBlockColorValue::Orange) => Some(0xfbebde),
        (AppearanceMode::Light, CardPageBlockColorValue::Yellow) => Some(0xf9f3dc),
        (AppearanceMode::Light, CardPageBlockColorValue::Green) => Some(0xe8f1ec),
        (AppearanceMode::Light, CardPageBlockColorValue::Blue) => Some(0xe5f2fc),
        (AppearanceMode::Light, CardPageBlockColorValue::Purple) => Some(0xf3ebf9),
        (AppearanceMode::Light, CardPageBlockColorValue::Pink) => Some(0xfae9f1),
        (AppearanceMode::Light, CardPageBlockColorValue::Red) => Some(0xfce9e7),
        (AppearanceMode::Dark, CardPageBlockColorValue::Gray) => Some(0x383836),
        (AppearanceMode::Dark, CardPageBlockColorValue::Brown) => Some(0x45362d),
        (AppearanceMode::Dark, CardPageBlockColorValue::Orange) => Some(0x53361f),
        (AppearanceMode::Dark, CardPageBlockColorValue::Yellow) => Some(0x504425),
        (AppearanceMode::Dark, CardPageBlockColorValue::Green) => Some(0x263d30),
        (AppearanceMode::Dark, CardPageBlockColorValue::Blue) => Some(0x233850),
        (AppearanceMode::Dark, CardPageBlockColorValue::Purple) => Some(0x3c2d47),
        (AppearanceMode::Dark, CardPageBlockColorValue::Pink) => Some(0x4e2b3c),
        (AppearanceMode::Dark, CardPageBlockColorValue::Red) => Some(0x502c29),
    }
}
