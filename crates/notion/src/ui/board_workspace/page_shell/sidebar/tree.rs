use super::{
    NotionSidebarNodeKey, NotionSidebarRow, NotionSidebarRowAction, NotionSidebarRowLayout,
    NotionSidebarSectionKey, NotionSidebarVisibleRow, PageShellSidebarItem, SharedString,
};
use crate::ui::surface::NotionSidebarState;

/// Where a sidebar item sits: its section, its index path within that section,
/// and its nesting depth.
pub(super) struct NotionSidebarItemPosition<'a> {
    pub(super) section_key: &'a NotionSidebarSectionKey,
    pub(super) path: &'a mut Vec<usize>,
    pub(super) depth: usize,
}

impl NotionSidebarState {
    pub(super) fn flatten_notion_sidebar_item_rows(
        &self,
        item: &PageShellSidebarItem,
        position: NotionSidebarItemPosition<'_>,
        rows: &mut Vec<NotionSidebarVisibleRow>,
    ) {
        let NotionSidebarItemPosition {
            section_key,
            path,
            depth,
        } = position;
        let key = NotionSidebarNodeKey::new(item.identity.as_ref(), section_key, path);
        let has_children = !item.children.is_empty() || !item.sidebar_children_resolved;
        let expanded = has_children && self.expanded_nodes.contains(&key);
        let label: SharedString = item.title.clone().into();
        let action = notion_sidebar_item_action(item, label.clone());
        rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
            element_id: format!("notion-sidebar-node-{key:?}").into(),
            label,
            secondary_label: None,
            icon: item.icon.clone(),
            action,
            layout: NotionSidebarRowLayout::Tree,
            active: item.active,
            depth,
            node_key: Some(key),
            has_children,
            expanded,
        }));
        if expanded {
            for (child_index, child) in item.children.iter().enumerate() {
                path.push(child_index);
                self.flatten_notion_sidebar_item_rows(
                    child,
                    NotionSidebarItemPosition {
                        section_key,
                        path: &mut *path,
                        depth: depth + 1,
                    },
                    rows,
                );
                path.pop();
            }
        }
    }
}

fn notion_sidebar_item_action(
    item: &PageShellSidebarItem,
    label: SharedString,
) -> NotionSidebarRowAction {
    match item.target_board_url.clone() {
        Some(board_url) => NotionSidebarRowAction::OpenWorkspace { board_url, label },
        None => NotionSidebarRowAction::None,
    }
}
