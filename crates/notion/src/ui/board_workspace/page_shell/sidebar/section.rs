use super::tree::NotionSidebarItemPosition;
use super::{
    div, px, relative, rgb, AnyElement, AppearanceMode, Div, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, NotionSidebarSectionKey, NotionSidebarSectionRow,
    NotionSidebarTab, NotionSidebarUpdate, NotionSidebarVisibleRow, PageShellIcon,
    PageShellSidebarSection, PageShellSidebarSectionIdentity, ParentElement, SharedString,
    SidebarAction, SidebarRenderer, Styled, SIDEBAR_ROW_HEIGHT, SIDEBAR_SECTION_GAP,
};
use crate::ui::surface::NotionSidebarState;
use crate::ui::Arc;
use gpui::{list, App, ListSizingBehavior};

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_body(
        &self,
        state: &NotionSidebarState,
        rows: Arc<[NotionSidebarVisibleRow]>,
        cx: &mut App,
    ) -> AnyElement {
        let body = div()
            .id("notion-sidebar-scroll")
            .flex_grow(1.0)
            .min_h(px(0.0))
            .overflow_hidden();
        match state.active_tab {
            NotionSidebarTab::Home | NotionSidebarTab::Chat | NotionSidebarTab::Meetings => body
                .child(self.render_notion_sidebar_home_list(state, rows))
                .into_any_element(),
            NotionSidebarTab::Inbox => body
                .child(self.render_notion_sidebar_inbox(state, cx))
                .into_any_element(),
        }
    }
}

impl NotionSidebarState {
    pub(super) fn push_notion_sidebar_teamspaces_group(
        &self,
        teamspaces: &[PageShellSidebarSection],
        first_section_index: usize,
        top_gap: bool,
        rows: &mut Vec<NotionSidebarVisibleRow>,
    ) {
        let identity = PageShellSidebarSectionIdentity::Teamspaces;
        let key = NotionSidebarSectionKey::Identity(identity.clone());
        let collapsed = self.collapsed_sections.contains(&key);
        let section = PageShellSidebarSection {
            identity: Some(identity),
            title: "Teamspaces".to_string(),
            icon: PageShellIcon::named("teamspace"),
            items: Vec::new(),
        };
        let header = notion_sidebar_section_row(&section, key, !collapsed);
        if top_gap {
            push_notion_sidebar_section_gap(rows, &header.element_id);
        }
        rows.push(NotionSidebarVisibleRow::Section(header));
        if !collapsed {
            for (offset, section) in teamspaces.iter().enumerate() {
                self.push_notion_sidebar_section(
                    section,
                    first_section_index + offset,
                    false,
                    rows,
                );
            }
        }
    }

