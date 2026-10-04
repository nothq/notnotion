use super::NotionSidebarTab;
use crate::model::{PageShellIcon, PageShellNodeIdentity, PageShellSidebarSectionIdentity};
use gpui::SharedString;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum NotionSidebarSectionKey {
    Identity(PageShellSidebarSectionIdentity),
    Legacy { index: usize, title: String },
}

impl NotionSidebarSectionKey {
    pub(crate) fn new(
        identity: Option<&PageShellSidebarSectionIdentity>,
        index: usize,
        title: &str,
    ) -> Self {
        match identity {
            Some(identity) => Self::Identity(identity.clone()),
            None => Self::Legacy {
                index,
                title: title.to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum NotionSidebarNodeKey {
    Identity {
        section: NotionSidebarSectionKey,
        node: PageShellNodeIdentity,
    },
    Legacy {
        section: NotionSidebarSectionKey,
        path: Vec<usize>,
    },
}

impl NotionSidebarNodeKey {
    pub(crate) fn new(
        identity: Option<&PageShellNodeIdentity>,
        section: &NotionSidebarSectionKey,
        path: &[usize],
    ) -> Self {
        match identity {
            Some(identity) => Self::Identity {
                section: section.clone(),
                node: identity.clone(),
            },
            None => Self::Legacy {
                section: section.clone(),
                path: path.to_vec(),
            },
        }
    }
}

#[derive(Clone)]
pub(crate) enum NotionSidebarRowAction {
    ActivateTab(NotionSidebarTab),
    OpenWorkspace {
        board_url: String,
        label: SharedString,
    },
    OpenUrl(SharedString),
    ToggleMeetingsMore,
    ToggleSearch,
    OpenChatThread(SharedString),
    None,
}

#[derive(Clone, Copy)]
pub(crate) enum NotionSidebarRowLayout {
    TopTab(NotionSidebarTab),
    TopIcon,
    PanelHeader,
    CalendarEvent,
    PanelEmpty,
    PanelAction,
    Skeleton,
    ChatAgents,
    ChatThread,
    Tree,
}

#[derive(Clone)]
pub(crate) struct NotionSidebarRow {
    pub(crate) element_id: SharedString,
    pub(crate) label: SharedString,
    pub(crate) secondary_label: Option<SharedString>,
    pub(crate) icon: PageShellIcon,
    pub(crate) action: NotionSidebarRowAction,
    pub(crate) layout: NotionSidebarRowLayout,
    pub(crate) active: bool,
    pub(crate) depth: usize,
    pub(crate) node_key: Option<NotionSidebarNodeKey>,
    pub(crate) has_children: bool,
    pub(crate) expanded: bool,
}

#[derive(Clone)]
pub(crate) struct NotionSidebarSectionRow {
    pub(crate) element_id: SharedString,
    pub(crate) label: SharedString,
    pub(crate) icon: PageShellIcon,
    pub(crate) key: NotionSidebarSectionKey,
    pub(crate) expanded: bool,
    pub(crate) teamspace: bool,
}

#[derive(Clone)]
pub(crate) enum NotionSidebarVisibleRow {
    Gap { list_key: SharedString, height: f32 },
    Section(NotionSidebarSectionRow),
    Item(NotionSidebarRow),
}

impl NotionSidebarVisibleRow {
    pub(crate) fn list_key(&self) -> &SharedString {
        match self {
            Self::Gap { list_key, .. } => list_key,
            Self::Section(row) => &row.element_id,
            Self::Item(row) => &row.element_id,
        }
    }
}
