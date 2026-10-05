use crate::model::{
    BoardSnapshot, MutateSidebarInboxAction, NotionSidebarInboxFilter, PageShellSidebarItem,
    PageShellSnapshot,
};
use crate::ui::{Arc, ScrollHandle};
use gpui::{px, ListAlignment, ListState, SharedString};
use std::{cell::RefCell, collections::HashSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum NotionSidebarTab {
    #[default]
    Home,
    Chat,
    Meetings,
    Inbox,
}

#[derive(Clone)]
pub(crate) enum NotionSidebarUpdate {
    ActivateTab(NotionSidebarTab),
    DismissInboxMenus,
    SelectInboxFilter(NotionSidebarInboxFilter),
    ShowAllMeetings,
    ToggleInboxArchiveMenu,
    ToggleInboxFilterMenu,
    ToggleNode(NotionSidebarNodeKey),
    ToggleSection(NotionSidebarSectionKey),
}

pub(crate) enum NotionSidebarEffect {
    None,
    Notify,
    RebuildRows,
    RebuildRowsAndClosePageMenu,
    RebuildRowsAndLoadNode(NotionSidebarNodeKey),
    LoadInbox,
}

pub(crate) struct NotionSidebarState {
    pub(crate) active_tab: NotionSidebarTab,
    pub(crate) collapsed_sections: HashSet<NotionSidebarSectionKey>,
    pub(crate) expanded_nodes: HashSet<NotionSidebarNodeKey>,
    pub(crate) loading_nodes: HashSet<NotionSidebarNodeKey>,
    pub(crate) meetings_show_all: bool,
    pub(crate) selected_chat_id: Option<SharedString>,
    pub(crate) inbox_filter: NotionSidebarInboxFilter,
    pub(crate) inbox_filter_menu_open: bool,
    pub(crate) inbox_archive_menu_open: bool,
    pub(crate) inbox_action_in_flight: bool,
    pub(crate) inbox_loading_more: bool,
    pub(crate) inbox_request_size: u32,
    pub(crate) prefetch_target: Option<String>,
    pub(crate) list_state: ListState,
    pub(crate) list_row_keys: RefCell<Vec<SharedString>>,
    pub(crate) rows: Arc<[NotionSidebarVisibleRow]>,
    pub(crate) tree: NotionSidebarTreeState,
    pub(crate) calendar: NotionSidebarCalendarState,
    pub(crate) chats: NotionSidebarChatsState,
    pub(crate) inbox: NotionSidebarInboxState,
    pub(crate) inbox_scroll_handle: ScrollHandle,
}

impl NotionSidebarState {
    pub(crate) fn new(board: &BoardSnapshot) -> Self {
        Self {
            active_tab: NotionSidebarTab::Home,
            collapsed_sections: HashSet::new(),
            expanded_nodes: initial_expanded_nodes(board),
            loading_nodes: HashSet::new(),
            meetings_show_all: false,
            selected_chat_id: None,
            inbox_filter: NotionSidebarInboxFilter::All,
            inbox_filter_menu_open: false,
            inbox_archive_menu_open: false,
            inbox_action_in_flight: false,
            inbox_loading_more: false,
            inbox_request_size: 20,
            prefetch_target: None,
            list_state: ListState::new(0, ListAlignment::Top, px(31.0)),
            list_row_keys: RefCell::default(),
            rows: Arc::default(),
            tree: NotionSidebarTreeState::default(),
            calendar: NotionSidebarCalendarState::default(),
            chats: NotionSidebarChatsState::default(),
            inbox: NotionSidebarInboxState::default(),
            inbox_scroll_handle: ScrollHandle::new(),
        }
    }

    pub(crate) fn inherit_startup_state(&mut self, previous: &Self) {
        self.active_tab = previous.active_tab;
        self.collapsed_sections = previous.collapsed_sections.clone();
        self.expanded_nodes
            .extend(previous.expanded_nodes.iter().cloned());
        self.meetings_show_all = previous.meetings_show_all;
        self.list_state = previous.list_state.clone();
        self.list_row_keys
            .replace(previous.list_row_keys.borrow().clone());
        self.inbox_scroll_handle = previous.inbox_scroll_handle.clone();
    }

    pub(crate) fn inherit_navigation_state(&mut self, previous: &Self) {
        self.inherit_startup_state(previous);
        self.calendar = previous.calendar.clone();
        if previous.tree == NotionSidebarTreeState::Loaded {
            self.tree = NotionSidebarTreeState::Loaded;
        }
        self.chats = previous.chats.clone();
        self.inbox = previous.inbox.clone();
        self.inbox_filter = previous.inbox_filter;
        self.inbox_request_size = previous.inbox_request_size;
        self.selected_chat_id = previous.selected_chat_id.clone();
    }

    pub(crate) fn activate_tab(&mut self, tab: NotionSidebarTab) -> bool {
        if self.active_tab == tab {
            return false;
        }
        self.active_tab = tab;
        self.dismiss_inbox_menus_without_notify();
        true
    }

    pub(crate) fn reduce(&mut self, update: NotionSidebarUpdate) -> NotionSidebarEffect {
        match update {
            NotionSidebarUpdate::ActivateTab(tab) => {
                if self.activate_tab(tab) {
                    NotionSidebarEffect::RebuildRowsAndClosePageMenu
                } else {
                    NotionSidebarEffect::None
                }
            }
            NotionSidebarUpdate::DismissInboxMenus => {
                if self.dismiss_inbox_menus_without_notify() {
                    NotionSidebarEffect::Notify
                } else {
                    NotionSidebarEffect::None
                }
            }
            NotionSidebarUpdate::SelectInboxFilter(filter) => {
                if self.select_inbox_filter(filter) {
                    NotionSidebarEffect::LoadInbox
                } else {
                    NotionSidebarEffect::Notify
                }
            }
            NotionSidebarUpdate::ShowAllMeetings => {
                if self.show_all_meetings() {
                    NotionSidebarEffect::RebuildRows
                } else {
                    NotionSidebarEffect::None
                }
            }
            NotionSidebarUpdate::ToggleInboxArchiveMenu => {
                if self.toggle_inbox_archive_menu() {
                    NotionSidebarEffect::Notify
                } else {
                    NotionSidebarEffect::None
                }
            }
            NotionSidebarUpdate::ToggleInboxFilterMenu => {
                self.toggle_inbox_filter_menu();
                NotionSidebarEffect::Notify
            }
            NotionSidebarUpdate::ToggleNode(key) => {
                if self.toggle_node(key.clone()) {
                    NotionSidebarEffect::RebuildRowsAndLoadNode(key)
                } else {
                    NotionSidebarEffect::RebuildRows
                }
            }
            NotionSidebarUpdate::ToggleSection(key) => {
                self.toggle_section(key);
                NotionSidebarEffect::RebuildRows
            }
        }
    }

    pub(crate) fn show_all_meetings(&mut self) -> bool {
        if self.meetings_show_all {
            return false;
        }
        self.meetings_show_all = true;
        true
    }

    pub(crate) fn toggle_section(&mut self, key: NotionSidebarSectionKey) {
        if !self.collapsed_sections.remove(&key) {
            self.collapsed_sections.insert(key);
        }
    }

    /// Toggles a tree node and returns whether it is expanded afterward.
    pub(crate) fn toggle_node(&mut self, key: NotionSidebarNodeKey) -> bool {
        if self.expanded_nodes.remove(&key) {
            return false;
        }
        self.expanded_nodes.insert(key);
        true
    }

    pub(crate) fn select_chat(&mut self, thread_id: SharedString) {
        self.selected_chat_id = Some(thread_id);
    }

    pub(crate) fn select_inbox_filter(&mut self, filter: NotionSidebarInboxFilter) -> bool {
        self.dismiss_inbox_menus_without_notify();
        if self.inbox_filter == filter {
            return false;
        }
        self.inbox_filter = filter;
        self.inbox_request_size = 20;
        self.inbox_loading_more = false;
        self.inbox_scroll_handle = ScrollHandle::new();
        self.inbox = NotionSidebarInboxState::Idle;
        true
    }

    pub(crate) fn dismiss_inbox_menus_without_notify(&mut self) -> bool {
        if !self.inbox_filter_menu_open && !self.inbox_archive_menu_open {
            return false;
        }
        self.inbox_filter_menu_open = false;
        self.inbox_archive_menu_open = false;
        true
    }

    pub(crate) fn inbox_menu_is_open(&self) -> bool {
        self.inbox_filter_menu_open || self.inbox_archive_menu_open
    }

    pub(crate) fn toggle_inbox_filter_menu(&mut self) {
        self.inbox_filter_menu_open = !self.inbox_filter_menu_open;
        self.inbox_archive_menu_open = false;
    }

    pub(crate) fn toggle_inbox_archive_menu(&mut self) -> bool {
        if self.inbox_filter == NotionSidebarInboxFilter::Archived {
            return false;
        }
        self.inbox_archive_menu_open = !self.inbox_archive_menu_open;
        self.inbox_filter_menu_open = false;
        true
    }

    pub(crate) fn apply_optimistic_inbox_action(&mut self, action: &MutateSidebarInboxAction) {
        let NotionSidebarInboxState::Loaded { items, has_more } = &self.inbox else {
            return;
        };
        let notification_ids = match action {
            MutateSidebarInboxAction::SetRead {
                notification_ids, ..
            }
            | MutateSidebarInboxAction::SetArchived {
                notification_ids, ..
            } => notification_ids,
            MutateSidebarInboxAction::MarkAllRead | MutateSidebarInboxAction::ArchiveAll { .. } => {
                return;
            }
        };
        let mut next = Vec::with_capacity(items.len());
        for item in items.iter() {
            let mut item = item.clone();
            if notification_ids.contains(&item.notification_id) {
                match action {
                    MutateSidebarInboxAction::SetRead { read, .. } => item.read = *read,
                    MutateSidebarInboxAction::SetArchived { archived, .. } => {
                        item.archived = *archived;
                        if *archived {
                            item.read = true;
                        }
                    }
                    MutateSidebarInboxAction::MarkAllRead
                    | MutateSidebarInboxAction::ArchiveAll { .. } => {}
                }
            }
            let visible = match self.inbox_filter {
                NotionSidebarInboxFilter::All => !item.archived,
                NotionSidebarInboxFilter::Unread => !item.archived && !item.read,
                NotionSidebarInboxFilter::Archived => item.archived,
                NotionSidebarInboxFilter::WorkspaceUpdates => true,
            };
            if visible {
                next.push(item);
            }
        }
        self.inbox = NotionSidebarInboxState::Loaded {
            items: next.into(),
            has_more: *has_more,
        };
    }

    pub(crate) fn rebuild_rows(&mut self, page_shell: Option<&PageShellSnapshot>) {
        let rows: Arc<[NotionSidebarVisibleRow]> = page_shell
            .map(|page_shell| match self.active_tab {
                NotionSidebarTab::Home => self.flatten_notion_sidebar_home_rows(page_shell),
                NotionSidebarTab::Meetings => self.flatten_notion_sidebar_meetings_rows(page_shell),
                NotionSidebarTab::Chat => self.flatten_notion_sidebar_chat_rows(),
                NotionSidebarTab::Inbox => Vec::new(),
            })
            .unwrap_or_default()
            .into();
        self.sync_list_state(&rows);
        self.rows = rows;
    }

    pub(crate) fn sync_list_state(&self, rows: &[NotionSidebarVisibleRow]) {
        let next_keys = rows
            .iter()
            .map(|row| row.list_key().clone())
            .collect::<Vec<_>>();
        let mut previous_keys = self.list_row_keys.borrow_mut();
        if *previous_keys == next_keys {
            return;
        }

        let prefix_len = previous_keys
            .iter()
            .zip(&next_keys)
            .take_while(|(previous, next)| previous == next)
            .count();
        let suffix_limit = previous_keys.len().min(next_keys.len()) - prefix_len;
        let suffix_len = previous_keys
            .iter()
            .rev()
            .zip(next_keys.iter().rev())
            .take(suffix_limit)
            .take_while(|(previous, next)| previous == next)
            .count();
        self.list_state.splice(
            prefix_len..previous_keys.len() - suffix_len,
            next_keys.len() - prefix_len - suffix_len,
        );
        *previous_keys = next_keys;
    }
}

fn initial_expanded_nodes(board: &BoardSnapshot) -> HashSet<NotionSidebarNodeKey> {
    let mut expanded = HashSet::new();
    let Some(page_shell) = board.page_shell.as_ref() else {
        return expanded;
    };
    for (section_index, section) in page_shell.sidebar_sections.iter().enumerate() {
        let section_key =
            NotionSidebarSectionKey::new(section.identity.as_ref(), section_index, &section.title);
        for (item_index, item) in section.items.iter().enumerate() {
            collect_active_ancestors(item, &section_key, &[item_index], &mut expanded);
        }
    }
    expanded
}

fn collect_active_ancestors(
    item: &PageShellSidebarItem,
    section_key: &NotionSidebarSectionKey,
    path: &[usize],
    expanded: &mut HashSet<NotionSidebarNodeKey>,
) -> bool {
    let mut child_is_active = false;
    for (child_index, child) in item.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(child_index);
        child_is_active |= collect_active_ancestors(child, section_key, &child_path, expanded);
    }
    let has_children = !item.children.is_empty() || !item.sidebar_children_resolved;
    if child_is_active || (item.active && has_children) {
        expanded.insert(NotionSidebarNodeKey::new(
            item.identity.as_ref(),
            section_key,
            path,
        ));
    }
    item.active || child_is_active
}

mod resource_job;
mod resource_loading;
mod resources;
mod rows;
mod tree_loading;
pub(crate) use resource_job::{
    SidebarResourceContext, SidebarResourceEffect, SidebarResourceEvent,
};
pub(crate) use resources::{
    NotionSidebarCalendarState, NotionSidebarChatsState, NotionSidebarInboxState,
    NotionSidebarTreeState,
};
pub(crate) use rows::{
    NotionSidebarNodeKey, NotionSidebarRow, NotionSidebarRowAction, NotionSidebarRowLayout,
    NotionSidebarSectionKey, NotionSidebarSectionRow, NotionSidebarVisibleRow,
};
pub(crate) use tree_loading::{SidebarWorkspacePreparation, SidebarWorkspaceReadiness};
