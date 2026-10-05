use super::{
    lightning_icon, svg_with_group, svg_with_paths, IconAsset, Theme, ToolbarIcons, ViewIcons,
};

pub(super) fn view_icons(theme: Theme) -> ViewIcons {
    let (view_table_active, view_table_inactive) = table_icons(theme);
    let (view_board_active, view_board_inactive) = board_icons(theme);
    let (view_list_active, view_list_inactive) = list_icons(theme);
    let (view_gallery_active, view_gallery_inactive) = gallery_icons(theme);
    let (view_calendar_active, view_calendar_inactive) = calendar_icons(theme);
    let (view_timeline_active, view_timeline_inactive) = timeline_icons(theme);
    let (calendar_chevron_left, calendar_chevron_right) = calendar_chevrons(theme);
    ViewIcons {
        view_table_active,
        view_table_inactive,
        view_board_inactive,
        view_board_active,
        view_list_active,
        view_list_inactive,
        view_gallery_active,
        view_gallery_inactive,
        view_calendar_active,
        view_calendar_inactive,
        view_timeline_active,
        view_timeline_inactive,
        calendar_chevron_left,
        calendar_chevron_right,
    }
}

fn list_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let paths = [
        "M4.5 4.125A2.125 2.125 0 0 0 2.375 6.25v7.5c0 1.174.951 2.125 2.125 2.125h11a2.125 2.125 0 0 0 2.125-2.125v-7.5A2.125 2.125 0 0 0 15.5 4.125zM3.625 6.25c0-.483.392-.875.875-.875h11c.483 0 .875.392.875.875v7.5a.875.875 0 0 1-.875.875h-11a.875.875 0 0 1-.875-.875z",
        "M5.2 7.15a.65.65 0 1 0 0 1.3.65.65 0 0 0 0-1.3m2.05.025a.625.625 0 1 0 0 1.25h6.6a.625.625 0 1 0 0-1.25zM5.2 9.35a.65.65 0 1 0 0 1.3.65.65 0 0 0 0-1.3m2.05.025a.625.625 0 1 0 0 1.25h6.6a.625.625 0 1 0 0-1.25zM5.2 11.55a.65.65 0 1 0 0 1.3.65.65 0 0 0 0-1.3m2.05.025a.625.625 0 1 0 0 1.25h6.6a.625.625 0 1 0 0-1.25z",
    ];
    (
        IconAsset::new(svg_with_paths("0 0 20 20", &paths, theme.text_primary)),
        IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &paths,
            theme.view_tab_icon_inactive,
        )),
    )
}

fn gallery_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let paths = [
        "M4.5 4.125A2.125 2.125 0 0 0 2.375 6.25v7.5c0 1.174.951 2.125 2.125 2.125h11a2.125 2.125 0 0 0 2.125-2.125v-7.5A2.125 2.125 0 0 0 15.5 4.125zM3.625 6.25c0-.483.392-.875.875-.875h11c.483 0 .875.392.875.875v7.5a.875.875 0 0 1-.875.875h-11a.875.875 0 0 1-.875-.875z",
        "M5.2 6.75h3.35c.345 0 .625.28.625.625v2.05c0 .345-.28.625-.625.625H5.2a.625.625 0 0 1-.625-.625v-2.05c0-.345.28-.625.625-.625m6.25 0h3.35c.345 0 .625.28.625.625v2.05c0 .345-.28.625-.625.625h-3.35a.625.625 0 0 1-.625-.625v-2.05c0-.345.28-.625.625-.625M5.2 11h3.35c.345 0 .625.28.625.625v1c0 .345-.28.625-.625.625H5.2a.625.625 0 0 1-.625-.625v-1c0-.345.28-.625.625-.625m6.25 0h3.35c.345 0 .625.28.625.625v1c0 .345-.28.625-.625.625h-3.35a.625.625 0 0 1-.625-.625v-1c0-.345.28-.625.625-.625",
    ];
    (
        IconAsset::new(svg_with_paths("0 0 20 20", &paths, theme.text_primary)),
        IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &paths,
            theme.view_tab_icon_inactive,
        )),
    )
}

