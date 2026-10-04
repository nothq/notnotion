use gpui::{
    div, px, uniform_list, AnyElement, Context, FontWeight, InteractiveElement, IntoElement,
    ListSizingBehavior, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};

use super::COMMENTS_COMPOSER_HEIGHT;
use crate::ui::surface::{NotionCommentMentionRow, NotionCommentsPanelState};
use crate::ui::{alpha, rgb, rgba, FluentBuilder, SurfaceState, Theme};

const COMMENTS_MENTION_ROW_HEIGHT: f32 = 38.0;

impl NotionCommentsPanelState {
    pub(super) fn render_notion_comment_at_button(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .id("notion-comment-at")
            .size(px(28.0))
            .rounded(px(5.0))
            .when(!self.mutation_in_flight, |button| {
                button
                    .role(Role::Button)
                    .aria_label("Mention a workspace user")
                    .cursor_pointer()
                    .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            if let Some(panel) = this.comments.panel.as_mut() {
                                let separator = if !panel.plain_tail.is_empty()
                                    && !panel.plain_tail.ends_with(char::is_whitespace)
                                {
                                    " "
                                } else {
                                    ""
                                };
                                let tail = panel.plain_tail.clone();
                                panel.set_plain_tail(format!("{tail}{separator}@"));
                                panel.focus_requested.set(true);
                                cx.notify();
                            }
                        }),
                    )
            })
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(15.0))
            .text_color(rgb(theme.text_secondary))
            .child("@")
            .into_any_element()
    }

    pub(super) fn render_notion_comment_mention_picker(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let rows = self.mention_rows.clone();
        let row_count = rows.len();
        let surface = cx.entity();
        let list = uniform_list(
            "notion-comment-mention-rows",
            row_count,
            move |range, _window, cx| {
                surface.update(cx, |_, cx| {
                    range
                        .filter_map(|index| {
                            rows.get(index).cloned().map(|row| {
                                NotionCommentsPanelState::render_notion_comment_mention_row(
                                    theme, row, cx,
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                })
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .track_scroll(&self.mention_scroll_handle)
        .w_full()
        .h_full();
        div()
            .id("notion-comment-mention-picker")
            .role(Role::ListBox)
            .aria_label("Mention workspace user")
            .absolute()
            .left(px(12.0))
            .right(px(12.0))
            .bottom(px(COMMENTS_COMPOSER_HEIGHT - 3.0))
            .h(px(mention_picker_height(row_count)))
            .overflow_hidden()
            .rounded(px(8.0))
            .border_1()
            .border_color(rgba(theme.surface_border))
            .bg(rgb(theme.elevated_surface_bg))
            .p(px(4.0))
            .occlude()
            .child(list)
            .into_any_element()
    }

    fn render_notion_comment_mention_row(
        theme: Theme,
        row: NotionCommentMentionRow,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let user_id = row.user_id.clone();
        div()
            .id(format!("notion-comment-mention-{}", row.user_id.as_str()))
            .role(Role::ListBoxOption)
            .aria_label(row.label.clone())
            .h(px(COMMENTS_MENTION_ROW_HEIGHT))
            .px(px(8.0))
            .rounded(px(5.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.07)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    let result = this
                        .comments
                        .panel
                        .as_mut()
                        .ok_or_else(|| "Notion comments panel closed".to_string())
                        .and_then(|panel| panel.select_mention(&user_id));
                    if let Err(error) = result {
                        this.print_notion_error(error);
                    }
                    cx.notify();
                }),
            )
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(Self::render_notion_comment_mention_avatar(&row))
            .child(Self::render_notion_comment_mention_label(theme, row))
            .into_any_element()
    }

    fn render_notion_comment_mention_avatar(row: &NotionCommentMentionRow) -> AnyElement {
        div()
            .size(px(24.0))
            .rounded_full()
            .bg(rgb(0x2383e2))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0xffffff))
            .child(row.initial.clone())
            .into_any_element()
    }

    fn render_notion_comment_mention_label(
        theme: Theme,
        row: NotionCommentMentionRow,
    ) -> AnyElement {
        div()
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .text_color(rgb(theme.text_primary))
            .child(row.label)
            .into_any_element()
    }
}

fn mention_picker_height(row_count: usize) -> f32 {
    (row_count.min(5) as f32 * COMMENTS_MENTION_ROW_HEIGHT + 8.0).min(198.0)
}
