use app_model::AppearanceMode;

use crate::ui::{lightning_icon, svg_from_body, svg_with_paths, IconAsset};

const SERVICE_COUNTER_PATH: &str = concat!(
    "M4.063 3.75c0-.966.596-1.562 1.562-1.562s1.563.596 1.563 1.562",
    "-.597 1.563-1.563 1.563-1.562-.597-1.562-1.563M15 5.313c.966 0 1.563-.597",
    " 1.563-1.563S15.966 2.188 15 2.188s-1.562.596-1.562 1.562.596 1.563 1.562 1.563",
    "m3.438 8.437H16.25v3.75h-2.5v-3.75h-2.187l1.953-3.906-.55-1.103-1.253 2.509H.625",
    "V10h.587l.838-1.678C2.76 6.903 3.644 6.25 5.622 6.25s2.862.653 3.572 2.072L10.03 10",
    "h.55l.838-1.678c.71-1.419 1.593-2.072 3.572-2.072 1.978 0 2.862.653 3.572 2.072",
    "l1.115 2.228-1.397.7-1.253-2.51-.55 1.104 1.953 3.906zM4.063 10V8.438c-.238 0-.375",
    ".109-.494.346L2.959 10zm4.228 0-.61-1.216c-.119-.237-.256-.347-.494-.347V10z",
);
const GOVERNMENT_PATH: &str = concat!(
    "M17.5 10.625v-2.5h-1.978c-.431-2.537-2.213-4.137-4.897-4.35v-.65l2.5-1.25V.625",
    "h-3.75v3.15c-2.684.212-4.466 1.813-4.897 4.35H2.5v2.5h1.875v3.75H2.5v2.5h15v-2.5",
    "h-1.875v-3.75zm-10.625 0H8.75v3.75H6.875zm6.25 3.75H11.25v-3.75h1.875z",
);
const HOME_PATH: &str = "M9.08 3.341a1.625 1.625 0 0 1 1.84 0l5.875 4.035c.441.304.705.805.705 1.34v6.034a2.125 2.125 0 0 1-2.125 2.125h-2.716a1.625 1.625 0 0 1-1.625-1.625v-4.065H8.967v4.065c0 .898-.728 1.625-1.625 1.625H4.625A2.125 2.125 0 0 1 2.5 14.75V8.716c0-.535.264-1.036.705-1.34zm1.132 1.03a.375.375 0 0 0-.424 0L3.913 8.407a.38.38 0 0 0-.163.309v6.034c0 .483.392.875.875.875h2.716a.375.375 0 0 0 .375-.375v-4.19c0-.621.503-1.125 1.125-1.125h2.319c.62 0 1.124.504 1.124 1.125v4.19c0 .207.168.375.375.375h2.716a.875.875 0 0 0 .875-.875V8.716c0-.124-.06-.24-.163-.31z";
const CHAT_PATH: &str = "M16.938 9.353c0-2.97-2.539-5.54-6.545-5.697L10 3.648c-4.232 0-6.938 2.639-6.938 5.705 0 1.438.583 2.752 1.617 3.76a.63.63 0 0 1 .18.546 7.3 7.3 0 0 1-.89 2.528c1.108-.13 2.12-.614 3.01-1.344l.063-.044a.63.63 0 0 1 .505-.073 9 9 0 0 0 2.454.333l.392-.007c4.006-.158 6.545-2.728 6.545-5.699m1.25 0c0 3.803-3.234 6.766-7.747 6.948l-.44.008a10.2 10.2 0 0 1-2.485-.299c-1.349 1.022-2.985 1.62-4.826 1.428a.625.625 0 0 1-.406-1.033c.712-.817 1.096-1.737 1.284-2.642-1.116-1.197-1.756-2.733-1.756-4.41 0-3.925 3.447-6.955 8.189-6.955l.44.009c4.512.181 7.747 3.143 7.747 6.946";
const INBOX_PATH: &str = "M6.303 3.625c-.71 0-1.374.355-1.768.946L2.232 8.025c-.233.35-.357.76-.357 1.18v5.045c0 1.174.951 2.125 2.125 2.125h12a2.125 2.125 0 0 0 2.125-2.125V9.204c0-.42-.124-.83-.357-1.179l-2.303-3.454a2.13 2.13 0 0 0-1.768-.946zm-.728 1.64a.88.88 0 0 1 .728-.39h7.394c.293 0 .566.146.728.39l2.303 3.454a1 1 0 0 1 .083.156h-4.702a.625.625 0 0 0-.625.625v.476a1.484 1.484 0 0 1-2.968 0V9.5a.625.625 0 0 0-.625-.625H3.189a1 1 0 0 1 .083-.156zm-2.45 4.86H7.27a2.734 2.734 0 0 0 5.46 0h4.145v4.125a.875.875 0 0 1-.875.875H4a.875.875 0 0 1-.875-.875z";
const SEARCH_PATH: &str = "M8.875 2.625a6.25 6.25 0 1 0 3.955 11.09l3.983 3.982a.625.625 0 1 0 .884-.884l-3.983-3.982a6.25 6.25 0 0 0-4.84-10.205m-5 6.25a5 5 0 1 1 10 0 5 5 0 0 1-10 0";
const NEW_CHAT_PATHS: [&str; 2] = [
    "M12.758 9.976a1.178 1.178 0 1 0 .377-2.326 1.178 1.178 0 0 0-.377 2.326M6.547 8.97a1.178 1.178 0 1 0 .377-2.327 1.178 1.178 0 0 0-.377 2.326",
    "M10.573 5.554a3.917 3.917 0 0 1 6.743.035.625.625 0 1 1-1.08.63 2.667 2.667 0 0 0-4.591-.023l-5.398 9.015 4.192.68a.625.625 0 0 1-.2 1.233l-5.102-.827a.625.625 0 0 1-.436-.938zM4.36 3.517a3.92 3.92 0 0 1 5.572.356.625.625 0 1 1-.945.818 2.67 2.67 0 0 0-3.795-.243.625.625 0 1 1-.833-.931",
];
const NEW_PAGE_PATHS: [&str; 2] = [
    "m16.774 4.341-.59.589-1.109-1.11.596-.594a.784.784 0 0 1 1.103 0c.302.302.302.8 0 1.102zM8.65 12.462l6.816-6.813-1.11-1.11-6.822 6.808a1.1 1.1 0 0 0-.236.393l-.289.932c-.052.196.131.38.315.314l.932-.288a.9.9 0 0 0 .394-.236",
    "M4.375 6.25c0-1.036.84-1.875 1.875-1.875H11a.625.625 0 1 0 0-1.25H6.25A3.125 3.125 0 0 0 3.125 6.25v7.5c0 1.726 1.4 3.125 3.125 3.125h7.5c1.726 0 3.125-1.4 3.125-3.125V9a.625.625 0 1 0-1.25 0v4.75c0 1.036-.84 1.875-1.875 1.875h-7.5a1.875 1.875 0 0 1-1.875-1.875z",
];
const MORE_PATH: &str = "M3.2 6.725a1.275 1.275 0 1 0 0 2.55 1.275 1.275 0 0 0 0-2.55m4.8 0a1.275 1.275 0 1 0 0 2.55 1.275 1.275 0 0 0 0-2.55m4.8 0a1.275 1.275 0 1 0 0 2.55 1.275 1.275 0 0 0 0-2.55";
const ADD_PATH: &str = "M8 2.74a.66.66 0 0 1 .66.66v3.94h3.94a.66.66 0 0 1 0 1.32H8.66v3.94a.66.66 0 0 1-1.32 0V8.66H3.4a.66.66 0 0 1 0-1.32h3.94V3.4A.66.66 0 0 1 8 2.74";
const CHEVRON_PATH: &str = "M7.47 10.93a.75.75 0 0 0 1.06 0l4.32-4.32a.75.75 0 1 0-1.06-1.06L8 9.34 4.21 5.55a.75.75 0 0 0-1.06 1.06z";

