use gpui::{
    div, px, uniform_list, AnyElement, App, FontWeight, InteractiveElement, IntoElement,
    ListSizingBehavior, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};
use gpui_components::backdrop::dismissible_backdrop_with_handler;

use super::rows::ShareRowsView;
use super::{
    ShareDialogView, ShareEvent, SHARE_DIALOG_HEADER_HEIGHT, SHARE_DIALOG_HEIGHT,
    SHARE_DIALOG_WIDTH,
};
use crate::ui::surface::NotionShareDialogState;
use crate::ui::{alpha, rgb, rgba, FluentBuilder};

impl ShareDialogView<'_> {
    pub(super) fn render(&self, cx: &mut App) -> AnyElement {
        let dialog = self.dialog;
        let panel = self
            .render_notion_share_dialog_panel(dialog, cx)
            .when_some(dialog.role_picker.as_ref(), |panel, picker| {
                panel.child(self.render_notion_share_role_picker(picker, cx))
            });
        div()
            .id("notion-share-dialog-overlay")
            .absolute()
            .inset_0()
            .child(dismissible_backdrop_with_handler(
                div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
                self.actions.listener(|_, _, _| ShareEvent::Dismiss),
            ))
            .child(panel)
            .into_any_element()
    }

    fn render_notion_share_dialog_panel(
        &self,
        dialog: &NotionShareDialogState,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        let panel = self.notion_share_dialog_panel_shell(dialog, cx);
        if dialog.loaded {
            panel
                .when(dialog.user_picker_open, |panel| {
                    panel.child(self.render_notion_share_member_search(cx))
                })
                .child(self.render_notion_share_permission_list(dialog, cx))
        } else {
            panel.child(self.render_notion_share_skeleton())
        }
    }

    fn notion_share_dialog_panel_shell(
        &self,
        dialog: &NotionShareDialogState,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id("notion-share-dialog")
            .role(Role::Dialog)
            .aria_label("Share")
            .absolute()
            .top(px(40.0))
            .right(px(self.right_inset))
            .w(px(SHARE_DIALOG_WIDTH))
            .h(px(SHARE_DIALOG_HEIGHT))
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(self.theme.surface_border))
            .bg(rgb(self.theme.elevated_surface_bg))
            .occlude()
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .flex()
            .flex_col()
            .child(self.render_notion_share_dialog_header(dialog, cx))
    }

    fn render_notion_share_permission_list(
        &self,
        dialog: &NotionShareDialogState,
        _cx: &mut App,
    ) -> AnyElement {
        let rows = dialog.rows.clone();
        let row_count = rows.len();
        let renderer = ShareRowsView::new(dialog, self.theme, self.actions.clone());
        let list = uniform_list(
            "notion-share-permission-rows",
            row_count,
            move |range, _window, cx| {
                range
                    .filter_map(|index| {
                        rows.get(index)
                            .map(|row| renderer.render_notion_share_dialog_row(row, cx))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .track_scroll(&dialog.scroll_handle)
        .w_full()
        .h_full();
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .overflow_hidden()
            .px(px(6.0))
            .pb(px(6.0))
            .child(list)
            .into_any_element()
    }

    fn render_notion_share_dialog_header(
        &self,
        dialog: &NotionShareDialogState,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .h(px(SHARE_DIALOG_HEADER_HEIGHT))
            .flex_none()
            .px(px(14.0))
            .flex()
            .items_center()
            .justify_between()
            .child(self.notion_share_header_title(dialog))
            .child(self.notion_share_close_button(cx))
            .into_any_element()
    }

    fn notion_share_header_title(&self, dialog: &NotionShareDialogState) -> AnyElement {
        div()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_primary))
                    .child("Share"),
            )
            .when(dialog.loaded, |header| {
                header.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(self.theme.text_muted))
                        .child(dialog.effective_role_label.clone()),
                )
            })
            .into_any_element()
    }

    fn notion_share_close_button(&self, _cx: &mut App) -> AnyElement {
        div()
            .id("notion-share-dialog-close")
            .role(Role::Button)
            .aria_label("Close sharing")
            .size(px(26.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    ShareEvent::Dismiss
                }),
            )
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(17.0))
            .text_color(rgb(self.theme.text_secondary))
            .child("×")
            .into_any_element()
    }

    fn render_notion_share_skeleton(&self) -> AnyElement {
        div()
            .flex_grow(1.0)
            .px(px(14.0))
            .pt(px(6.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children((0..5).map(|index| {
                div()
                    .h(px(36.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .size(px(28.0))
                            .rounded_full()
                            .bg(alpha(self.theme.text_primary, 0.08)),
                    )
                    .child(
                        div()
                            .h(px(10.0))
                            .w(px(if index % 2 == 0 { 180.0 } else { 140.0 }))
                            .rounded(px(4.0))
                            .bg(alpha(self.theme.text_primary, 0.08)),
                    )
            }))
            .into_any_element()
    }
}
