use crate::model::MutateSidebarInboxAction;
use gpui::SharedString;

use super::super::{
    alpha, div, px, rgb, Div, FluentBuilder, InteractiveElement, MouseButton, MouseDownEvent,
    PageShellInboxItem, ParentElement, SidebarAction, SidebarInboxAction, SidebarRenderer, Styled,
};
use super::icons::{inbox_action_icon, InboxActionIcon};
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_inbox_item_actions(
        &self,
        item: &PageShellInboxItem,
        group: SharedString,
        cx: &mut App,
    ) -> Div {
        let notification_id = item.notification_id.clone();
        let read = item.read;
        let archived = item.archived;
        div()
            .absolute()
            .right(px(8.0))
            .top(px(10.0))
            .h(px(30.0))
            .w(px(if archived { 30.0 } else { 56.0 }))
            .p(px(2.0))
            .gap(px(1.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgb(0x383836))
            .bg(rgb(0x2f2f2f))
            .flex()
            .items_center()
            .opacity(0.0)
            .group_hover(group, |mut style| {
                style.opacity = Some(1.0);
                style
            })
            .when(!archived, |this| {
                let notification_id = notification_id.clone();
                this.child(self.render_notion_inbox_item_read_action(notification_id, read, cx))
            })
            .child(self.render_notion_inbox_item_archive_action(notification_id, archived, cx))
    }

    fn render_notion_inbox_item_read_action(
        &self,
        notification_id: String,
        read: bool,
        cx: &mut App,
    ) -> Div {
        self.render_notion_inbox_item_action_button(
            if read {
                InboxActionIcon::Unread
            } else {
                InboxActionIcon::Read
            },
            cx,
            SidebarAction::Inbox(SidebarInboxAction::Mutate(
                MutateSidebarInboxAction::SetRead {
                    notification_ids: vec![notification_id.clone()],
                    read: !read,
                },
            )),
        )
    }

    fn render_notion_inbox_item_archive_action(
        &self,
        notification_id: String,
        archived: bool,
        cx: &mut App,
    ) -> Div {
        self.render_notion_inbox_item_action_button(
            if archived {
                InboxActionIcon::Unarchive
            } else {
                InboxActionIcon::Archive
            },
            cx,
            SidebarAction::Inbox(SidebarInboxAction::Mutate(
                MutateSidebarInboxAction::SetArchived {
                    notification_ids: vec![notification_id.clone()],
                    archived: !archived,
                },
            )),
        )
    }

    fn render_notion_inbox_item_action_button(
        &self,
        icon: InboxActionIcon,
        cx: &mut App,
        action: SidebarAction,
    ) -> Div {
        div()
            .size(px(24.0))
            .rounded(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0xffffff, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    action.clone()
                }),
            )
            .child(inbox_action_icon(icon, 0xbfbfbc, 16.0, cx))
    }
}