pub(crate) struct SidebarIcons {
    pub(crate) search: IconAsset,
    pub(crate) home: IconAsset,
    pub(crate) chat: IconAsset,
    pub(crate) inbox: IconAsset,
    pub(crate) new_chat: IconAsset,
    pub(crate) new_page: IconAsset,
    pub(crate) notion_ai: IconAsset,
    pub(crate) service_counter: IconAsset,
    pub(crate) government: IconAsset,
    pub(crate) more: IconAsset,
    pub(crate) add: IconAsset,
    pub(crate) chevron_expanded: IconAsset,
    pub(crate) chevron_collapsed: IconAsset,
}

pub(super) fn sidebar_icons(appearance_mode: AppearanceMode) -> SidebarIcons {
    let (builtin_fill, muted_fill, row_fill, asset_fill) = match appearance_mode {
        AppearanceMode::Light => (0x5f5e59, 0x9b9a97, 0x5f5e59, 0xa6a299),
        AppearanceMode::Dark => (0x8a8884, 0x8a8884, 0xbbbab6, 0x7f7f7f),
    };
    SidebarIcons {
        search: IconAsset::new(svg_with_paths("0 0 20 20", &[SEARCH_PATH], builtin_fill)),
        home: IconAsset::new(svg_with_paths("0 0 20 20", &[HOME_PATH], builtin_fill)),
        chat: IconAsset::new(svg_with_paths("0 0 20 20", &[CHAT_PATH], builtin_fill)),
        inbox: IconAsset::new(svg_with_paths("0 0 20 20", &[INBOX_PATH], builtin_fill)),
        new_chat: IconAsset::new(svg_with_paths("0 0 20 20", &NEW_CHAT_PATHS, builtin_fill)),
        new_page: IconAsset::new(svg_with_paths("0 0 20 20", &NEW_PAGE_PATHS, builtin_fill)),
        notion_ai: IconAsset::new(lightning_icon(muted_fill)),
        service_counter: IconAsset::new(svg_with_paths(
            "0 0 20 20",
            &[SERVICE_COUNTER_PATH],
            asset_fill,
        )),
        government: IconAsset::new(svg_with_paths("0 0 20 20", &[GOVERNMENT_PATH], asset_fill)),
        more: IconAsset::new(svg_with_paths("0 0 16 16", &[MORE_PATH], row_fill)),
        add: IconAsset::new(svg_with_paths("0 0 16 16", &[ADD_PATH], row_fill)),
        chevron_expanded: IconAsset::new(chevron_icon(row_fill, false)),
        chevron_collapsed: IconAsset::new(chevron_icon(row_fill, true)),
    }
}

fn chevron_icon(fill: u32, collapsed: bool) -> crate::ui::Arc<crate::ui::Image> {
    let transform = if collapsed {
        " transform=\"rotate(-90 8 8)\""
    } else {
        ""
    };
    svg_from_body(
        "0 0 16 16",
        format!("<path fill=\"#{fill:06x}\"{transform} d=\"{CHEVRON_PATH}\"/>"),
    )
}
