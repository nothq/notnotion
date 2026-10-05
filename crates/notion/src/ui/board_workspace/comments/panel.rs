use gpui::{
    div, list, px, AnyElement, Context, FontWeight, InteractiveElement, IntoElement,
    ListSizingBehavior, MouseButton, MouseDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled,
};

use crate::model::NotionDiscussionId;
use crate::ui::surface::{
    NotionCommentRow, NotionCommentSegmentRow, NotionCommentsListRow, NotionCommentsPanelState,
};
use crate::ui::{alpha, rgb, rgba, FluentBuilder, SurfaceState, Theme};

const COMMENTS_PANEL_WIDTH: f32 = 420.0;
const COMMENTS_PANEL_TOP: f32 = 42.0;

impl NotionCommentsPanelState {
    pub(super) fn render_notion_comments_panel(
        &self,
        theme: Theme,
        right: f32,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .id("notion-comments-panel")
            .role(Role::Dialog)
            .aria_label("Comments")
            .absolute()
            .top(px(COMMENTS_PANEL_TOP))
            .right(px(right))
            .bottom(px(8.0))
            .w(px(COMMENTS_PANEL_WIDTH))
            .overflow_hidden()
            .rounded(px(10.0))
            .border_1()
            .border_color(rgba(theme.surface_border))
            .bg(rgb(theme.elevated_surface_bg))
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| cx.stop_propagation()),
            )
            .flex()
            .flex_col()
            .child(self.render_notion_comments_header(theme, cx))
            .child(self.render_notion_comments_list(theme, cx))
            .child(self.render_notion_comment_composer(theme, cx))
            .into_any_element()
    }

    fn render_notion_comments_header(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .h(px(54.0))
            .flex_none()
            .px(px(14.0))
            .border_b_1()
            .border_color(rgba(theme.surface_border))
            .flex()
            .items_center()
            .justify_between()
            .child(self.render_notion_comments_header_title(theme))
            .child(self.render_notion_comments_close(theme, cx))
            .into_any_element()
    }

    fn render_notion_comments_header_title(&self, theme: Theme) -> AnyElement {
        let target_label = if self.target_id == self.page_id {
            "Page discussion"
        } else {
            "Block discussion"
        };
        div()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.text_primary))
                    .child("Comments"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(theme.text_muted))
                    .child(target_label),
            )
            .into_any_element()
    }

    fn render_notion_comments_close(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .id("notion-comments-close")
            .role(Role::Button)
            .aria_label("Close comments")
            .size(px(26.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(theme.text_primary, 0.08)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.comments.dismiss_notion_comments_panel(cx);
                }),
            )
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(17.0))
            .text_color(rgb(theme.text_secondary))
            .child("×")
            .into_any_element()
    }

    fn render_notion_comments_list(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        if self.rows.is_empty() {
            return self.render_empty_notion_comments(theme);
        }
        let rows = self.rows.clone();
        let mutation_in_flight = self.mutation_in_flight;
        let surface = cx.entity();
        list(self.thread_list_state.clone(), move |index, _window, cx| {
            let row = rows
                .get(index)
                .cloned()
                .expect("comments ListState count must match shaped rows");
            surface.update(cx, move |_surface, cx| {
                NotionCommentsPanelState::render_notion_comments_list_row(
                    theme,
                    mutation_in_flight,
                    row,
                    cx,
                )
            })
        })
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .flex_grow(1.0)
        .min_h(px(0.0))
        .px(px(12.0))
        .into_any_element()
    }

    fn render_empty_notion_comments(&self, theme: Theme) -> AnyElement {
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .text_color(rgb(theme.text_muted))
            .child("No comments yet")
            .into_any_element()
    }

    fn render_notion_comments_list_row(
        theme: Theme,
        mutation_in_flight: bool,
        row: NotionCommentsListRow,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        match row {
            NotionCommentsListRow::ThreadHeader {
                stable_id,
                read_only_reason,
            } => Self::render_notion_comment_thread_header(theme, stable_id, read_only_reason),
            NotionCommentsListRow::Comment { comment } => {
                Self::render_notion_comment_row(theme, comment)
            }
            NotionCommentsListRow::Reply {
                stable_id,
                discussion_id,
                enabled,
            } => Self::render_notion_comment_reply(
                theme,
                stable_id,
                discussion_id,
                enabled && !mutation_in_flight,
                cx,
            ),
        }
    }

    fn render_notion_comment_thread_header(
        theme: Theme,
        stable_id: gpui::SharedString,
        read_only_reason: Option<gpui::SharedString>,
    ) -> AnyElement {
        div()
            .id(stable_id)
            .pt(px(14.0))
            .pb(px(5.0))
            .border_b_1()
            .border_color(rgba(theme.surface_border))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(theme.text_secondary))
                    .child("Thread"),
            )
            .when_some(read_only_reason, |row, reason| {
                row.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(theme.text_muted))
                        .child(reason),
                )
            })
            .into_any_element()
    }

    fn render_notion_comment_reply(
        theme: Theme,
        stable_id: gpui::SharedString,
        discussion_id: NotionDiscussionId,
        enabled: bool,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let discussion = discussion_id.clone();
        div()
            .id(stable_id)
            .pb(px(8.0))
            .flex()
            .when(enabled, |row| {
                row.child(
                    div()
                        .id(format!("notion-comment-reply-{}", discussion.as_str()))
                        .role(Role::Button)
                        .aria_label("Reply to discussion")
                        .px(px(6.0))
                        .py(px(3.0))
                        .rounded(px(4.0))
                        .cursor_pointer()
                        .hover(|style| style.bg(alpha(theme.text_primary, 0.06)))
                        .text_size(px(11.0))
                        .text_color(rgb(theme.text_secondary))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.comments
                                    .begin_notion_comment_reply(discussion.clone(), cx);
                            }),
                        )
                        .child("Reply"),
                )
            })
            .into_any_element()
    }

    fn render_notion_comment_row(theme: Theme, comment: NotionCommentRow) -> AnyElement {
        div()
            .id(comment.stable_id.clone())
            .py(px(9.0))
            .flex()
            .items_start()
            .gap(px(9.0))
            .child(Self::render_notion_comment_avatar(
                comment.author_initial.clone(),
            ))
            .child(Self::render_notion_comment_content(theme, comment))
            .into_any_element()
    }

    fn render_notion_comment_avatar(initial: gpui::SharedString) -> AnyElement {
        div()
            .size(px(26.0))
            .flex_none()
            .rounded_full()
            .bg(rgb(0x2383e2))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0xffffff))
            .child(initial)
            .into_any_element()
    }

    fn render_notion_comment_content(theme: Theme, comment: NotionCommentRow) -> AnyElement {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(3.0))
            .child(Self::render_notion_comment_byline(theme, &comment))
            .child(Self::render_notion_comment_segments(theme, &comment))
            .into_any_element()
    }

    fn render_notion_comment_byline(theme: Theme, comment: &NotionCommentRow) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.text_primary))
                    .child(comment.author.clone()),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(theme.text_muted))
                    .child(comment.timestamp.clone()),
            )
            .into_any_element()
    }

    fn render_notion_comment_segments(theme: Theme, comment: &NotionCommentRow) -> AnyElement {
        div()
            .w_full()
            .flex()
            .flex_wrap()
            .text_size(px(13.0))
            .line_height(px(19.0))
            .children(
                comment
                    .segments
                    .iter()
                    .cloned()
                    .map(|segment| Self::render_notion_comment_segment(theme, segment)),
            )
            .into_any_element()
    }

    fn render_notion_comment_segment(theme: Theme, segment: NotionCommentSegmentRow) -> AnyElement {
        match segment {
            NotionCommentSegmentRow::Plain(text) => div()
                .text_color(rgb(theme.text_primary))
                .child(text)
                .into_any_element(),
            NotionCommentSegmentRow::Mention(text) => div()
                .px(px(2.0))
                .rounded(px(3.0))
                .bg(alpha(0x2383e2, 0.12))
                .text_color(rgb(0x2383e2))
                .child(text)
                .into_any_element(),
            NotionCommentSegmentRow::Unsupported(text) => div()
                .px(px(3.0))
                .rounded(px(3.0))
                .bg(alpha(theme.text_primary, 0.06))
                .text_color(rgb(theme.text_muted))
                .child(text)
                .into_any_element(),
        }
    }
}
