use crate::model::{PageShellCalendarEvent, PageShellChatThread, PageShellInboxItem};
use crate::ui::Arc;

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum NotionSidebarTreeState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Failed,
}

#[derive(Clone, Default)]
pub(crate) enum NotionSidebarCalendarState {
    #[default]
    Idle,
    Loading,
    Unavailable,
    Loaded(Arc<[PageShellCalendarEvent]>),
    Failed,
}

#[derive(Clone, Default)]
pub(crate) enum NotionSidebarChatsState {
    #[default]
    Idle,
    Loading,
    Loaded {
        threads: Arc<[PageShellChatThread]>,
    },
    Failed,
}

#[derive(Clone, Default)]
pub(crate) enum NotionSidebarInboxState {
    #[default]
    Idle,
    Loading,
    Loaded {
        items: Arc<[PageShellInboxItem]>,
        has_more: bool,
    },
    Failed,
}
