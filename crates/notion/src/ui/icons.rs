use self::assembly::assemble_icon_set;
use self::block_menu::block_menu_icons;
use self::card::card_icons;
use self::quick_find::quick_find_icons;
use self::sidebar::{sidebar_icons, SidebarIcons};
use self::topbar::topbar_icons;
use crate::ui::{CardPageBlockColor, CardPageBlockColorValue, CardPageProperty};
use app_model::AppearanceMode;

use crate::ui::{
    calendar_property_icon, lightning_icon, list_property_icon, person_property_icon,
    relation_property_icon, square_plus_icon, status_property_icon, svg_with_group, svg_with_paths,
    IconAsset, Theme,
};

mod assembly;
mod block_menu;
mod card;
mod quick_find;
mod sidebar;
mod topbar;

mod catalog;

use catalog::{toolbar_icons, view_icons};

pub(crate) struct IconSet {
    pub(crate) sidebar: SidebarIcons,
    pub(crate) topbar_menu: IconAsset,
    pub(crate) topbar_share: IconAsset,
    pub(crate) topbar_share_chevron: IconAsset,
    pub(crate) topbar_lock: IconAsset,
    pub(crate) topbar_chip_chevron: IconAsset,
    pub(crate) topbar_favorite_inactive: IconAsset,
    pub(crate) topbar_favorite_active: IconAsset,
    pub(crate) topbar_actions: IconAsset,
    pub(crate) topbar_copy_link: IconAsset,
    pub(crate) view_table_active: IconAsset,
    pub(crate) view_table_inactive: IconAsset,
    pub(crate) view_board_inactive: IconAsset,
    pub(crate) view_board_active: IconAsset,
    pub(crate) view_list_active: IconAsset,
    pub(crate) view_list_inactive: IconAsset,
    pub(crate) view_gallery_active: IconAsset,
    pub(crate) view_gallery_inactive: IconAsset,
    pub(crate) view_calendar_active: IconAsset,
    pub(crate) view_calendar_inactive: IconAsset,
    pub(crate) view_timeline_active: IconAsset,
    pub(crate) view_timeline_inactive: IconAsset,
    pub(crate) calendar_chevron_left: IconAsset,
    pub(crate) calendar_chevron_right: IconAsset,
    pub(crate) toolbar_minimize: IconAsset,
    pub(crate) toolbar_restore: IconAsset,
    pub(crate) toolbar_filter: IconAsset,
    pub(crate) toolbar_sort: IconAsset,
    pub(crate) toolbar_lightning: IconAsset,
    pub(crate) toolbar_magic_wand: IconAsset,
    pub(crate) ai_autofill_magic_wand: IconAsset,
    pub(crate) ai_autofill_external_link: IconAsset,
    pub(crate) ai_autofill_plus: IconAsset,
    pub(crate) ai_autofill_chevron: IconAsset,
    pub(crate) toolbar_search: IconAsset,
    pub(crate) toolbar_expand: IconAsset,
    pub(crate) toolbar_settings: IconAsset,
    pub(crate) new_button_chevron: IconAsset,
    pub(crate) card_action_edit: IconAsset,
    pub(crate) card_action_ellipsis: IconAsset,
    pub(crate) close: IconAsset,
    pub(crate) page: IconAsset,
    pub(crate) page_link_picker_random: IconAsset,
    pub(crate) page_link_picker_categories: [IconAsset; 11],
    pub(crate) page_toggle_collapsed: IconAsset,
    page_toggle_collapsed_colors: [IconAsset; 10],
    page_toggle_expanded_colors: [IconAsset; 10],
    pub(crate) page_checkbox_checked: IconAsset,
    pub(crate) page_block_add: IconAsset,
    pub(crate) page_block_handle: IconAsset,
    pub(crate) block_menu_turn_into: IconAsset,
    pub(crate) block_menu_color: IconAsset,
    pub(crate) block_menu_quote_size: IconAsset,
    pub(crate) block_menu_code_language: IconAsset,
    pub(crate) block_menu_code_wrap: IconAsset,
    pub(crate) block_menu_copy_code: IconAsset,
    pub(crate) code_action_copy: IconAsset,
    pub(crate) code_action_language_chevron: IconAsset,
    pub(crate) block_menu_copy_link: IconAsset,
    pub(crate) block_menu_duplicate: IconAsset,
    pub(crate) block_menu_move: IconAsset,
    pub(crate) block_menu_delete: IconAsset,
    pub(crate) block_menu_comment: IconAsset,
    pub(crate) block_menu_suggest_edits: IconAsset,
    pub(crate) block_menu_present: IconAsset,
    pub(crate) block_menu_ask_ai: IconAsset,
    pub(crate) block_menu_skills: IconAsset,
    pub(crate) block_menu_chevron: IconAsset,
    pub(crate) block_menu_checked: IconAsset,
    pub(crate) folder: IconAsset,
    pub(crate) property_status: IconAsset,
    pub(crate) property_person: IconAsset,
    pub(crate) property_relation: IconAsset,
    pub(crate) property_date: IconAsset,
    pub(crate) property_list: IconAsset,
    pub(crate) header_plus: IconAsset,
    pub(crate) quick_find_search: IconAsset,
    pub(crate) quick_find_person: IconAsset,
    pub(crate) quick_find_page: IconAsset,
    pub(crate) quick_find_database: IconAsset,
    pub(crate) quick_find_hide_highlight_pane: IconAsset,
    pub(crate) quick_find_hide_filters: IconAsset,
    pub(crate) quick_find_title_only: IconAsset,
    pub(crate) quick_find_title_only_active: IconAsset,
    pub(crate) quick_find_filter_chevron: IconAsset,
    pub(crate) quick_find_add_filter: IconAsset,
    pub(crate) quick_find_thumb_up: IconAsset,
    pub(crate) quick_find_thumb_down: IconAsset,
    pub(crate) quick_find_preview_link: IconAsset,
    pub(crate) quick_find_preview_open: IconAsset,
}

