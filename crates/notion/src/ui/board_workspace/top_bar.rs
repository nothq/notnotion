use super::PageShellIconRenderer;
use super::{
    div, img, px, render_top_bar_chip_impl, render_top_bar_locked_chip_impl,
    render_top_bar_menu_button_impl, render_top_bar_private_chip_impl, rgb, rgba, AnyElement,
    ClipboardItem, Div, FluentBuilder, IconAsset, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, ToolbarDialogKind,
    TopBarPrivateChipOpacity,
};
use crate::model::{PagePresenceSnapshot, PageShellSnapshot};
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{IconSet, Theme};
use gpui::{App, Image};
use std::sync::Arc;

mod host;
mod presence;

pub(super) enum TopBarAction {
    ToggleSidebar,
    Share,
    ToggleFavorite,
    OpenDialog(ToolbarDialogKind),
}

struct TopBarViewState<'a> {
    page_title: &'a str,
    database_title: &'a str,
    page_shell: Option<&'a PageShellSnapshot>,
    is_standalone: bool,
    is_private: bool,
    is_locked: bool,
    edited_label: &'a str,
    presence: Option<&'a PagePresenceSnapshot>,
    board_url: Option<String>,
    sidebar_shown: bool,
    favorited: bool,
    ai_open: bool,
}

/// Where the top bar starts when no sidebar sits under the window controls
/// notnotion draws in the surface's top-left corner on macOS.
const TOP_BAR_WINDOW_CONTROLS_INSET: f32 = if cfg!(target_os = "macos") {
    76.0
} else {
    12.0
};

pub(crate) struct TopBarRenderer<'a> {
    view: TopBarViewState<'a>,
    theme: Theme,
    icons: &'a IconSet,
    page_icons: PageShellIconRenderer,
    presence_images: &'a [Option<Arc<Image>>],
    actions: ViewActionSink<TopBarAction>,
}

