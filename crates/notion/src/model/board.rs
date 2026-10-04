use serde::{Deserialize, Serialize};

use crate::model::{
    CardPage, DatabaseViewGroupState, DatabaseViewSortState, NotionShareTargetId, ViewTab,
};

mod database;
mod icon;

pub use database::{
    BoardColumn, BoardDateValue, BoardItem, BoardItemProperty, CalendarViewConfig,
    DatabaseProperty, DatabasePropertyOption, DatabaseStatusGroup, DatabaseViewPropertyLayout,
    DatabaseViewPropertyVisibility, TableViewColumn, TimelineViewConfig,
};
pub use icon::PageShellIcon;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PageShellNodeIdentity {
    Home,
    Chat,
    Meetings,
    Inbox,
    Page { block_id: String },
    Database { block_id: String },
}

impl PageShellNodeIdentity {
    pub fn block_id(&self) -> Option<&str> {
        match self {
            Self::Page { block_id } | Self::Database { block_id } => Some(block_id),
            Self::Home | Self::Chat | Self::Meetings | Self::Inbox => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PageShellSidebarSectionIdentity {
    Favorites,
    Teamspaces,
    Teamspace { team_id: String },
    Private,
    Shared,
    Meetings { sidebar_section_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellSidebarItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<PageShellNodeIdentity>,
    pub title: String,
    pub icon: PageShellIcon,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub target_board_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<PageShellSidebarItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_block_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_child_block_ids: Vec<String>,
    /// Whether every immediate sidebar child was discovered through this item's content tree.
    #[serde(default)]
    pub sidebar_children_resolved: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellSidebarSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<PageShellSidebarSectionIdentity>,
    pub title: String,
    pub icon: PageShellIcon,
    #[serde(default)]
    pub items: Vec<PageShellSidebarItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellLink {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<PageShellNodeIdentity>,
    pub title: String,
    pub icon: PageShellIcon,
    #[serde(default)]
    pub target_board_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellCalendarEvent {
    pub event_id: String,
    pub title: String,
    pub time_label: String,
    pub target_url: String,
    #[serde(default)]
    pub ongoing: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageShellSnapshot {
    pub workspace_name: String,
    pub page_icon: PageShellIcon,
    pub page_icon_is_explicit: bool,
    #[serde(default)]
    pub builtin_links: Vec<PageShellLink>,
    #[serde(default)]
    pub sidebar_sections: Vec<PageShellSidebarSection>,
    #[serde(default)]
    pub links: Vec<PageShellLink>,
}

impl PageShellSnapshot {
    pub(crate) fn merge_sidebar_hydration_from(
        &mut self,
        previous_sections: &[PageShellSidebarSection],
    ) {
        for (section_index, next_section) in self.sidebar_sections.iter_mut().enumerate() {
            let Some(previous_section) =
                matching_previous_section(previous_sections, next_section, section_index)
            else {
                continue;
            };
            for next_item in &mut next_section.items {
                let Some(previous_item) = previous_section
                    .items
                    .iter()
                    .find(|previous_item| sidebar_items_match(previous_item, next_item))
                else {
                    continue;
                };
                merge_sidebar_item_hydration(previous_item, next_item);
            }
        }
    }
}

fn matching_previous_section<'a>(
    previous_sections: &'a [PageShellSidebarSection],
    next_section: &PageShellSidebarSection,
    section_index: usize,
) -> Option<&'a PageShellSidebarSection> {
    match next_section.identity.as_ref() {
        Some(identity) => previous_sections
            .iter()
            .find(|section| section.identity.as_ref() == Some(identity)),
        None => previous_sections
            .get(section_index)
            .filter(|section| section.identity.is_none() && section.title == next_section.title),
    }
}

fn merge_sidebar_item_hydration(previous: &PageShellSidebarItem, next: &mut PageShellSidebarItem) {
    if !next.sidebar_children_resolved
        && (previous.sidebar_children_resolved || !previous.children.is_empty())
    {
        next.children.clone_from(&previous.children);
        next.child_block_ids.clone_from(&previous.child_block_ids);
        next.unresolved_child_block_ids
            .clone_from(&previous.unresolved_child_block_ids);
        next.sidebar_children_resolved = previous.sidebar_children_resolved;
        return;
    }
    for next_child in &mut next.children {
        let Some(previous_child) = previous
            .children
            .iter()
            .find(|previous_child| sidebar_items_match(previous_child, next_child))
        else {
            continue;
        };
        merge_sidebar_item_hydration(previous_child, next_child);
    }
}

fn sidebar_items_match(previous: &PageShellSidebarItem, next: &PageShellSidebarItem) -> bool {
    match (previous.identity.as_ref(), next.identity.as_ref()) {
        (Some(previous), Some(next)) => previous == next,
        (None, None) => {
            previous.target_board_url == next.target_board_url && previous.title == next.title
        }
        _ => false,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagePresenceAvatar {
    pub base64: String,
    pub mimetype: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagePresenceProfile {
    pub user_id: String,
    pub name: String,
    #[serde(default)]
    pub profile_photo: Option<String>,
    #[serde(default)]
    pub avatar: Option<PagePresenceAvatar>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PagePresenceSnapshot {
    pub profiles: Vec<PagePresenceProfile>,
    pub overflow_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoardSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_target_id: Option<NotionShareTargetId>,
    pub page_title: String,
    pub database_title: String,
    pub edited_label: String,
    /// The signed-in Notion user's validated IANA time-zone identifier.
    pub user_time_zone: String,
    /// The user's current UTC offset, resolved from `user_time_zone` at load time.
    pub user_utc_offset_seconds: i32,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub is_favorited: bool,
    #[serde(default)]
    pub presence: Option<PagePresenceSnapshot>,
    pub columns: Vec<BoardColumn>,
    pub view_tabs: Vec<ViewTab>,
    #[serde(default)]
    pub items: Vec<BoardItem>,
    #[serde(default)]
    pub table_view_columns: Vec<TableViewColumn>,
    #[serde(default)]
    pub active_view_property_layout: DatabaseViewPropertyLayout,
    #[serde(default)]
    pub active_view_sorts: DatabaseViewSortState,
    #[serde(default)]
    pub active_view_group: DatabaseViewGroupState,
    #[serde(default)]
    pub database_properties: Vec<DatabaseProperty>,
    #[serde(default)]
    pub timeline_view: Option<TimelineViewConfig>,
    #[serde(default)]
    pub calendar_view: Option<CalendarViewConfig>,
    #[serde(default)]
    pub date_undated_count: Option<usize>,
    #[serde(default)]
    pub page_content: Option<CardPage>,
    #[serde(default)]
    pub page_shell: Option<PageShellSnapshot>,
}
