use crate::model::NotionSidebarInboxFilter;

use super::super::{
    alpha, div, px, rgb, AnyElement, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, NotionSidebarUpdate, ParentElement, SidebarAction,
    SidebarRenderer, Styled,
};
use super::icons::inbox_icon;
use gpui::App;

impl SidebarRenderer {
    pub(super) fn render_notion_inbox_skeleton(&self) -> AnyElement {
        let rows = (0..5).map(|index| {
            div()
                .id(format!("notion-inbox-skeleton-{index}"))
                .w_full()
                .h(px(76.0))
                .px(px(8.0))
                .py(px(10.0))
                .flex()
                .items_start()
                .child(
                    div()
                        .size(px(24.0))
                        .mr(px(8.0))
                        .rounded_full()
                        .bg(alpha(self.notion_sidebar_row_text_color(), 0.10)),
                )
                .child(
                    div()
                        .flex_grow(1.0)
                        .child(
                            div()
                                .mt(px(2.0))
                                .w(px(126.0))
                                .h(px(9.0))
                                .rounded(px(5.0))
                                .bg(alpha(self.notion_sidebar_row_text_color(), 0.10)),
                        )
                        .child(
                            div()
                                .mt(px(9.0))
                                .w(px(170.0))
                                .h(px(11.0))
                                .rounded(px(5.0))
                                .bg(alpha(self.notion_sidebar_row_text_color(), 0.13)),
                        )
                        .child(
                            div()
                                .mt(px(8.0))
                                .w(px(142.0))
                                .h(px(9.0))
                                .rounded(px(5.0))
                                .bg(alpha(self.notion_sidebar_row_text_color(), 0.08)),
                        ),
                )
        });
        div().w_full().children(rows).into_any_element()
    }

    pub(super) fn render_notion_inbox_empty(
        &self,
        filter: NotionSidebarInboxFilter,
        cx: &mut App,
    ) -> AnyElement {
        let (title, description) = match filter {
            NotionSidebarInboxFilter::All | NotionSidebarInboxFilter::Unread => {
                ("You're all caught up", None)
            }
            NotionSidebarInboxFilter::Archived => ("Archived notifications go here", None),
            NotionSidebarInboxFilter::WorkspaceUpdates => (
                "No activity in the workspace",
                Some("Any activity in the workspace will appear here"),
            ),
        };
        div()
            .w_full()
            .min_h(px(220.0))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .px(px(20.0))
            .text_color(rgb(super::super::notion_sidebar_muted(
                self.appearance_mode,
            )))
            .child(div().pb(px(6.0)).child(self.render_notion_page_shell_icon(
                &inbox_icon(),
                28.0,
                cx,
            )))
            .child(
                div()
                    .mt(px(2.0))
                    .mb(px(4.0))
                    .text_size(px(14.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.notion_sidebar_top_text_color()))
                    .child(title),
            )
            .when_some(description, |this, description| {
                this.child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(18.0))
                        .text_align(gpui::TextAlign::Center)
                        .child(description),
                )
            })
            .child(self.render_notion_inbox_empty_filter_button(cx))
            .into_any_element()
    }

    fn render_notion_inbox_empty_filter_button(&self, _cx: &mut App) -> gpui::Stateful<Div> {
        div()
            .id("notion-inbox-empty-edit-filter")
            .mt(px(16.0))
            .h(px(28.0))
            .px(px(10.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(14.0))
            .text_color(rgb(self.notion_sidebar_top_text_color()))
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    SidebarAction::Update(NotionSidebarUpdate::ToggleInboxFilterMenu)
                }),
            )
            .child("Edit filter")
    }
}
