use super::tree::NotionSidebarItemPosition;
use super::{
    NotionSidebarCalendarState, NotionSidebarRow, NotionSidebarRowAction, NotionSidebarRowLayout,
    NotionSidebarSectionKey, NotionSidebarTab, NotionSidebarVisibleRow, PageShellCalendarEvent,
    PageShellIcon, PageShellSidebarSection, PageShellSidebarSectionIdentity, PageShellSnapshot,
};
use crate::ui::surface::NotionSidebarState;

const HOME_MEETING_EVENT_LIMIT: usize = 2;
const MEETINGS_INITIAL_EVENT_LIMIT: usize = 3;
const MEETINGS_EXPANDED_EVENT_LIMIT: usize = 9;
const HOME_MEETINGS_SECTION_GAP: f32 = 16.0;
const MEETINGS_PANEL_GAP: f32 = 42.0;

impl NotionSidebarState {
    pub(crate) fn flatten_notion_sidebar_home_rows(
        &self,
        page_shell: &PageShellSnapshot,
    ) -> Vec<NotionSidebarVisibleRow> {
        let sections = &page_shell.sidebar_sections;
        let mut rows = Vec::new();
        let mut suppress_next_gap = self.push_notion_sidebar_home_meetings_rows(&mut rows);
        let mut index = 0;
        while index < sections.len() {
            if is_notion_meetings_section(&sections[index]) {
                index += 1;
                continue;
            }
            let top_gap = !rows.is_empty() && !suppress_next_gap;
            suppress_next_gap = false;
            if !is_notion_teamspace_section(&sections[index]) {
                self.push_notion_sidebar_section(&sections[index], index, top_gap, &mut rows);
                index += 1;
                continue;
            }
            let start = index;
            while index < sections.len() && is_notion_teamspace_section(&sections[index]) {
                index += 1;
            }
            self.push_notion_sidebar_teamspaces_group(
                &sections[start..index],
                start,
                top_gap,
                &mut rows,
            );
        }
        rows
    }

    pub(crate) fn flatten_notion_sidebar_meetings_rows(
        &self,
        page_shell: &PageShellSnapshot,
    ) -> Vec<NotionSidebarVisibleRow> {
        let mut rows = Vec::new();
        self.push_notion_sidebar_upcoming_meetings_rows(&mut rows);
        if !rows.is_empty() {
            push_panel_gap(&mut rows, "meetings-before-new-note", MEETINGS_PANEL_GAP);
        }
        push_panel_action(
            &mut rows,
            "meetings-new-ai-meeting-note",
            "New AI meeting note",
            "panel-add",
            NotionSidebarRowAction::None,
        );
        push_panel_gap(
            &mut rows,
            "meetings-before-recent-notes",
            MEETINGS_PANEL_GAP,
        );
        self.push_notion_sidebar_recent_meetings_rows(page_shell, &mut rows);
        rows
    }

    fn push_notion_sidebar_upcoming_meetings_rows(&self, rows: &mut Vec<NotionSidebarVisibleRow>) {
        match &self.calendar {
            NotionSidebarCalendarState::Loading => {}
            NotionSidebarCalendarState::Loaded(events) => {
                push_panel_header(rows, "meetings-upcoming", "Upcoming");
                if events.is_empty() {
                    push_panel_empty(rows, "meetings-upcoming-empty", "No upcoming events");
                }
                let limit = if self.meetings_show_all {
                    MEETINGS_EXPANDED_EVENT_LIMIT
                } else {
                    MEETINGS_INITIAL_EVENT_LIMIT
                };
                for event in events.iter().take(limit) {
                    push_calendar_event_row(rows, "meetings-upcoming", event);
                }
                if events.len() > MEETINGS_INITIAL_EVENT_LIMIT && !self.meetings_show_all {
                    push_panel_action(
                        rows,
                        "meetings-more",
                        "More",
                        "more",
                        NotionSidebarRowAction::ToggleMeetingsMore,
                    );
                }
            }
            NotionSidebarCalendarState::Unavailable => {
                push_panel_header(rows, "meetings-upcoming", "Upcoming");
                push_panel_empty(rows, "meetings-upcoming-empty", "No upcoming events");
            }
            NotionSidebarCalendarState::Idle | NotionSidebarCalendarState::Failed => {}
        }
    }

    fn push_notion_sidebar_recent_meetings_rows(
        &self,
        page_shell: &PageShellSnapshot,
        rows: &mut Vec<NotionSidebarVisibleRow>,
    ) {
        let Some((section_index, section)) = page_shell
            .sidebar_sections
            .iter()
            .enumerate()
            .find(|(_, section)| is_notion_meetings_section(section))
        else {
            return;
        };
        let key =
            NotionSidebarSectionKey::new(section.identity.as_ref(), section_index, &section.title);
        for (item_index, item) in section.items.iter().enumerate() {
            let mut path = vec![item_index];
            self.flatten_notion_sidebar_item_rows(
                item,
                NotionSidebarItemPosition {
                    section_key: &key,
                    path: &mut path,
                    depth: 0,
                },
                rows,
            );
        }
    }
}