    pub(super) fn push_notion_sidebar_section(
        &self,
        section: &PageShellSidebarSection,
        section_index: usize,
        top_gap: bool,
        rows: &mut Vec<NotionSidebarVisibleRow>,
    ) {
        let key =
            NotionSidebarSectionKey::new(section.identity.as_ref(), section_index, &section.title);
        let collapsed = self.collapsed_sections.contains(&key);
        let header = notion_sidebar_section_row(section, key.clone(), !collapsed);
        if top_gap {
            push_notion_sidebar_section_gap(rows, &header.element_id);
        }
        rows.push(NotionSidebarVisibleRow::Section(header));
        if collapsed {
            return;
        }

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

impl SidebarRenderer {
    fn render_notion_sidebar_visible_row(
        &self,
        row: &NotionSidebarVisibleRow,
        cx: &mut App,
    ) -> AnyElement {
        match row {
            NotionSidebarVisibleRow::Gap { height, .. } => {
                div().w_full().h(px(*height)).into_any_element()
            }
            NotionSidebarVisibleRow::Section(row) => div()
                .w_full()
                .px(px(8.0))
                .child(self.render_notion_sidebar_section_header(row, cx))
                .into_any_element(),
            NotionSidebarVisibleRow::Item(row) => div()
                .w_full()
                .px(px(8.0))
                .child(self.render_notion_sidebar_row(row, cx))
                .into_any_element(),
        }
    }

    fn render_notion_sidebar_section_header(
        &self,
        row: &NotionSidebarSectionRow,
        cx: &mut App,
    ) -> AnyElement {
        let group = row.element_id.clone();
        let shell = self.notion_sidebar_section_header_shell(group.clone(), row.key.clone());
        let shell = if row.teamspace {
            self.render_notion_teamspace_header_content(shell, row, cx)
        } else {
            self.render_notion_section_header_content(shell, row, group.clone(), cx)
        };
        shell
            .child(self.render_notion_sidebar_hover_actions(
                group,
                SidebarAction::TogglePageMenu,
                cx,
            ))
            .id(row.element_id.clone())
            .into_any_element()
    }

    fn notion_sidebar_section_header_shell(
        &self,
        group: SharedString,
        key: NotionSidebarSectionKey,
    ) -> Div {
        div()
            .group(group)
            .relative()
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .rounded(px(6.0))
            .pl(px(8.0))
            .pr(px(4.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, _| {
                    SidebarAction::Update(NotionSidebarUpdate::ToggleSection(key.clone()))
                }),
            )
    }

    fn render_notion_teamspace_header_content(
        &self,
        shell: Div,
        row: &NotionSidebarSectionRow,
        cx: &mut App,
    ) -> Div {
        shell
            .child(
                self.notion_sidebar_section_icon_slot(16.0, 4.0)
                    .child(self.render_notion_sidebar_chevron(row.expanded, cx)),
            )
            .child(
                self.notion_sidebar_section_icon_slot(18.0, 6.0)
                    .child(self.render_notion_page_shell_icon(&row.icon, 18.0, cx)),
            )
            .child(self.render_notion_sidebar_section_text(row.label.clone()))
    }

    fn render_notion_section_header_content(
        &self,
        shell: Div,
        row: &NotionSidebarSectionRow,
        group: SharedString,
        cx: &mut App,
    ) -> Div {
        shell
            .child(self.render_notion_sidebar_section_text(row.label.clone()))
            .child(
                self.notion_sidebar_section_icon_slot(16.0, 0.0)
                    .ml(px(4.0))
                    .opacity(0.0)
                    .group_hover(group, |mut style| {
                        style.opacity = Some(1.0);
                        style
                    })
                    .child(self.render_notion_sidebar_chevron(row.expanded, cx)),
            )
    }

    fn notion_sidebar_section_icon_slot(&self, size: f32, right_margin: f32) -> Div {
        div()
            .size(px(size))
            .mr(px(right_margin))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
    }

    fn render_notion_sidebar_section_text(&self, title: SharedString) -> Div {
        div()
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .line_height(relative(1.0))
            .text_color(rgb(self.notion_sidebar_section_text_color()))
            .child(title)
    }

    fn notion_sidebar_section_text_color(&self) -> u32 {
        match self.appearance_mode {
            AppearanceMode::Light => 0xa19e99,
            AppearanceMode::Dark => super::notion_sidebar_muted(self.appearance_mode),
        }
    }
}

impl SidebarRenderer {
    fn render_notion_sidebar_home_list(
        &self,
        state: &NotionSidebarState,
        rows: Arc<[NotionSidebarVisibleRow]>,
    ) -> AnyElement {
        let list_state = state.list_state.clone();
        let renderer = self.clone();
        list(list_state, move |index, _window, cx| {
            let row = rows
                .get(index)
                .expect("notion sidebar row index should exist");
            renderer.render_notion_sidebar_visible_row(row, cx)
        })
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .size_full()
        .pt(px(6.0))
        .pb(px(12.0))
        .into_any_element()
    }
}

fn notion_sidebar_section_row(
    section: &PageShellSidebarSection,
    key: NotionSidebarSectionKey,
    expanded: bool,
) -> NotionSidebarSectionRow {
    NotionSidebarSectionRow {
        element_id: format!("notion-sidebar-section-{key:?}").into(),
        label: section.title.clone().into(),
        icon: section.icon.clone(),
        key,
        expanded,
        teamspace: matches!(
            section.identity.as_ref(),
            Some(PageShellSidebarSectionIdentity::Teamspace { .. })
        ),
    }
}

fn push_notion_sidebar_section_gap(
    rows: &mut Vec<NotionSidebarVisibleRow>,
    section_id: &SharedString,
) {
    rows.push(NotionSidebarVisibleRow::Gap {
        list_key: format!("{section_id}-gap").into(),
        height: SIDEBAR_SECTION_GAP,
    });
}