struct ViewIcons {
    view_table_active: IconAsset,
    view_table_inactive: IconAsset,
    view_board_inactive: IconAsset,
    view_board_active: IconAsset,
    view_list_active: IconAsset,
    view_list_inactive: IconAsset,
    view_gallery_active: IconAsset,
    view_gallery_inactive: IconAsset,
    view_calendar_active: IconAsset,
    view_calendar_inactive: IconAsset,
    view_timeline_active: IconAsset,
    view_timeline_inactive: IconAsset,
    calendar_chevron_left: IconAsset,
    calendar_chevron_right: IconAsset,
}

struct ToolbarIcons {
    toolbar_minimize: IconAsset,
    toolbar_restore: IconAsset,
    toolbar_filter: IconAsset,
    toolbar_sort: IconAsset,
    toolbar_lightning: IconAsset,
    toolbar_magic_wand: IconAsset,
    ai_autofill_magic_wand: IconAsset,
    ai_autofill_external_link: IconAsset,
    ai_autofill_plus: IconAsset,
    ai_autofill_chevron: IconAsset,
    toolbar_search: IconAsset,
    toolbar_expand: IconAsset,
    toolbar_settings: IconAsset,
    new_button_chevron: IconAsset,
}

struct CardIcons {
    card_action_edit: IconAsset,
    card_action_ellipsis: IconAsset,
    close: IconAsset,
    page: IconAsset,
    page_toggle_collapsed: IconAsset,
    page_toggle_collapsed_colors: [IconAsset; 10],
    page_toggle_expanded_colors: [IconAsset; 10],
    page_checkbox_checked: IconAsset,
    folder: IconAsset,
}

struct PropertyIcons {
    property_status: IconAsset,
    property_person: IconAsset,
    property_relation: IconAsset,
    property_date: IconAsset,
    property_list: IconAsset,
    header_plus: IconAsset,
}