impl TopBarRenderer<'_> {
    pub(crate) fn render_top_bar(&self, cx: &mut App) -> Div {
        div()
            .h(px(44.0))
            .w_full()
            .pl(px(if self.view.sidebar_shown {
                12.0
            } else {
                TOP_BAR_WINDOW_CONTROLS_INSET
            }))
            .pr(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(rgb(self.theme.app_bg))
            .child(self.render_top_bar_title_row(cx))
            .when(!self.view.ai_open, |this| {
                this.child(self.render_top_bar_actions(cx))
            })
    }

    fn render_top_bar_title_row(&self, cx: &mut App) -> Div {
        let mut row = self.render_top_bar_base_title_row(cx);
        if self.view.page_title != self.view.database_title {
            row = row
                .child(
                    div()
                        .px(px(6.0))
                        .text_size(px(14.0))
                        .text_color(rgb(self.theme.text_secondary))
                        .child("/"),
                )
                .child(self.render_top_bar_chip(self.view.database_title, self.theme.text_primary));
        }
        if self.view.is_private {
            row = row.child(self.render_top_bar_private_chip(cx));
        }
        if self.view.is_locked {
            row = row.child(self.render_top_bar_locked_chip(cx));
        }
        row
    }

    fn render_top_bar_base_title_row(&self, cx: &mut App) -> Div {
        if let Some(page_shell) = self.view.page_shell.filter(|_| self.view.is_standalone) {
            return div()
                .flex()
                .items_center()
                .text_color(rgb(self.theme.text_primary))
                .when(
                    !page_shell.builtin_links.is_empty() || !page_shell.sidebar_sections.is_empty(),
                    |this| this.child(self.render_top_bar_menu_button(cx)),
                )
                .child(
                    div()
                        .size(px(20.0))
                        .rounded(px(4.0))
                        .bg(rgba(self.theme.page_icon_bg))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(self.page_icons.sidebar_builtin("home", cx)),
                )
                .child(
                    self.render_top_bar_chip(&page_shell.workspace_name, self.theme.text_primary),
                )
                .child(
                    div()
                        .px(px(6.0))
                        .text_size(px(14.0))
                        .text_color(rgb(self.theme.text_secondary))
                        .child("/"),
                )
                .child(self.page_icons.render(&page_shell.page_icon, 16.0, cx))
                .child(self.render_top_bar_chip(self.view.page_title, self.theme.text_primary));
        }
        div()
            .flex()
            .items_center()
            .text_color(rgb(self.theme.text_primary))
            .child(self.render_top_bar_menu_button(cx))
            .child(self.render_top_bar_chip(self.view.page_title, self.theme.text_primary))
    }

    fn render_top_bar_actions(&self, cx: &mut App) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_top_bar_button(self.view.edited_label, self.theme.text_secondary))
            .when_some(self.view.presence, |this, presence| {
                this.child(self.render_top_bar_presence(presence))
            })
            .child(self.render_top_bar_share_button(cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(self.render_top_bar_copy_link_button(cx))
                    .child(self.render_top_bar_favorite_button(cx))
                    .child(self.render_top_bar_icon_button(
                        &self.icons.topbar_actions,
                        cx,
                        false,
                        Some(ToolbarDialogKind::Actions),
                    )),
            )
    }

    pub(crate) fn render_top_bar_menu_button(&self, cx: &mut App) -> AnyElement {
        let button = render_top_bar_menu_button_impl(self.icons, 1.0, cx).when(
            self.view.page_shell.is_some(),
            |this| {
                this.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    self.actions
                        .listener(|_: &MouseDownEvent, _, _| TopBarAction::ToggleSidebar),
                )
            },
        );
        (button).into_any_element()
    }

    pub(crate) fn render_top_bar_chip(&self, label: &str, color: u32) -> Div {
        render_top_bar_chip_impl(label, color, 1.0)
    }

    pub(crate) fn render_top_bar_private_chip(&self, cx: &mut App) -> Div {
        render_top_bar_private_chip_impl(
            self.theme,
            self.icons,
            TopBarPrivateChipOpacity::new(1.0, 1.0, 1.0),
            cx,
        )
    }

    pub(crate) fn render_top_bar_locked_chip(&self, cx: &mut App) -> Div {
        render_top_bar_locked_chip_impl(self.theme, self.icons, 1.0, 1.0, cx)
    }

    pub(crate) fn render_top_bar_button(&self, label: &str, color: u32) -> Div {
        div()
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .text_color(rgb(color))
            .child(label.to_string())
    }

    pub(crate) fn render_top_bar_share_button(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-topbar-share")
            .role(gpui::Role::Button)
            .aria_label("Share")
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| TopBarAction::Share),
            )
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_primary))
            .child(
                img(self.icons.topbar_share.render(cx))
                    .w(px(15.01))
                    .h(px(16.0)),
            )
            .child("Share")
            .child(
                img(self.icons.topbar_share_chevron.render(cx))
                    .w(px(7.41))
                    .h(px(12.0)),
            )
            .into_any_element()
    }

    pub(crate) fn render_top_bar_favorite_button(&self, cx: &mut App) -> Div {
        let icon = if self.view.favorited {
            &self.icons.topbar_favorite_active
        } else {
            &self.icons.topbar_favorite_inactive
        };

        div()
            .h(px(28.0))
            .w(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .when(self.view.favorited, |this| {
                this.bg(rgba(self.theme.favorite_active_bg))
            })
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, _| TopBarAction::ToggleFavorite),
            )
            .child(img(icon.render(cx)).size(px(20.0)))
    }

    pub(crate) fn render_top_bar_copy_link_button(&self, cx: &mut App) -> AnyElement {
        let icon = self.icons.topbar_copy_link.render(cx);
        div()
            .id("notion-topbar-copy-link")
            .h(px(28.0))
            .w(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .when_some(self.view.board_url.clone(), |this, board_url| {
                this.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    move |_: &MouseDownEvent, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(board_url.clone()));
                    },
                )
            })
            .child(img(icon).size(px(20.0)))
            .into_any_element()
    }

    pub(crate) fn render_top_bar_icon_button(
        &self,
        icon: &IconAsset,
        cx: &mut App,
        accent_fill: bool,
        dialog: Option<ToolbarDialogKind>,
    ) -> Div {
        div()
            .h(px(28.0))
            .w(px(28.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .when(dialog.is_some(), |this| this.cursor_pointer())
            .when_some(dialog, |this, dialog| {
                this.on_mouse_down(
                    MouseButton::Left,
                    self.actions
                        .listener(move |_: &MouseDownEvent, _, _| TopBarAction::OpenDialog(dialog)),
                )
            })
            .child(img(icon.render(cx)).size(if accent_fill { px(20.0) } else { px(22.0) }))
    }
}