fn table_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let path = "M4.5 4.125A2.125 2.125 0 0 0 2.375 6.25v7.5c0 1.174.951 2.125 2.125 2.125h11a2.125 2.125 0 0 0 2.125-2.125v-7.5A2.125 2.125 0 0 0 15.5 4.125zm11.875 7h-5.75v-2.25h5.75zm-5.75 1.25h5.75v1.375a.875.875 0 0 1-.875.875h-4.875zm-1.25-1.25h-5.75v-2.25h5.75zm-5.75 1.25h5.75v2.25H4.5a.875.875 0 0 1-.875-.875zm0-4.75V6.25c0-.483.392-.875.875-.875h4.875v2.25zm7 0v-2.25H15.5c.483 0 .875.392.875.875v1.375z";
    (
        IconAsset::new(svg_with_paths("0 0 20 20", &[path], theme.text_primary)),
        IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &[path],
            theme.view_tab_icon_inactive,
        )),
    )
}

fn board_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let path = "M2.375 6.25c0-1.174.951-2.125 2.125-2.125h11c1.174 0 2.125.951 2.125 2.125v7.5a2.125 2.125 0 0 1-2.125 2.125h-11a2.125 2.125 0 0 1-2.125-2.125zm10.584 8.375H15.5a.875.875 0 0 0 .875-.875v-7.5a.875.875 0 0 0-.875-.875h-2.541zm-1.25-9.25H8.292v9.25h3.417zm-7.209 0a.875.875 0 0 0-.875.875v7.5c0 .483.392.875.875.875h2.542v-9.25z";
    (
        IconAsset::new(svg_with_paths("0 0 20 20", &[path], theme.text_primary)),
        IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &[path],
            theme.view_tab_icon_inactive,
        )),
    )
}

fn calendar_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let paths = [
        "M9.537 8.843a.694.694 0 1 1-1.39 0 .694.694 0 0 1 1.39 0m-.695 3.009a.694.694 0 1 0 0-1.389.694.694 0 0 0 0 1.389m.695 1.62a.695.695 0 1 1-1.39 0 .695.695 0 0 1 1.39 0m1.62-3.935a.694.694 0 1 0 0-1.389.694.694 0 0 0 0 1.389m.695 1.621a.694.694 0 1 1-1.39 0 .694.694 0 0 1 1.39 0m-.695 3.009a.695.695 0 1 0 0-1.39.695.695 0 0 0 0 1.39m3.01-5.324a.694.694 0 1 1-1.39 0 .694.694 0 0 1 1.39 0m-7.639 3.009a.694.694 0 1 0 0-1.389.694.694 0 0 0 0 1.389m.694 1.62a.695.695 0 1 1-1.389 0 .695.695 0 0 1 1.39 0m6.249-1.62a.694.694 0 1 0 0-1.389.694.694 0 0 0 0 1.389",
        "M5.25 3.125A2.125 2.125 0 0 0 3.125 5.25v9.5c0 1.174.951 2.125 2.125 2.125h9.5a2.125 2.125 0 0 0 2.125-2.125v-9.5a2.125 2.125 0 0 0-2.125-2.125zm-.875 3.69h11.25v7.935a.875.875 0 0 1-.875.875h-9.5a.875.875 0 0 1-.875-.875z",
    ];
    (
        IconAsset::new(svg_with_paths("0 0 20 20", &paths, theme.text_primary)),
        IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &paths,
            theme.view_tab_icon_inactive,
        )),
    )
}