impl IconSet {
    pub(crate) fn new(appearance_mode: AppearanceMode) -> Self {
        let theme = Theme::for_appearance_mode(appearance_mode);
        let sidebar = sidebar_icons(appearance_mode);
        let topbar = topbar_icons(theme);
        let views = view_icons(theme);
        let toolbar = toolbar_icons(theme);
        let card = card_icons(theme, appearance_mode);
        let block_menu = block_menu_icons(theme);
        let property = property_icons(theme);
        let quick_find = quick_find_icons(theme);
        let page_link_picker_categories = page_link_picker_category_icons(theme);
        let page_link_picker_random = IconAsset::new(svg_with_paths(
            "0 0 16 16",
            &["M11.982 2.526a.625.625 0 0 0-.884.884l.915.915H10.93a3.83 3.83 0 0 0-3.27 1.837l-.386.635-.388-.637a3.83 3.83 0 0 0-3.268-1.837H2.48a.625.625 0 0 0 0 1.25h1.14c.9 0 1.733.469 2.2 1.237L6.543 8 5.82 9.19a2.58 2.58 0 0 1-2.2 1.237H2.48a.625.625 0 1 0 0 1.25h1.14A3.83 3.83 0 0 0 6.887 9.84l.388-.638.386.636a3.83 3.83 0 0 0 3.268 1.837h1.085l-.916.915a.625.625 0 1 0 .884.884l1.98-1.98a.625.625 0 0 0 0-.884l-1.98-1.98a.625.625 0 0 0-.884.884l.911.91h-1.08a2.58 2.58 0 0 1-2.2-1.236L8.006 8l.723-1.188a2.58 2.58 0 0 1 2.2-1.237h1.08l-.91.911a.625.625 0 1 0 .883.884l1.98-1.98a.625.625 0 0 0 0-.884z"],
            theme.text_secondary,
        ));
        assemble_icon_set!(
            sidebar,
            topbar,
            views,
            toolbar,
            card,
            block_menu,
            property,
            quick_find,
            page_link_picker_random,
            page_link_picker_categories,
            theme,
        )
    }

    pub(crate) fn property_icon_for(&self, property: &CardPageProperty) -> Option<&IconAsset> {
        match property.property_type.as_str() {
            "status" => Some(&self.property_status),
            "person" => Some(&self.property_person),
            "relation" => Some(&self.property_relation),
            "date" => Some(&self.property_date),
            _ if property.label == "Schedule" => Some(&self.property_list),
            _ => None,
        }
    }

    pub(crate) fn page_toggle_marker(
        &self,
        expanded: bool,
        color: CardPageBlockColor,
    ) -> &IconAsset {
        let value = match color {
            CardPageBlockColor::Text(value) => value,
            CardPageBlockColor::Background(_) => CardPageBlockColorValue::Default,
        };
        if expanded {
            &self.page_toggle_expanded_colors[value.index()]
        } else {
            &self.page_toggle_collapsed_colors[value.index()]
        }
    }
}

