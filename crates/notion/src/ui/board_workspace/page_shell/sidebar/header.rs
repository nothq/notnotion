use super::{
    div, px, Div, NotionSidebarRow, NotionSidebarRowAction, NotionSidebarRowLayout,
    NotionSidebarTab, PageShellIcon, PageShellLink, PageShellNodeIdentity, PageShellSnapshot,
    ParentElement, SidebarRenderer, Styled, SIDEBAR_HEADER_HEIGHT,
};
use crate::ui::surface::NotionSidebarState;
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_header(
        &self,
        state: &NotionSidebarState,
        page_shell: &PageShellSnapshot,
        search_open: bool,
        cx: &mut App,
    ) -> Div {
        let tab_rows = page_shell
            .builtin_links
            .iter()
            .enumerate()
            .filter_map(|(index, link)| state.top_tab_row(link, index))
            .map(|row| self.render_notion_sidebar_row(&row, cx))
            .collect::<Vec<_>>();
        div()
            .h(px(SIDEBAR_HEADER_HEIGHT))
            .flex_none()
            .pl(px(8.0))
            .pr(px(12.0))
            .py(px(8.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(self.render_notion_sidebar_tab_list(tab_rows))
            .child(self.render_notion_sidebar_row(&notion_sidebar_search_row(search_open), cx))
    }

    fn render_notion_sidebar_tab_list(&self, rows: Vec<gpui::AnyElement>) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .h(px(32.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .children(rows)
    }
}

fn notion_sidebar_search_row(search_open: bool) -> NotionSidebarRow {
    NotionSidebarRow {
        element_id: "notion-sidebar-search".into(),
        label: "Search".into(),
        secondary_label: None,
        icon: PageShellIcon::named("search"),
        action: NotionSidebarRowAction::ToggleSearch,
        layout: NotionSidebarRowLayout::TopIcon,
        active: search_open,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: false,
    }
}

impl NotionSidebarState {
    fn top_tab_row(&self, link: &PageShellLink, index: usize) -> Option<NotionSidebarRow> {
        let tab = notion_sidebar_link_tab(link)?;
        Some(NotionSidebarRow {
            element_id: notion_sidebar_link_element_id(link, index).into(),
            label: link.title.clone().into(),
            secondary_label: None,
            icon: link.icon.clone(),
            action: NotionSidebarRowAction::ActivateTab(tab),
            layout: NotionSidebarRowLayout::TopTab(tab),
            active: self.active_tab == tab,
            depth: 0,
            node_key: None,
            has_children: false,
            expanded: false,
        })
    }
}

fn notion_sidebar_link_tab(link: &PageShellLink) -> Option<NotionSidebarTab> {
    match link.identity.as_ref() {
        Some(PageShellNodeIdentity::Home) => Some(NotionSidebarTab::Home),
        Some(PageShellNodeIdentity::Chat) => Some(NotionSidebarTab::Chat),
        Some(PageShellNodeIdentity::Meetings) => Some(NotionSidebarTab::Meetings),
        Some(PageShellNodeIdentity::Inbox) => Some(NotionSidebarTab::Inbox),
        Some(PageShellNodeIdentity::Page { .. } | PageShellNodeIdentity::Database { .. }) => None,
        None if link.icon.kind == "named" && link.icon.value == "search" => None,
        None if link.target_board_url.is_some() => Some(NotionSidebarTab::Home),
        None => None,
    }
}

fn notion_sidebar_link_element_id(link: &PageShellLink, index: usize) -> String {
    match link.identity.as_ref() {
        Some(identity) => format!("notion-sidebar-top-{identity:?}"),
        None => format!("notion-sidebar-top-legacy-{index}"),
    }
}
