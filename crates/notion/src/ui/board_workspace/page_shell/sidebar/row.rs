use super::{
    div, px, relative, rgb, AnyElement, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, NotionSidebarRow, NotionSidebarRowAction,
    NotionSidebarRowLayout, NotionSidebarTab, NotionSidebarUpdate, ParentElement, SidebarAction,
    SidebarNavigationAction, SidebarRenderer, StatefulInteractiveElement, Styled,
    SIDEBAR_ROW_ADVANCE, SIDEBAR_ROW_HEIGHT,
};
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_sidebar_row(
        &self,
        row: &NotionSidebarRow,
        cx: &mut App,
    ) -> AnyElement {
        match row.layout {
            NotionSidebarRowLayout::TopTab(tab) => self.render_notion_sidebar_top_tab(row, tab, cx),
            NotionSidebarRowLayout::TopIcon => self.render_notion_sidebar_top_icon(row, cx),
            NotionSidebarRowLayout::PanelHeader => self.render_notion_sidebar_panel_header(row, cx),
            NotionSidebarRowLayout::CalendarEvent => {
                self.render_notion_sidebar_calendar_event(row, cx)
            }
            NotionSidebarRowLayout::PanelEmpty => self.render_notion_sidebar_panel_empty(row, cx),
            NotionSidebarRowLayout::PanelAction => self.render_notion_sidebar_panel_action(row, cx),
            NotionSidebarRowLayout::Skeleton => self.render_notion_sidebar_skeleton(row),
            NotionSidebarRowLayout::ChatAgents => self.render_notion_sidebar_chat_agents(row, cx),
            NotionSidebarRowLayout::ChatThread => self.render_notion_sidebar_chat_thread(row, cx),
            NotionSidebarRowLayout::Tree => self.render_notion_sidebar_tree_row(row, cx),
        }
    }

    fn render_notion_sidebar_top_tab(
        &self,
        row: &NotionSidebarRow,
        tab: NotionSidebarTab,
        cx: &mut App,
    ) -> AnyElement {
        let width = self.notion_sidebar_top_tab_width(row.active, tab);
        let show_label = row.active && width > 32.0;
        let element = div()
            .w(px(width))
            .min_w(px(width))
            .max_w(px(width))
            .h(px(32.0))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .when(row.active, |this| {
                this.bg(self.notion_sidebar_selected_bg())
            })
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .child(
                div()
                    .size(px(20.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_notion_page_shell_icon(&row.icon, 20.0, cx)),
            )
            .when(show_label, |this| {
                this.child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(14.0))
                        .font_weight(FontWeight::MEDIUM)
                        .line_height(relative(1.2))
                        .text_color(rgb(self.notion_sidebar_top_text_color()))
                        .child(row.label.clone()),
                )
            });
        self.bind_notion_sidebar_row_action(element, &row.action, cx)
            .id(row.element_id.clone())
            .into_any_element()
    }

    fn notion_sidebar_top_tab_width(&self, active: bool, tab: NotionSidebarTab) -> f32 {
        if !active {
            return 32.0;
        }
        let selected_width = notion_sidebar_selected_tab_width(tab);
        let required_sidebar_width = selected_width + 156.0;
        if self.width >= required_sidebar_width {
            selected_width
        } else {
            32.0
        }
    }

    fn render_notion_sidebar_top_icon(&self, row: &NotionSidebarRow, cx: &mut App) -> AnyElement {
        let element = div()
            .size(px(32.0))
            .rounded_full()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(row.active, |this| {
                this.bg(self.notion_sidebar_selected_bg())
            })
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .child(self.render_notion_page_shell_icon(&row.icon, 20.0, cx));
        self.bind_notion_sidebar_row_action(element, &row.action, cx)
            .id(row.element_id.clone())
            .into_any_element()
    }

    fn render_notion_sidebar_tree_row(&self, row: &NotionSidebarRow, cx: &mut App) -> AnyElement {
        let group = row.element_id.clone();
        let element = div()
            .group(group.clone())
            .relative()
            .w_full()
            .h(px(SIDEBAR_ROW_HEIGHT))
            .mb(px(SIDEBAR_ROW_ADVANCE - SIDEBAR_ROW_HEIGHT))
            .rounded(px(6.0))
            .pl(px(8.0 + row.depth as f32 * 8.0))
            .pr(px(4.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .when(row.active, |this| {
                this.bg(self.notion_sidebar_selected_bg())
            })
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .child(self.render_notion_sidebar_tree_icon(row, &group, cx))
            .child(self.render_notion_sidebar_tree_label(row))
            .child(self.render_notion_sidebar_hover_actions(
                group,
                SidebarAction::TogglePageMenu,
                cx,
            ));
        let element = self
            .bind_notion_sidebar_row_action(element, &row.action, cx)
            .id(row.element_id.clone());
        match &row.action {
            NotionSidebarRowAction::OpenWorkspace { board_url, .. } => {
                let board_url = board_url.clone();
                element
                    .on_hover(self.actions.listener(move |hovered: &bool, _, _| {
                        SidebarAction::Navigate(SidebarNavigationAction::SetWorkspacePrefetch {
                            board_url: board_url.clone(),
                            hovered: *hovered,
                        })
                    }))
                    .into_any_element()
            }
            _ => element.into_any_element(),
        }
    }

    fn render_notion_sidebar_tree_icon(
        &self,
        row: &NotionSidebarRow,
        group: &gpui::SharedString,
        cx: &mut App,
    ) -> Div {
        let slot = div()
            .relative()
            .w(px(22.0))
            .h(px(18.0))
            .mr(px(8.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center();
        if !row.has_children {
            return slot.child(self.render_notion_page_shell_icon(&row.icon, 18.0, cx));
        }
        let node_key = row
            .node_key
            .clone()
            .expect("expandable sidebar row requires a node key");
        let icon = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .opacity(1.0)
            .group_hover(group.clone(), |mut style| {
                style.opacity = Some(0.0);
                style
            })
            .child(self.render_notion_page_shell_icon(&row.icon, 18.0, cx));
        let chevron = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .opacity(0.0)
            .group_hover(group.clone(), |mut style| {
                style.opacity = Some(1.0);
                style
            })
            .child(self.render_notion_sidebar_chevron(row.expanded, cx));
        slot.cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    SidebarAction::Update(NotionSidebarUpdate::ToggleNode(node_key.clone()))
                }),
            )
            .child(icon)
            .child(chevron)
    }

    fn render_notion_sidebar_tree_label(&self, row: &NotionSidebarRow) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .line_height(relative(1.5))
            .text_color(rgb(if row.active {
                self.notion_sidebar_selected_text_color()
            } else {
                self.notion_sidebar_row_text_color()
            }))
            .child(row.label.clone())
    }

    pub(super) fn bind_notion_sidebar_row_action(
        &self,
        element: Div,
        action: &NotionSidebarRowAction,
        cx: &mut App,
    ) -> Div {
        match action {
            NotionSidebarRowAction::ActivateTab(tab) => {
                let tab = *tab;
                element.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    self.actions.listener(move |_: &MouseDownEvent, _, _| {
                        SidebarAction::Update(NotionSidebarUpdate::ActivateTab(tab))
                    }),
                )
            }
            NotionSidebarRowAction::OpenWorkspace { board_url, label } => self
                .bind_notion_sidebar_workspace_action(
                    element,
                    board_url.clone(),
                    label.clone(),
                    cx,
                ),
            NotionSidebarRowAction::OpenUrl(url) => {
                let url = url.clone();
                element.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    self.actions.listener(move |_: &MouseDownEvent, _, _| {
                        SidebarAction::Navigate(SidebarNavigationAction::OpenUrl(url.clone()))
                    }),
                )
            }
            NotionSidebarRowAction::ToggleMeetingsMore => element.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, _| {
                    SidebarAction::Update(NotionSidebarUpdate::ShowAllMeetings)
                }),
            ),
            NotionSidebarRowAction::ToggleSearch => element.cursor_pointer().on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| SidebarAction::ToggleSearch),
            ),
            NotionSidebarRowAction::OpenChatThread(thread_id) => {
                self.bind_notion_sidebar_chat_action(element, thread_id.clone(), cx)
            }
            NotionSidebarRowAction::None => element,
        }
    }

    fn bind_notion_sidebar_workspace_action(
        &self,
        element: Div,
        board_url: String,
        label: gpui::SharedString,
        _cx: &mut App,
    ) -> Div {
        element.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            self.actions.listener(move |_: &MouseDownEvent, _, _| {
                SidebarAction::Navigate(SidebarNavigationAction::OpenWorkspace {
                    board_url: board_url.clone(),
                    label: label.clone(),
                })
            }),
        )
    }

    fn bind_notion_sidebar_chat_action(
        &self,
        element: Div,
        thread_id: gpui::SharedString,
        _cx: &mut App,
    ) -> Div {
        element.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            self.actions.listener(move |_: &MouseDownEvent, _, _| {
                SidebarAction::OpenChatThread(thread_id.clone())
            }),
        )
    }
}

fn notion_sidebar_selected_tab_width(tab: NotionSidebarTab) -> f32 {
    match tab {
        NotionSidebarTab::Home => 87.0,
        NotionSidebarTab::Chat => 79.2,
        NotionSidebarTab::Meetings => 103.0,
        NotionSidebarTab::Inbox => 84.0,
    }
}