fn timeline_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let paths = [
        "M5.5 6.975a.55.55 0 1 0 0 1.1h5.85a.55.55 0 0 0 0-1.1zM6.525 10a.55.55 0 0 1 .55-.55h5.85a.55.55 0 1 1 0 1.1h-5.85a.55.55 0 0 1-.55-.55m1.9 1.925a.55.55 0 1 0 0 1.1h5.85a.55.55 0 0 0 0-1.1z",
        "M4.5 4.125A2.125 2.125 0 0 0 2.375 6.25v7.5c0 1.174.951 2.125 2.125 2.125h11a2.125 2.125 0 0 0 2.125-2.125v-7.5A2.125 2.125 0 0 0 15.5 4.125zM3.625 6.25c0-.483.392-.875.875-.875h11c.483 0 .875.392.875.875v7.5a.875.875 0 0 1-.875.875h-11a.875.875 0 0 1-.875-.875z",
    ];
    (
        IconAsset::new(svg_with_paths("0 0 20 20", &paths, theme.text_primary)),
        IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &paths,
            theme.view_tab_icon_inactive,
        )),
    )
}

fn calendar_chevrons(theme: Theme) -> (IconAsset, IconAsset) {
    let path = "M10.202 3.238a.625.625 0 0 0-.884 0l-4.32 4.32a.625.625 0 0 0 0 .884l4.32 4.32a.625.625 0 0 0 .884-.884L6.324 8l3.878-3.878a.625.625 0 0 0 0-.884";
    (
        IconAsset::new(svg_with_paths("0 0 16 16", &[path], theme.text_muted)),
        IconAsset::new(svg_with_group(
            "0 0 16 16",
            "rotate(180 8 8)",
            &[path],
            theme.text_muted,
        )),
    )
}

pub(super) fn toolbar_icons(theme: Theme) -> ToolbarIcons {
    let (toolbar_minimize, toolbar_restore, toolbar_filter, toolbar_sort) = toolbar_core(theme);
    let (toolbar_magic_wand, ai_autofill_magic_wand, ai_autofill_external_link) = ai_icons(theme);
    let (ai_autofill_plus, ai_autofill_chevron) = ai_action_icons(theme);
    let (toolbar_search, toolbar_expand, toolbar_settings, new_button_chevron) =
        toolbar_actions(theme);
    ToolbarIcons {
        toolbar_minimize,
        toolbar_restore,
        toolbar_filter,
        toolbar_sort,
        toolbar_lightning: IconAsset::new(lightning_icon(theme.text_muted)),
        toolbar_magic_wand,
        ai_autofill_magic_wand,
        ai_autofill_external_link,
        ai_autofill_plus,
        ai_autofill_chevron,
        toolbar_search,
        toolbar_expand,
        toolbar_settings,
        new_button_chevron,
    }
}

fn toolbar_core(theme: Theme) -> (IconAsset, IconAsset, IconAsset, IconAsset) {
    let minimize = [
        "M4.482 3.238a.625.625 0 1 0-.884.884L7.476 8l-3.878 3.878a.625.625 0 0 0 .884.884l4.32-4.32a.625.625 0 0 0 0-.884z",
        "M8.882 3.238a.625.625 0 0 0-.884.884L11.876 8l-3.878 3.878a.625.625 0 0 0 .884.884l4.32-4.32a.625.625 0 0 0 0-.884z",
    ];
    let restore = [
        "M8.002 3.238a.625.625 0 0 0-.884 0l-4.32 4.32a.625.625 0 0 0 0 .884l4.32 4.32a.625.625 0 0 0 .884-.884L4.124 8l3.878-3.878a.625.625 0 0 0 0-.884",
        "M12.402 3.238a.625.625 0 0 0-.884 0l-4.32 4.32a.625.625 0 0 0 0 .884l4.32 4.32a.625.625 0 0 0 .884-.884L8.524 8l3.878-3.878a.625.625 0 0 0 0-.884",
    ];
    let filter = "M2.4 3.7a.7.7 0 1 0 0 1.4h11.2a.7.7 0 1 0 0-1.4zm9.5 3.594H4.1a.7.7 0 1 0 0 1.4h7.8a.7.7 0 1 0 0-1.4M5.8 10.9a.7.7 0 1 0 0 1.4h4.4a.7.7 0 1 0 0-1.4z";
    let sort = "M11.349 2.672a.625.625 0 0 0-.884 0L7.666 5.471a.625.625 0 1 0 .884.883l1.732-1.73v8.262a.625.625 0 1 0 1.25 0V4.623l1.73 1.731a.625.625 0 0 0 .885-.883zM5.093 2.49a.625.625 0 0 0-.625.624v8.263l-1.73-1.731a.625.625 0 1 0-.885.883l2.798 2.799c.244.244.64.244.884 0l2.798-2.798a.625.625 0 0 0-.883-.884l-1.732 1.73V3.115a.625.625 0 0 0-.625-.625";
    (
        IconAsset::new(svg_with_paths("0 0 16 16", &minimize, theme.text_muted)),
        IconAsset::new(svg_with_paths("0 0 16 16", &restore, theme.text_muted)),
        IconAsset::new(svg_with_paths("0 0 16 16", &[filter], theme.text_muted)),
        IconAsset::new(svg_with_group(
            "0 0 16 16",
            "rotate(180 8 8)",
            &[sort],
            theme.text_muted,
        )),
    )
}

