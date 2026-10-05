use crate::ui::board_workspace::ShareEvent;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled,
};

use super::{TopBarAction, TopBarRenderer, TopBarViewState};
use crate::ui::{div, px, view_actions::ViewActionSink, SurfaceState};

impl SurfaceState {
    pub(crate) fn top_bar_renderer(&self, cx: &Context<Self>) -> TopBarRenderer<'_> {
        TopBarRenderer {
            view: TopBarViewState {
                page_title: &self.board.page_title,
                database_title: &self.board.database_title,
                page_shell: self.board.page_shell.as_ref(),
                is_standalone: self.board.page_content.is_some(),
                is_private: self.board.is_private,
                is_locked: self.board.is_locked,
                edited_label: &self.board.edited_label,
                presence: self.board.presence.as_ref(),
                board_url: self.notion_startup.board_url(),
                sidebar_shown: self.board.page_shell.is_some()
                    && self.notion_chrome.notion_sidebar_visible,
                favorited: self.favorited,
                ai_open: self.notion_chrome.notion_ai_open,
            },
            theme: self.theme,
            icons: &self.icons,
            page_icons: self.page_shell_icon_renderer(cx),
            presence_images: &self.presence_images,
            actions: ViewActionSink::new(cx, |this, action, _, cx| match action {
                TopBarAction::ToggleSidebar => this.notion_chrome.toggle_notion_sidebar(cx),
                TopBarAction::Share => this.dispatch_notion_share_event(ShareEvent::Open, cx),
                TopBarAction::ToggleFavorite => this.toggle_favorite(cx),
                TopBarAction::OpenDialog(dialog) => this.open_toolbar_dialog(dialog, cx),
            }),
        }
    }

    pub(crate) fn render_view_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        (div()
            .id("notion-database-view-tabs")
            .role(gpui::Role::TabList)
            .aria_label("Database views")
            .flex()
            .items_center()
            .gap(px(1.0))
            .children(
                self.board
                    .view_tabs
                    .iter()
                    .enumerate()
                    .map(|(index, tab)| {
                        self.board_toolbar_renderer(None, cx)
                            .view_tab(index, tab, None, cx)
                    })
                    .collect::<Vec<_>>(),
            ))
        .into_any_element()
    }
}
