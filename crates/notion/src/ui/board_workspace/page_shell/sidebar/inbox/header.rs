use crate::model::{MutateSidebarInboxAction, NotionSidebarInboxFilter};
use gpui::deferred;

use super::super::{
    div, px, AnyElement, Div, FluentBuilder, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, NotionSidebarUpdate, ParentElement, SidebarAction, SidebarInboxAction,
    SidebarRenderer, Styled,
};
use super::icons::{inbox_action_icon, InboxActionIcon};
use crate::ui::surface::NotionSidebarState;
use gpui::App;

/// An inbox header icon button and the action it sends.
struct InboxHeaderButton {
    element_id: &'static str,
    icon: InboxActionIcon,
    active: bool,
    action: SidebarAction,
}

impl SidebarRenderer {
    pub(super) fn render_notion_inbox_header_actions(
        &self,
        state: &NotionSidebarState,
        cx: &mut App,
    ) -> Div {
        let filter = state.inbox_filter;
        let notification_actions = !matches!(
            filter,
            NotionSidebarInboxFilter::Archived | NotionSidebarInboxFilter::WorkspaceUpdates
        );
        div()
            .ml_auto()
            .w(px(if notification_actions { 80.0 } else { 24.0 }))
            .h(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.0))
            .when(notification_actions, |this| {
                this.child(self.render_notion_inbox_header_button(
                    InboxHeaderButton {
                        element_id: "notion-inbox-mark-all-read",
                        icon: InboxActionIcon::Check,
                        active: false,
                        action: SidebarAction::Inbox(SidebarInboxAction::Mutate(
                            MutateSidebarInboxAction::MarkAllRead,
                        )),
                    },
                    cx,
                ))
            })
            .when(notification_actions, |this| {
                this.child(self.render_notion_inbox_archive_menu_trigger(state, cx))
            })
            .child(self.render_notion_inbox_filter_menu_trigger(state, cx))
    }

    fn render_notion_inbox_archive_menu_trigger(
        &self,
        state: &NotionSidebarState,
        cx: &mut App,
    ) -> Div {
        div()
            .relative()
            .size(px(24.0))
            .child(self.render_notion_inbox_header_button(
                InboxHeaderButton {
                    element_id: "notion-inbox-archive-menu-button",
                    icon: InboxActionIcon::Archive,
                    active: state.inbox_archive_menu_open,
                    action: SidebarAction::Update(NotionSidebarUpdate::ToggleInboxArchiveMenu),
                },
                cx,
            ))
            .when(state.inbox_archive_menu_open, |button| {
                button.child(
                    deferred(self.render_notion_inbox_archive_menu(state, cx)).with_priority(100),
                )
            })
    }

    fn render_notion_inbox_filter_menu_trigger(
        &self,
        state: &NotionSidebarState,
        cx: &mut App,
    ) -> Div {
        div()
            .relative()
            .size(px(24.0))
            .child(self.render_notion_inbox_header_button(
                InboxHeaderButton {
                    element_id: "notion-inbox-filter-menu-button",
                    icon: InboxActionIcon::Filter,
                    active: state.inbox_filter_menu_open,
                    action: SidebarAction::Update(NotionSidebarUpdate::ToggleInboxFilterMenu),
                },
                cx,
            ))
            .when(state.inbox_filter_menu_open, |button| {
                button.child(
                    deferred(self.render_notion_inbox_filter_menu(state, cx)).with_priority(100),
                )
            })
    }

    fn render_notion_inbox_header_button(
        &self,
        button: InboxHeaderButton,
        cx: &mut App,
    ) -> AnyElement {
        let InboxHeaderButton {
            element_id,
            icon,
            active,
            action,
        } = button;
        div()
            .id(element_id)
            .size(px(24.0))
            .rounded(px(5.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .when(active, |this| this.bg(self.notion_sidebar_hover_bg()))
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    action.clone()
                }),
            )
            .child(inbox_action_icon(
                icon,
                self.notion_sidebar_row_text_color(),
                16.0,
                cx,
            ))
            .into_any_element()
    }
}
