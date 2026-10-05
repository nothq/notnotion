use app_model::AppearanceMode;

use super::CardIcons;
use crate::ui::{
    svg_with_group, svg_with_paths, zed_folder_icon, CardPageBlockColorValue, IconAsset, Theme,
};

pub(super) fn card_icons(theme: Theme, appearance_mode: AppearanceMode) -> CardIcons {
    let (page_toggle_collapsed_colors, page_toggle_expanded_colors) =
        page_toggle_color_icons(appearance_mode);
    CardIcons {
        card_action_edit: IconAsset::new(svg_with_paths(
            "0 0 16 16",
            &[
                "M11.243 3.457a.803.803 0 0 0-1.13 0l-.554.552a.075.075 0 0 0 0 .106l1.03 1.03a.075.075 0 0 0 .107 0l.547-.546a.1.1 0 0 0 .019-.032.804.804 0 0 0-.02-1.11m-2.246 1.22a.075.075 0 0 0-.106 0l-6.336 6.326a1.1 1.1 0 0 0-.237.393l-.27.87v.002c-.062.232.153.466.389.383l.863-.267q.221-.061.397-.239l6.332-6.331a.075.075 0 0 0 0-.106zm-3.355 6.898a.08.08 0 0 0-.053.022l-1.1 1.1a.075.075 0 0 0 .053.128h9.06a.625.625 0 1 0 0-1.25z",
            ],
            theme.text_muted,
        )),
        card_action_ellipsis: IconAsset::new(svg_with_paths(
            "0 0 16 16",
            &[
                "M3.2 6.725a1.275 1.275 0 1 0 0 2.55 1.275 1.275 0 0 0 0-2.55m4.8 0a1.275 1.275 0 1 0 0 2.55 1.275 1.275 0 0 0 0-2.55m4.8 0a1.275 1.275 0 1 0 0 2.55 1.275 1.275 0 0 0 0-2.55",
            ],
            theme.text_muted,
        )),
        close: IconAsset::new(svg_with_paths(
            "0 0 16 16",
            &[
                "M4.177 3.293a.625.625 0 0 0-.884.884L7.116 8l-3.823 3.823a.625.625 0 1 0 .884.884L8 8.884l3.823 3.823a.625.625 0 1 0 .884-.884L8.884 8l3.823-3.823a.625.625 0 0 0-.884-.884L8 7.116z",
            ],
            theme.text_muted,
        )),
        page: IconAsset::new(svg_with_paths(
            "4.12 2.37 11.75 15.25",
            &[
                "M13.3 14.25a.55.55 0 0 1-.55.55h-5.5a.55.55 0 1 1 0-1.1h5.5a.55.55 0 0 1 .55.55m-.55-1.95a.55.55 0 1 0 0-1.1h-5.5a.55.55 0 0 0 0 1.1z",
                "M6.25 2.375A2.125 2.125 0 0 0 4.125 4.5v11c0 1.174.951 2.125 2.125 2.125h7.5a2.125 2.125 0 0 0 2.125-2.125V8.121c0-.563-.224-1.104-.622-1.502L11.63 2.997a2.13 2.13 0 0 0-1.502-.622zM5.375 4.5c0-.483.392-.875.875-.875h3.7V6.25A2.05 2.05 0 0 0 12 8.3h2.625v7.2a.875.875 0 0 1-.875.875h-7.5a.875.875 0 0 1-.875-.875zm8.691 2.7H12a.95.95 0 0 1-.95-.95V4.184z",
            ],
            theme.text_secondary,
        )),
        page_toggle_collapsed: IconAsset::new(svg_with_group(
            "0 0 16 16",
            "rotate(-90 8 8)",
            &["M2.835 3.25a.8.8 0 0 0-.69 1.203l5.164 8.854a.8.8 0 0 0 1.382 0l5.165-8.854a.8.8 0 0 0-.691-1.203z"],
            theme.text_muted,
        )),
        page_toggle_collapsed_colors,
        page_toggle_expanded_colors,
        page_checkbox_checked: IconAsset::new(svg_with_paths(
            "0 0 16 16",
            &["M11.62 3.18a.876.876 0 0 1 1.5.9l-5.244 8.74a.876.876 0 0 1-1.414.12L2.966 8.86a.875.875 0 1 1 1.328-1.138L7 10.879z"],
            0xffffff,
        )),
        folder: IconAsset::new(zed_folder_icon(theme.text_secondary)),
    }
}

fn page_toggle_color_icons(appearance_mode: AppearanceMode) -> ([IconAsset; 10], [IconAsset; 10]) {
    let collapsed = CardPageBlockColorValue::ALL.map(|value| {
        IconAsset::new(svg_with_group(
            "0 0 16 16",
            "rotate(-90 8 8)",
            &["M2.835 3.25a.8.8 0 0 0-.69 1.203l5.164 8.854a.8.8 0 0 0 1.382 0l5.165-8.854a.8.8 0 0 0-.691-1.203z"],
            crate::ui::page_block_text_color_hex(value, appearance_mode),
        ))
    });
    let expanded = CardPageBlockColorValue::ALL.map(|value| {
        IconAsset::new(svg_with_paths(
            "0 0 16 16",
            &["M2.835 3.25a.8.8 0 0 0-.69 1.203l5.164 8.854a.8.8 0 0 0 1.382 0l5.165-8.854a.8.8 0 0 0-.691-1.203z"],
            crate::ui::page_block_text_color_hex(value, appearance_mode),
        ))
    });
    (collapsed, expanded)
}
