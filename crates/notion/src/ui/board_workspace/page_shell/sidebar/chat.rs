use chrono::{DateTime, Local, Utc};

use super::{
    NotionSidebarChatsState, NotionSidebarRow, NotionSidebarRowAction, NotionSidebarRowLayout,
    NotionSidebarVisibleRow, PageShellChatThread, PageShellIcon, SharedString,
};
use crate::ui::surface::NotionSidebarState;

impl NotionSidebarState {
    pub(crate) fn flatten_notion_sidebar_chat_rows(&self) -> Vec<NotionSidebarVisibleRow> {
        let mut rows = Vec::new();
        push_chat_header(&mut rows, "chat-notion-ai", "Notion AI");
        push_chat_agents(&mut rows);
        match &self.chats {
            NotionSidebarChatsState::Idle | NotionSidebarChatsState::Loading => {
                push_chat_loading_rows(&mut rows)
            }
            NotionSidebarChatsState::Loaded { threads } => {
                self.push_loaded_chat_rows(threads, &mut rows)
            }
            NotionSidebarChatsState::Failed => {}
        }
        rows
    }
}

impl NotionSidebarState {
    fn push_loaded_chat_rows(
        &self,
        threads: &[PageShellChatThread],
        rows: &mut Vec<NotionSidebarVisibleRow>,
    ) {
        let pinned = threads.iter().filter(|thread| thread.pinned);
        if pinned.clone().next().is_some() {
            push_chat_header(rows, "chat-pinned", "Pinned");
            for thread in pinned {
                self.push_chat_thread(rows, thread);
            }
        }
        let mut previous_bucket = None;
        for thread in threads.iter().filter(|thread| !thread.pinned) {
            let bucket = chat_date_bucket(thread.sort_time);
            if previous_bucket != Some(bucket) {
                push_chat_header(
                    rows,
                    &format!("chat-bucket-{}", bucket.element_segment()),
                    bucket.label(),
                );
                previous_bucket = Some(bucket);
            }
            self.push_chat_thread(rows, thread);
        }
        if threads.is_empty() {
            push_chat_header(rows, "chat-empty-today", "Today");
        }
    }

    fn push_chat_thread(
        &self,
        rows: &mut Vec<NotionSidebarVisibleRow>,
        thread: &PageShellChatThread,
    ) {
        rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
            element_id: format!("notion-chat-thread-{}", thread.thread_id).into(),
            label: thread.title.clone().into(),
            secondary_label: thread.unread.then(|| "Unread".into()),
            icon: named_chat_icon("notion-ai"),
            action: NotionSidebarRowAction::OpenChatThread(thread.thread_id.clone().into()),
            layout: NotionSidebarRowLayout::ChatThread,
            active: self.selected_chat_id.as_deref() == Some(thread.thread_id.as_str()),
            depth: 0,
            node_key: None,
            has_children: false,
            expanded: false,
        }));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChatDateBucket {
    Today,
    Yesterday,
    PastWeek,
    PastThirtyDays,
    Older,
}

impl ChatDateBucket {
    const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::PastWeek => "Past week",
            Self::PastThirtyDays => "Past 30 days",
            Self::Older => "Older",
        }
    }

    const fn element_segment(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::PastWeek => "past-week",
            Self::PastThirtyDays => "past-30-days",
            Self::Older => "older",
        }
    }
}

fn chat_date_bucket(timestamp_ms: u64) -> ChatDateBucket {
    let timestamp_ms = i64::try_from(timestamp_ms).unwrap_or(i64::MAX);
    let timestamp = DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
        .with_timezone(&Local)
        .date_naive();
    let days = Local::now()
        .date_naive()
        .signed_duration_since(timestamp)
        .num_days();
    match days {
        ..=0 => ChatDateBucket::Today,
        1 => ChatDateBucket::Yesterday,
        2..=7 => ChatDateBucket::PastWeek,
        8..=30 => ChatDateBucket::PastThirtyDays,
        _ => ChatDateBucket::Older,
    }
}

fn push_chat_header(
    rows: &mut Vec<NotionSidebarVisibleRow>,
    element_id: &str,
    label: impl Into<SharedString>,
) {
    rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
        element_id: element_id.to_string().into(),
        label: label.into(),
        secondary_label: None,
        icon: named_chat_icon("chat"),
        action: NotionSidebarRowAction::None,
        layout: NotionSidebarRowLayout::PanelHeader,
        active: false,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: false,
    }));
}

fn push_chat_agents(rows: &mut Vec<NotionSidebarVisibleRow>) {
    rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
        element_id: "chat-agent-gallery".into(),
        label: "Notion AI".into(),
        secondary_label: None,
        icon: named_chat_icon("notion-ai"),
        action: NotionSidebarRowAction::None,
        layout: NotionSidebarRowLayout::ChatAgents,
        active: false,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: false,
    }));
}

fn push_chat_loading_rows(rows: &mut Vec<NotionSidebarVisibleRow>) {
    push_chat_header(rows, "chat-loading-today", "Today");
    for index in 0..4 {
        rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
            element_id: format!("chat-loading-first-{index}").into(),
            label: SharedString::default(),
            secondary_label: None,
            icon: named_chat_icon("chat"),
            action: NotionSidebarRowAction::None,
            layout: NotionSidebarRowLayout::Skeleton,
            active: false,
            depth: 0,
            node_key: None,
            has_children: false,
            expanded: false,
        }));
    }
}

fn named_chat_icon(value: &str) -> PageShellIcon {
    PageShellIcon::named(value)
}
