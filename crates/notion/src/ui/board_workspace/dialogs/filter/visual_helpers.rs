use crate::ui::board_workspace::dialogs::filter::prelude::*;

pub(super) fn database_filter_property_icon(property_type: &str) -> PropertyPickerIconKind {
    match property_type {
        "number" | "auto_increment_id" => PropertyPickerIconKind::Number,
        "checkbox" => PropertyPickerIconKind::Checkbox,
        "formula" => PropertyPickerIconKind::Formula,
        "rollup" => PropertyPickerIconKind::Rollup,
        "created_time" | "last_edited_time" | "last_visited_time" => PropertyPickerIconKind::Time,
        "created_by" | "last_edited_by" => PropertyPickerIconKind::Person,
        "person" => PropertyPickerIconKind::Person,
        "relation" => PropertyPickerIconKind::Relation,
        "date" => PropertyPickerIconKind::Date,
        "select" | "multi_select" => PropertyPickerIconKind::List,
        "status" => PropertyPickerIconKind::Status,
        _ => PropertyPickerIconKind::Text,
    }
}

pub(super) fn database_filter_avatar_fallback(name: &str) -> String {
    name.chars()
        .find(|character| character.is_alphanumeric())
        .map(|character| character.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

pub(super) fn move_wrapped_index(current: usize, direction: isize, len: usize) -> usize {
    ((current as isize + direction).rem_euclid(len as isize)) as usize
}

pub(super) fn database_filter_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x191919, 0.05),
            offset: point(px(0.0), px(20.0)),
            blur_radius: px(24.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x191919, 0.027),
            offset: point(px(0.0), px(5.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x2a1c00, 0.07),
            offset: point(px(0.0), px(0.0)),
            blur_radius: px(0.0),
            spread_radius: px(1.0),
            inset: false,
        },
    ]
}
