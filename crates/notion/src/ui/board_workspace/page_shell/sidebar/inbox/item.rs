use crate::model::NotionSidebarInboxFilter;
use gpui::{HighlightStyle, ObjectFit, SharedString, StyledImage, StyledText};

use super::super::{
    alpha, div, img, px, rgb, AnyElement, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, PageShellInboxItem, ParentElement, SidebarAction,
    SidebarInboxAction, SidebarRenderer, Styled,
};
use super::icons::inbox_target_icon;
use super::time::{inbox_action_label, inbox_time_label};
use crate::ui::surface::NotionSidebarState;
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_inbox_item(
        &self,
        state: &NotionSidebarState,
        item: &PageShellInboxItem,
        cx: &mut App,
    ) -> AnyElement {
        let element_id = format!("notion-inbox-item-{}", item.notification_id);
        let group = SharedString::from(element_id.clone());
        let card = self
            .render_notion_inbox_item_card(item, group.clone(), cx)
            .when(
                state.inbox_filter != NotionSidebarInboxFilter::WorkspaceUpdates,
                |card| card.child(self.render_notion_inbox_item_actions(item, group, cx)),
            );
        self.bind_notion_inbox_item_navigation(card, item, cx)
            .id(element_id)
            .into_any_element()
    }

    fn render_notion_inbox_item_card(
        &self,
        item: &PageShellInboxItem,
        group: SharedString,
        cx: &mut App,
    ) -> Div {
        div()
            .group(group)
            .relative()
            .w_full()
            .min_h(px(84.0))
            .border_b_1()
            .border_color(rgb(self.notion_inbox_separator_color()))
            .rounded(px(8.0))
            .pl(px(8.0))
            .pr(px(18.0))
            .py(px(8.0))
            .flex()
            .items_start()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .when(!item.read, |this| {
                this.child(
                    div()
                        .absolute()
                        .right(px(10.0))
                        .top(px(19.0))
                        .size(px(8.0))
                        .rounded_full()
                        .bg(rgb(0x2783de)),
                )
            })
            .child(self.render_notion_inbox_avatar(item))
            .child(self.render_notion_inbox_item_content(item, cx))
    }

    fn render_notion_inbox_item_content(&self, item: &PageShellInboxItem, cx: &mut App) -> Div {
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .child(self.render_notion_inbox_item_actor_row(item))
            .child(self.render_notion_inbox_item_target_row(item, cx))
            .when_some(item.body.clone(), |content, body| {
                content.child(
                    div()
                        .w_full()
                        .max_h(px(63.0))
                        .overflow_hidden()
                        .text_size(px(14.0))
                        .line_height(px(21.0))
                        .text_color(rgb(self.notion_sidebar_row_text_color()))
                        .child(body),
                )
            })
    }

    fn render_notion_inbox_item_actor_row(&self, item: &PageShellInboxItem) -> Div {
        let action = inbox_action_label(&item.notification_type);
        let actor = item
            .actor
            .as_ref()
            .map(|actor| actor.name.as_str())
            .unwrap_or("Notion");
        let actor_action = format!("{actor} {action}");
        let actor_range = 0..actor.len();
        div()
            .w_full()
            .flex()
            .items_start()
            .child(
                div()
                    .min_w(px(0.0))
                    .flex_grow(1.0)
                    .max_h(px(42.0))
                    .overflow_hidden()
                    .text_size(px(14.0))
                    .line_height(px(21.0))
                    .text_color(rgb(self.notion_sidebar_row_text_color()))
                    .child(StyledText::new(actor_action).with_highlights(vec![(
                        actor_range,
                        HighlightStyle {
                            color: Some(rgb(self.notion_sidebar_top_text_color()).into()),
                            font_weight: Some(FontWeight::MEDIUM),
                            ..Default::default()
                        },
                    )])),
            )
            .when_some(item.event_time_ms, |row, timestamp| {
                row.child(
                    div()
                        .ml(px(8.0))
                        .flex_none()
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_color(rgb(super::super::notion_sidebar_muted(
                            self.appearance_mode,
                        )))
                        .child(inbox_time_label(timestamp)),
                )
            })
    }

    fn render_notion_inbox_item_target_row(&self, item: &PageShellInboxItem, cx: &mut App) -> Div {
        div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(4.0))
            .text_size(px(14.0))
            .line_height(px(21.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.notion_sidebar_top_text_color()))
            .child(self.render_notion_page_shell_icon(&inbox_target_icon(), 16.0, cx))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(item.title.clone()),
            )
    }

    fn bind_notion_inbox_item_navigation(
        &self,
        card: Div,
        item: &PageShellInboxItem,
        _cx: &mut App,
    ) -> Div {
        let Some(board_url) = item.target_board_url.clone() else {
            return card;
        };
        let target_label = item.title.clone();
        let notification_id = item.notification_id.clone();
        let unread_notification_id = (!item.read).then_some(notification_id);
        card.cursor_pointer().on_mouse_down(
            MouseButton::Left,
            self.actions.listener(move |_: &MouseDownEvent, _, _| {
                SidebarAction::Inbox(SidebarInboxAction::OpenInboxTarget {
                    board_url: board_url.clone(),
                    label: target_label.clone(),
                    unread_notification_id: unread_notification_id.clone(),
                })
            }),
        )
    }

    fn render_notion_inbox_avatar(&self, item: &PageShellInboxItem) -> Div {
        let actor_name = item
            .actor
            .as_ref()
            .map(|actor| actor.name.as_str())
            .unwrap_or("Notion");
        let initial = actor_name
            .chars()
            .find(|character| character.is_alphanumeric())
            .map(|character| character.to_uppercase().to_string())
            .unwrap_or_else(|| "N".to_string());
        let avatar_url = item
            .actor
            .as_ref()
            .and_then(|actor| actor.avatar_url.as_ref())
            .cloned();
        div()
            .relative()
            .size(px(24.0))
            .mr(px(12.0))
            .mt(px(2.0))
            .flex_none()
            .rounded_full()
            .overflow_hidden()
            .bg(alpha(self.notion_sidebar_row_text_color(), 0.12))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(self.notion_sidebar_row_text_color()))
            .child(initial)
            .when_some(avatar_url, |avatar, url| {
                avatar.child(
                    img(url)
                        .absolute()
                        .inset_0()
                        .size_full()
                        .object_fit(ObjectFit::Cover)
                        .with_loading(|| div().size_full().into_any_element())
                        .with_fallback(|| div().size_full().into_any_element()),
                )
            })
    }

    fn notion_inbox_separator_color(&self) -> u32 {
        match self.appearance_mode {
            app_model::AppearanceMode::Light => 0xe9e9e7,
            app_model::AppearanceMode::Dark => 0x2c2c2b,
        }
    }
}
