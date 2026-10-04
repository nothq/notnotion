use crate::model::{MutateSidebarInboxAction, NotionSidebarInboxFilter};
use gpui::{ClickEvent, StatefulInteractiveElement};

use super::super::{
    alpha, div, point, px, rgb, AnyElement, BoxShadow, Div, FluentBuilder, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, NotionSidebarUpdate, ParentElement, SidebarAction,
    SidebarInboxAction, SidebarRenderer, Styled,
};
use super::icons::{inbox_action_icon, InboxActionIcon};
use crate::ui::surface::NotionSidebarState;
use gpui::App;

/// An inbox filter menu row: the filter it selects, its label, and its icon.
struct InboxFilterOption {
    filter: NotionSidebarInboxFilter,
    label: &'static str,
    icon: InboxActionIcon,
}

/// An archive menu row, and whether it archives only read notifications.
struct InboxArchiveOption {
    element_id: &'static str,
    label: &'static str,
    read_only: bool,
}

impl SidebarRenderer {
    pub(super) fn render_notion_inbox_filter_menu(
        &self,
        state: &NotionSidebarState,
        cx: &mut App,
    ) -> AnyElement {
        let filter = state.inbox_filter;
        div()
            .id("notion-inbox-filter-menu")
            .absolute()
            .left(px(0.0))
            .top(px(26.0))
            .w(px(250.0))
            .p(px(4.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(self.notion_inbox_popover_border_color()))
            .bg(rgb(self.notion_inbox_popover_bg_color()))
            .shadow(notion_inbox_popover_shadow())
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(self.render_notion_inbox_filter_menu_title())
            .child(self.render_notion_inbox_filter_row(
                InboxFilterOption {
                    filter: NotionSidebarInboxFilter::All,
                    label: "Unread & read",
                    icon: InboxActionIcon::Inbox,
                },
                filter,
                cx,
            ))
            .child(self.render_notion_inbox_filter_row(
                InboxFilterOption {
                    filter: NotionSidebarInboxFilter::Unread,
                    label: "Unread",
                    icon: InboxActionIcon::Unread,
                },
                filter,
                cx,
            ))
            .child(self.render_notion_inbox_filter_row(
                InboxFilterOption {
                    filter: NotionSidebarInboxFilter::Archived,
                    label: "Archived",
                    icon: InboxActionIcon::Archive,
                },
                filter,
                cx,
            ))
            .child(self.render_notion_inbox_filter_row(
                InboxFilterOption {
                    filter: NotionSidebarInboxFilter::WorkspaceUpdates,
                    label: "All workspace updates",
                    icon: InboxActionIcon::Clock,
                },
                filter,
                cx,
            ))
            .into_any_element()
    }

    fn render_notion_inbox_filter_menu_title(&self) -> Div {
        div()
            .h(px(24.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .line_height(px(18.0))
            .text_color(rgb(super::super::notion_sidebar_muted(
                self.appearance_mode,
            )))
            .child("Filter")
    }

    fn render_notion_inbox_filter_row(
        &self,
        option: InboxFilterOption,
        active_filter: NotionSidebarInboxFilter,
        cx: &mut App,
    ) -> AnyElement {
        let InboxFilterOption {
            filter: row_filter,
            label,
            icon,
        } = option;
        div()
            .id(format!("notion-inbox-filter-{row_filter:?}"))
            .w_full()
            .h(px(29.0))
            .px(px(8.0))
            .rounded(px(5.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .on_click(self.actions.listener(move |_: &ClickEvent, _, cx| {
                cx.stop_propagation();
                SidebarAction::Update(NotionSidebarUpdate::SelectInboxFilter(row_filter))
            }))
            .child(inbox_action_icon(
                icon,
                self.notion_sidebar_top_text_color(),
                16.0,
                cx,
            ))
            .child(
                div()
                    .flex_grow(1.0)
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(self.notion_sidebar_top_text_color()))
                    .child(label),
            )
            .when(row_filter == active_filter, |this| {
                this.child(inbox_action_icon(
                    InboxActionIcon::Check,
                    self.notion_sidebar_top_text_color(),
                    16.0,
                    cx,
                ))
            })
            .into_any_element()
    }

    pub(super) fn render_notion_inbox_archive_menu(
        &self,
        state: &NotionSidebarState,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id("notion-inbox-archive-menu")
            .absolute()
            .left(px(0.0))
            .top(px(26.0))
            .w(px(250.0))
            .p(px(4.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(self.notion_inbox_popover_border_color()))
            .bg(rgb(self.notion_inbox_popover_bg_color()))
            .shadow(notion_inbox_popover_shadow())
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(self.render_notion_inbox_archive_menu_row(
                InboxArchiveOption {
                    element_id: "notion-inbox-archive-all",
                    label: "Archive all",
                    read_only: false,
                },
                state.inbox_filter,
                cx,
            ))
            .when(
                state.inbox_filter == NotionSidebarInboxFilter::All,
                |this| {
                    this.child(self.render_notion_inbox_archive_menu_row(
                        InboxArchiveOption {
                            element_id: "notion-inbox-archive-read",
                            label: "Archive read",
                            read_only: true,
                        },
                        state.inbox_filter,
                        cx,
                    ))
                },
            )
            .into_any_element()
    }

    fn render_notion_inbox_archive_menu_row(
        &self,
        option: InboxArchiveOption,
        filter: NotionSidebarInboxFilter,
        cx: &mut App,
    ) -> AnyElement {
        let InboxArchiveOption {
            element_id,
            label,
            read_only,
        } = option;
        div()
            .id(element_id)
            .w_full()
            .h(px(29.0))
            .px(px(8.0))
            .rounded(px(5.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .cursor_pointer()
            .hover(|style| style.bg(self.notion_sidebar_hover_bg()))
            .on_click(self.actions.listener(move |_: &ClickEvent, _, cx| {
                cx.stop_propagation();
                SidebarAction::Inbox(SidebarInboxAction::Mutate(
                    MutateSidebarInboxAction::ArchiveAll { filter, read_only },
                ))
            }))
            .child(inbox_action_icon(
                if read_only {
                    InboxActionIcon::ArchiveRead
                } else {
                    InboxActionIcon::Archive
                },
                self.notion_sidebar_top_text_color(),
                16.0,
                cx,
            ))
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(self.notion_sidebar_top_text_color()))
                    .child(label),
            )
            .into_any_element()
    }

    fn notion_inbox_popover_bg_color(&self) -> u32 {
        match self.appearance_mode {
            app_model::AppearanceMode::Light => 0xffffff,
            app_model::AppearanceMode::Dark => 0x252525,
        }
    }

    fn notion_inbox_popover_border_color(&self) -> u32 {
        match self.appearance_mode {
            app_model::AppearanceMode::Light => 0xe6e6e4,
            app_model::AppearanceMode::Dark => 0x383836,
        }
    }
}

fn notion_inbox_popover_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: alpha(0x000000, 0.12),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
            inset: false,
        },
        BoxShadow {
            color: alpha(0x000000, 0.08),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(2.0),
            spread_radius: px(0.0),
            inset: false,
        },
    ]
}