impl NotionSidebarState {
    fn push_notion_sidebar_home_meetings_rows(
        &self,
        rows: &mut Vec<NotionSidebarVisibleRow>,
    ) -> bool {
        match &self.calendar {
            NotionSidebarCalendarState::Loading => return false,
            NotionSidebarCalendarState::Loaded(events) => {
                push_panel_header(rows, "home-meetings", "Meetings");
                if events.is_empty() {
                    push_panel_empty(rows, "home-meetings-empty", "No upcoming events");
                }
                for event in events.iter().take(HOME_MEETING_EVENT_LIMIT) {
                    push_calendar_event_row(rows, "home-meetings", event);
                }
            }
            NotionSidebarCalendarState::Unavailable => {
                push_panel_header(rows, "home-meetings", "Meetings");
                push_panel_empty(rows, "home-meetings-empty", "No upcoming events");
            }
            NotionSidebarCalendarState::Idle | NotionSidebarCalendarState::Failed => return false,
        }
        push_panel_action(
            rows,
            "home-new-ai-meeting-note",
            "New AI meeting note",
            "panel-add",
            NotionSidebarRowAction::None,
        );
        push_panel_action(
            rows,
            "home-view-all-meetings",
            "View all",
            "panel-open",
            NotionSidebarRowAction::ActivateTab(NotionSidebarTab::Meetings),
        );
        push_panel_gap(rows, "home-meetings-section-gap", HOME_MEETINGS_SECTION_GAP);
        true
    }
}

fn push_panel_header(rows: &mut Vec<NotionSidebarVisibleRow>, element_id: &str, label: &str) {
    rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
        element_id: element_id.to_string().into(),
        label: label.to_string().into(),
        secondary_label: None,
        icon: named_panel_icon("meetings"),
        action: NotionSidebarRowAction::None,
        layout: NotionSidebarRowLayout::PanelHeader,
        active: false,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: true,
    }));
}

fn push_panel_empty(rows: &mut Vec<NotionSidebarVisibleRow>, element_id: &str, label: &str) {
    rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
        element_id: element_id.to_string().into(),
        label: label.to_string().into(),
        secondary_label: None,
        icon: named_panel_icon("meetings"),
        action: NotionSidebarRowAction::None,
        layout: NotionSidebarRowLayout::PanelEmpty,
        active: false,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: false,
    }));
}

fn push_calendar_event_row(
    rows: &mut Vec<NotionSidebarVisibleRow>,
    group: &str,
    event: &PageShellCalendarEvent,
) {
    rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
        element_id: format!("{group}-event-{}", event.event_id).into(),
        label: event.title.clone().into(),
        secondary_label: Some(event.time_label.clone().into()),
        icon: named_panel_icon(if event.ongoing {
            "calendar-event-current"
        } else {
            "calendar-event-upcoming"
        }),
        action: NotionSidebarRowAction::OpenUrl(event.target_url.clone().into()),
        layout: NotionSidebarRowLayout::CalendarEvent,
        active: false,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: false,
    }));
}

fn push_panel_action(
    rows: &mut Vec<NotionSidebarVisibleRow>,
    element_id: &str,
    label: &str,
    icon: &str,
    action: NotionSidebarRowAction,
) {
    rows.push(NotionSidebarVisibleRow::Item(NotionSidebarRow {
        element_id: element_id.to_string().into(),
        label: label.to_string().into(),
        secondary_label: None,
        icon: named_panel_icon(icon),
        action,
        layout: NotionSidebarRowLayout::PanelAction,
        active: false,
        depth: 0,
        node_key: None,
        has_children: false,
        expanded: false,
    }));
}

fn push_panel_gap(rows: &mut Vec<NotionSidebarVisibleRow>, element_id: &str, height: f32) {
    rows.push(NotionSidebarVisibleRow::Gap {
        list_key: element_id.to_string().into(),
        height,
    });
}

fn named_panel_icon(value: &str) -> PageShellIcon {
    PageShellIcon::named(value)
}

fn is_notion_teamspace_section(section: &PageShellSidebarSection) -> bool {
    matches!(
        section.identity.as_ref(),
        Some(PageShellSidebarSectionIdentity::Teamspace { .. })
    )
}

fn is_notion_meetings_section(section: &PageShellSidebarSection) -> bool {
    matches!(
        section.identity.as_ref(),
        Some(PageShellSidebarSectionIdentity::Meetings { .. })
    )
}
