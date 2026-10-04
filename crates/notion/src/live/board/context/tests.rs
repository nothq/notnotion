use super::sidebar_breadcrumbs;
use crate::model::{
    PageShellIcon, PageShellNodeIdentity, PageShellSidebarItem, PageShellSidebarSection,
    PageShellSnapshot,
};

fn sidebar_item(
    block_id: &str,
    title: &str,
    children: Vec<PageShellSidebarItem>,
) -> PageShellSidebarItem {
    PageShellSidebarItem {
        identity: Some(PageShellNodeIdentity::Page {
            block_id: block_id.to_string(),
        }),
        title: title.to_string(),
        icon: PageShellIcon::named("page"),
        active: false,
        target_board_url: None,
        children,
        child_block_ids: Vec::new(),
        unresolved_child_block_ids: Vec::new(),
        sidebar_children_resolved: true,
    }
}

#[test]
fn sidebar_breadcrumbs_are_free_cached_recents_metadata() {
    let leaf = sidebar_item(
        "00000000-0000-0000-0000-000000000003",
        "Team Directory",
        Vec::new(),
    );
    let middle = sidebar_item(
        "00000000-0000-0000-0000-000000000002",
        "Administration",
        vec![leaf],
    );
    let root = sidebar_item(
        "00000000-0000-0000-0000-000000000001",
        "General",
        vec![middle],
    );
    let page_shell = PageShellSnapshot {
        workspace_name: "Acme".to_string(),
        page_icon: PageShellIcon::named("page"),
        page_icon_is_explicit: false,
        builtin_links: Vec::new(),
        sidebar_sections: vec![PageShellSidebarSection {
            identity: None,
            title: "Teamspaces".to_string(),
            icon: PageShellIcon::named("page"),
            items: vec![root],
        }],
        links: Vec::new(),
    };

    let breadcrumbs = sidebar_breadcrumbs(&page_shell);

    assert_eq!(
        breadcrumbs
            .get("00000000-0000-0000-0000-000000000002")
            .map(String::as_str),
        Some("General")
    );
    assert_eq!(
        breadcrumbs
            .get("00000000-0000-0000-0000-000000000003")
            .map(String::as_str),
        Some("General / Administration")
    );
}