fn page_link_picker_category_icons(theme: Theme) -> [IconAsset; 11] {
    const PATHS: [&[&str]; 11] = [
        &[
            "M10.625 5.725a.625.625 0 1 0-1.25 0v3.65H6.4a.625.625 0 1 0 0 1.25H10c.345 0 .625-.28.625-.625z",
            "M10 2.375a7.625 7.625 0 1 0 0 15.25 7.625 7.625 0 0 0 0-15.25M3.625 10a6.375 6.375 0 1 1 12.75 0 6.375 6.375 0 0 1-12.75 0",
        ],
        &[
            "M8.045 11.706a.625.625 0 0 0-1.036.698A3.6 3.6 0 0 0 10.005 14c1.245 0 2.35-.637 2.996-1.596a.625.625 0 0 0-1.036-.698 2.37 2.37 0 0 1-1.96 1.044 2.36 2.36 0 0 1-1.96-1.044m-.68-2.041c.49 0 .88-.46.88-1.02s-.39-1.02-.88-1.02-.88.46-.88 1.02.39 1.02.88 1.02m6.15-1.02c0 .56-.39 1.02-.88 1.02s-.88-.46-.88-1.02.39-1.02.88-1.02.88.46.88 1.02",
            "M10 2.375a7.625 7.625 0 1 0 0 15.25 7.625 7.625 0 0 0 0-15.25M3.625 10a6.375 6.375 0 1 1 12.75 0 6.375 6.375 0 0 1-12.75 0",
        ],
        &[
            "M2.545 3.31a.625.625 0 0 0-.625.625c0 5.882 4.312 10.763 9.765 10.763h3.581c.737.8 1.472 1.718 2.306 2.88a.624.624 0 1 0 1.012-.728c-.947-1.32-1.78-2.344-2.621-3.232C15.747 7.94 11.518 3.31 6.207 3.31zm12.024 8.956c-1.239-1.104-2.624-2.118-4.489-3.483l-1.023-.75a.62.62 0 0 0-.872.134.63.63 0 0 0 .137.875l1.013.742c2.007 1.47 3.42 2.505 4.68 3.664h-2.33c-4.46 0-8.208-3.871-8.497-8.888h3.019c4.102 0 7.602 3.276 8.362 7.706",
        ],
        &[
            "M11.375 5.413a4.5 4.5 0 0 1-.152-1.177c0-1.482.673-2.683 1.504-2.683.742 0 1.358.957 1.481 2.217.898-.62 1.846-.767 2.318-.296.471.472.324 1.42-.296 2.318 1.26.123 2.217.74 2.217 1.481 0 .83-1.201 1.504-2.683 1.504a4.5 4.5 0 0 1-1.177-.152 3.23 3.23 0 0 1-1.378 3.606l-9.233 5.853c-1.346.852-2.912-.714-2.06-2.06L7.77 6.791a3.23 3.23 0 0 1 3.606-1.378m.517 1.708a1.976 1.976 0 0 0-3.067.339l-.175.276 1.232 1.232a.55.55 0 1 1-.777.778L8.047 8.688l-.467.736 1.306 1.307a.55.55 0 1 1-.778.778l-1.132-1.132-4.004 6.317a.23.23 0 0 0-.042.16.26.26 0 0 0 .075.14.26.26 0 0 0 .141.076.23.23 0 0 0 .16-.042l3.221-2.042-.561-.561a.55.55 0 1 1 .778-.778l.735.736 5.06-3.208a1.976 1.976 0 0 0 .34-3.067z",
        ],
        &[
            "M10 1.875a8.125 8.125 0 1 0 0 16.25 8.125 8.125 0 0 0 0-16.25M4.896 5.393a6.9 6.9 0 0 1 2.298-1.671L9.45 5.36v1.35L7.04 8.46l-1.283-.417zm-1.733 5.334L5.417 9.09l1.284.417.92 2.833-.793 1.091H4.04a6.8 6.8 0 0 1-.878-2.703m5.416 6.001-.861-2.651.793-1.091h2.978l.793 1.09-.861 2.652a6.9 6.9 0 0 1-2.842 0m7.38-3.298h-2.787l-.793-1.091.92-2.833 1.284-.417 2.254 1.638a6.8 6.8 0 0 1-.878 2.703m-.855-8.037-.861 2.65-1.283.417-2.41-1.75V5.36l2.256-1.638c.879.393 1.66.966 2.298 1.671",
        ],
        &[
            "M5.29 3.366a.63.63 0 0 1 .54-.311h2.81c.182 0 .356.08.474.218l4.023 4.692h2.983c1.112 0 2.025.897 2.025 2.025v.02a2.03 2.03 0 0 1-2.025 2.025h-2.983l-4.023 4.692a.63.63 0 0 1-.474.218H5.83a.625.625 0 0 1-.542-.936l2.274-3.974H6.496l-1.949 1.358-.026.017c-.24.15-.533.255-.851.255h-.65a.625.625 0 0 1-.597-.81L3.306 10l-.883-2.855a.625.625 0 0 1 .597-.81h.65c.298 0 .61.079.883.276l1.943 1.354h1.066L5.288 3.99a.63.63 0 0 1 .001-.624m1.618.939 2.274 3.974a.625.625 0 0 1-.542.936H6.3a.63.63 0 0 1-.357-.112L3.896 7.677l.661 2.138a.63.63 0 0 1 0 .37l-.661 2.138 2.047-1.426a.63.63 0 0 1 .357-.112h2.34a.625.625 0 0 1 .542.935l-2.274 3.975h1.445l4.023-4.692a.63.63 0 0 1 .474-.218h3.27a.78.78 0 0 0 .775-.775v-.02a.773.773 0 0 0-.775-.775h-3.27a.63.63 0 0 1-.474-.218L8.353 4.305z",
        ],
        &[
            "M12.21 13.71c-.34 0-.62-.28-.62-.62 0-1.31.7-2.45 1.32-3.46.12-.2.24-.39.36-.58.34-.58.53-1.3.53-2.06 0-2.06-1.7-3.73-3.79-3.73S6.22 4.93 6.22 6.99c0 .76.18 1.47.53 2.06.11.19.23.39.35.58l.023.037c.614 1 1.307 2.129 1.297 3.423 0 .34-.24.61-.63.62-.35 0-.62-.28-.62-.63 0-.96-.55-1.85-1.14-2.8q-.091-.15-.185-.3-.095-.15-.185-.3c-.46-.78-.7-1.71-.7-2.69 0-2.74 2.26-4.98 5.04-4.98s5.04 2.23 5.04 4.98c0 .98-.24 1.92-.7 2.69-.12.2-.24.4-.37.61-.59.95-1.14 1.84-1.14 2.8 0 .34-.28.62-.62.63zm-1.26 4.28h-1.9c-.35 0-.62-.28-.62-.62s.28-.62.62-.62h1.9c.34 0 .62.28.62.62s-.28.62-.62.62M8.1 15.85h3.8c.34 0 .62-.28.62-.62s-.28-.62-.62-.62H8.1c-.34 0-.62.28-.62.62s.27.62.62.62",
        ],
        &[
            "M12.876 7.982a.625.625 0 1 0-1.072-.644L9.25 11.595 7.815 9.92a.625.625 0 0 0-.95.813l2 2.334a.625.625 0 0 0 1.01-.085z",
            "M10 2.375a7.625 7.625 0 1 0 0 15.25 7.625 7.625 0 0 0 0-15.25M3.625 10a6.375 6.375 0 1 1 12.75 0 6.375 6.375 0 0 1-12.75 0",
        ],
        &[
            "M10.282 3.66a5.39 5.39 0 0 0-5.217-.54l-.711.305a.63.63 0 0 0-.38.575v12.425a.625.625 0 1 0 1.25 0v-3.913l.333-.142a4.14 4.14 0 0 1 4.008.414 5.39 5.39 0 0 0 5.504.405l1.06-.53a.63.63 0 0 0 .346-.56V4a.625.625 0 0 0-.905-.558l-1.06.53c-1.36.68-2.983.56-4.228-.312m-5.057 7.495V4.412l.332-.142a4.14 4.14 0 0 1 4.008.413 5.39 5.39 0 0 0 5.504.406l.156-.078v6.703l-.715.357c-1.36.68-2.983.56-4.228-.312a5.4 5.4 0 0 0-5.057-.604",
        ],
        &[
            "M4 3.375c-.897 0-1.625.728-1.625 1.625v2.5c0 .897.728 1.625 1.625 1.625h3.5c.897 0 1.625-.728 1.625-1.625V5c0-.897-.728-1.625-1.625-1.625zM3.625 5c0-.207.168-.375.375-.375h3.5c.207 0 .375.168.375.375v2.5a.375.375 0 0 1-.375.375H4a.375.375 0 0 1-.375-.375zM4 10.875c-.897 0-1.625.727-1.625 1.625V15c0 .898.728 1.625 1.625 1.625h3.5c.897 0 1.625-.727 1.625-1.625v-2.5c0-.898-.728-1.625-1.625-1.625zM3.625 12.5c0-.207.168-.375.375-.375h3.5c.207 0 .375.168.375.375V15a.375.375 0 0 1-.375.375H4A.375.375 0 0 1 3.625 15zm7.25-7.5c0-.897.727-1.625 1.625-1.625H16c.898 0 1.625.728 1.625 1.625v2.5c0 .897-.727 1.625-1.625 1.625h-3.5A1.625 1.625 0 0 1 10.875 7.5zm1.625-.375a.375.375 0 0 0-.375.375v2.5c0 .207.168.375.375.375H16a.375.375 0 0 0 .375-.375V5A.375.375 0 0 0 16 4.625zm0 6.25c-.898 0-1.625.727-1.625 1.625V15c0 .898.727 1.625 1.625 1.625H16c.898 0 1.625-.727 1.625-1.625v-2.5c0-.898-.727-1.625-1.625-1.625zm-.375 1.625c0-.207.168-.375.375-.375H16c.207 0 .375.168.375.375V15a.375.375 0 0 1-.375.375h-3.5a.375.375 0 0 1-.375-.375z",
        ],
        &[
            "M10 2.375a7.625 7.625 0 1 0 0 15.25 7.625 7.625 0 0 0 0-15.25m0 4c.345 0 .625.28.625.625v2.375H13a.625.625 0 1 1 0 1.25h-2.375V13a.625.625 0 1 1-1.25 0v-2.375H7a.625.625 0 1 1 0-1.25h2.375V7c0-.345.28-.625.625-.625",
        ],
    ];

    PATHS.map(|paths| IconAsset::new(svg_with_paths("0 0 20 20", paths, theme.text_secondary)))
}

fn property_icons(theme: Theme) -> PropertyIcons {
    PropertyIcons {
        property_status: IconAsset::new(status_property_icon(theme.text_secondary)),
        property_person: IconAsset::new(person_property_icon(theme.text_secondary)),
        property_relation: IconAsset::new(relation_property_icon(theme.text_secondary)),
        property_date: IconAsset::new(calendar_property_icon(theme.text_secondary)),
        property_list: IconAsset::new(list_property_icon(theme.text_secondary)),
        header_plus: IconAsset::new(square_plus_icon(theme.text_muted)),
    }
}