fn ai_icons(theme: Theme) -> (IconAsset, IconAsset, IconAsset) {
    let wand = "M8 1.825a.575.575 0 0 0-.575.575V4a.575.575 0 0 0 1.15 0V2.4A.575.575 0 0 0 8 1.825m4.367 1.809a.575.575 0 0 0-.814 0l-1.131 1.131a.575.575 0 1 0 .813.813l1.132-1.131a.575.575 0 0 0 0-.813m-7.92 0a.575.575 0 0 0-.813.813l1.13 1.131a.575.575 0 1 0 .814-.813zm3.248 3.071a.7.7 0 1 0-.99.99l.77.77.99-.99zm4.305.72a.575.575 0 0 0 0 1.15h1.6a.575.575 0 1 0 0-1.15zm-9.6 0a.575.575 0 1 0 0 1.15H4a.575.575 0 0 0 0-1.15zm11.895 5.88-5.37-5.37-.99.99 5.37 5.37a.7.7 0 0 0 .99-.99m-8.717-2.883a.575.575 0 0 0-.813 0l-1.131 1.131a.575.575 0 1 0 .813.813l1.131-1.131a.575.575 0 0 0 0-.813M8 11.425a.575.575 0 0 0-.575.575v1.6a.575.575 0 1 0 1.15 0V12A.575.575 0 0 0 8 11.425";
    let autofill = "M10.75 3a.75.75 0 0 0-1.5 0v2a.75.75 0 0 0 1.5 0zM8.293 8.293a1 1 0 0 1 1.414 0l.897.896-1.372 1.372-.04.045-.9-.899a1 1 0 0 1 0-1.414m1.602 3.015 6.649 6.65a1 1 0 0 0 1.414-1.415l-6.646-6.647-1.372 1.372zM10.75 15a.75.75 0 0 0-1.5 0v2a.75.75 0 0 0 1.5 0zm-3.755-1.995a.75.75 0 0 1 0 1.06L5.58 15.48a.75.75 0 1 1-1.06-1.06l1.413-1.415a.75.75 0 0 1 1.06 0M15.48 4.52a.75.75 0 0 1 0 1.06l-1.414 1.415a.75.75 0 1 1-1.06-1.06l1.413-1.415a.75.75 0 0 1 1.061 0M5.75 10a.75.75 0 0 1-.75.75H3a.75.75 0 0 1 0-1.5h2a.75.75 0 0 1 .75.75m12 0a.75.75 0 0 1-.75.75h-2a.75.75 0 0 1 0-1.5h2a.75.75 0 0 1 .75.75M6.995 6.995a.75.75 0 0 1-1.06 0L4.52 5.58a.75.75 0 0 1 1.06-1.06l1.415 1.414a.75.75 0 0 1 0 1.06";
    let external = "M5.603 3.663a.625.625 0 1 0 0 1.25h4.6l-6.37 6.371a.615.615 0 0 0 .013.87.616.616 0 0 0 .87.014l6.371-6.372v4.601a.625.625 0 1 0 1.25 0v-6.11a.625.625 0 0 0-.625-.624z";
    (
        IconAsset::new(svg_with_paths("0 0 16 16", &[wand], theme.text_muted)),
        IconAsset::new(svg_with_paths("0 0 20 20", &[autofill], theme.text_primary)),
        IconAsset::new(svg_with_paths("0 0 16 16", &[external], theme.text_muted)),
    )
}

fn ai_action_icons(theme: Theme) -> (IconAsset, IconAsset) {
    let plus = "M8 2.74a.66.66 0 0 1 .66.66v3.94h3.94a.66.66 0 0 1 0 1.32H8.66v3.94a.66.66 0 0 1-1.32 0V8.66H3.4a.66.66 0 0 1 0-1.32h3.94V3.4A.66.66 0 0 1 8 2.74";
    let chevron = "M6.722 3.238a.625.625 0 1 0-.884.884L9.716 8l-3.878 3.878a.625.625 0 0 0 .884.884l4.32-4.32a.625.625 0 0 0 0-.884z";
    (
        IconAsset::new(svg_with_paths("0 0 16 16", &[plus], 0xf3f9fd)),
        IconAsset::new(svg_with_paths("0 0 16 16", &[chevron], theme.text_muted)),
    )
}

fn toolbar_actions(theme: Theme) -> (IconAsset, IconAsset, IconAsset, IconAsset) {
    let search = "M7.1 1.975a5.125 5.125 0 1 0 3.155 9.164l3.107 3.107a.625.625 0 1 0 .884-.884l-3.107-3.107A5.125 5.125 0 0 0 7.1 1.975M3.225 7.1a3.875 3.875 0 1 1 7.75 0 3.875 3.875 0 0 1-7.75 0";
    let expand = "M8.912 2.463a.625.625 0 1 0 0 1.25h2.491L8.358 6.758a.625.625 0 1 0 .884.884l3.045-3.045v2.49a.625.625 0 1 0 1.25 0v-4a.625.625 0 0 0-.625-.624zM7.64 8.36a.625.625 0 0 0-.885 0l-3.042 3.043V8.912a.625.625 0 1 0-1.25 0v4c0 .345.28.625.625.625h4a.625.625 0 0 0 0-1.25H4.597l3.042-3.043a.625.625 0 0 0 0-.883";
    let settings = "M2.25 5.531h5.692a2.126 2.126 0 0 0 4.116 0h1.692a.625.625 0 1 0 0-1.25H12a2.126 2.126 0 0 0-4 0H2.25a.625.625 0 1 0 0 1.25M10 4.125a.875.875 0 1 1 0 1.75.875.875 0 0 1 0-1.75m-4 9c.921 0 1.706-.586 2-1.406h5.75a.625.625 0 0 0 0-1.25H8.058a2.126 2.126 0 0 0-4.116 0H2.25a.625.625 0 1 0 0 1.25H4a2.13 2.13 0 0 0 2 1.406m0-1.25a.875.875 0 1 1 0-1.75.875.875 0 0 1 0 1.75";
    let chevron = "M9.38 13.619a.875.875 0 0 0 1.238 0l5.4-5.4a.875.875 0 0 0-1.237-1.238L10 11.763 5.218 6.98a.875.875 0 1 0-1.237 1.24z";
    (
        IconAsset::new(svg_with_paths("0 0 16 16", &[search], theme.text_muted)),
        IconAsset::new(svg_with_paths("0 0 16 16", &[expand], theme.text_muted)),
        IconAsset::new(svg_with_paths("0 0 16 16", &[settings], theme.text_muted)),
        IconAsset::new(svg_with_paths("0 0 20 20", &[chevron], 0xf3f9fd)),
    )
}
