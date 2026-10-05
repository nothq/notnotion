use std::rc::Rc;

use gpui::{
    div, px, AnyElement, AppContext, Context, FontWeight, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Role, StatefulInteractiveElement, Styled,
};
use gpui_components::text_input::{
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

use super::COMMENTS_COMPOSER_HEIGHT;
use crate::model::{NotionCommentDraftSegment, NotionDiscussionId};
use crate::ui::surface::NotionCommentsPanelState;
use crate::ui::{alpha, rgb, rgba, FluentBuilder, SurfaceState, Theme};

impl NotionCommentsPanelState {
    pub(super) fn render_notion_comment_composer(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        if !self.can_comment {
            return self.render_notion_comment_read_only(theme);
        }
        self.render_notion_comment_editor(theme, cx)
    }

    fn render_notion_comment_read_only(&self, theme: Theme) -> AnyElement {
        div()
            .h(px(48.0))
            .flex_none()
            .px(px(14.0))
            .border_t_1()
            .border_color(rgba(theme.surface_border))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .text_color(rgb(theme.text_muted))
            .child("Comments are read-only")
            .into_any_element()
    }

    fn render_notion_comment_editor(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let mention_picker = (!self.mutation_in_flight && !self.mention_rows.is_empty())
            .then(|| self.render_notion_comment_mention_picker(theme, cx));
        div()
            .h(px(COMMENTS_COMPOSER_HEIGHT))
            .flex_none()
            .relative()
            .px(px(12.0))
            .py(px(9.0))
            .border_t_1()
            .border_color(rgba(theme.surface_border))
            .flex()
            .flex_col()
            .gap(px(5.0))
            .when_some(self.reply_to.as_ref(), |composer, discussion_id| {
                composer.child(self.render_notion_comment_reply_banner(
                    discussion_id.clone(),
                    self.mutation_in_flight,
                    theme,
                    cx,
                ))
            })
            .child(self.render_notion_comment_committed_segments(theme, cx))
            .child(self.render_notion_comment_controls(theme, cx))
            .when_some(mention_picker, |composer, picker| composer.child(picker))
            .into_any_element()
    }

    fn render_notion_comment_reply_banner(
        &self,
        discussion_id: NotionDiscussionId,
        mutation_in_flight: bool,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .h(px(18.0))
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(10.0))
            .text_color(rgb(theme.text_muted))
            .child("Replying to thread")
            .child(
                div()
                    .id(format!(
                        "notion-comment-cancel-reply-{}",
                        discussion_id.as_str()
                    ))
                    .when(!mutation_in_flight, |button| {
                        button
                            .role(Role::Button)
                            .aria_label("Cancel reply")
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    if let Some(panel) = this.comments.panel.as_mut() {
                                        panel.reply_to = None;
                                        panel.focus_requested.set(true);
                                    }
                                    cx.notify();
                                }),
                            )
                    })
                    .child("Cancel"),
            )
            .into_any_element()
    }

    fn render_notion_comment_committed_segments(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let segments = self
            .committed_segments
            .iter()
            .map(|segment| self.render_notion_comment_draft_segment(segment, theme))
            .collect::<Vec<_>>();
        div()
            .h(px(18.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .children(segments)
            .when(panel_can_remove_mention(self), |row| {
                row.child(self.render_notion_remove_mention(theme, cx))
            })
            .into_any_element()
    }

    fn render_notion_comment_draft_segment(
        &self,
        segment: &NotionCommentDraftSegment,
        theme: Theme,
    ) -> AnyElement {
        match segment {
            NotionCommentDraftSegment::Plain(text) => div()
                .text_size(px(10.0))
                .text_color(rgb(theme.text_secondary))
                .child(text.clone())
                .into_any_element(),
            NotionCommentDraftSegment::Mention(user_id) => div()
                .px(px(4.0))
                .rounded(px(3.0))
                .bg(alpha(0x2383e2, 0.12))
                .text_size(px(10.0))
                .text_color(rgb(0x2383e2))
                .child(self.mention_label(user_id))
                .into_any_element(),
        }
    }

    fn render_notion_remove_mention(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .id("notion-comment-remove-last-mention")
            .role(Role::Button)
            .aria_label("Remove most recent mention")
            .cursor_pointer()
            .text_size(px(10.0))
            .text_color(rgb(theme.text_muted))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if this
                        .comments
                        .panel
                        .as_mut()
                        .is_some_and(|panel| panel.remove_most_recent_mention())
                    {
                        cx.notify();
                    }
                }),
            )
            .child("Remove @")
            .into_any_element()
    }

    fn render_notion_comment_controls(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        div()
            .h(px(34.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .h_full()
                    .child(self.notion_comment_input_entity(theme, cx)),
            )
            .child(self.render_notion_comment_at_button(theme, cx))
            .child(self.render_notion_comment_submit_button(theme, cx))
            .into_any_element()
    }

    fn notion_comment_input_entity(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> gpui::Entity<TextInput> {
        let props = TextInputProps::single_line(self.plain_tail.clone())
            .placeholder("Write a comment…")
            .disabled(self.mutation_in_flight)
            .request_focus(self.focus_requested.replace(false))
            .style(self.notion_comment_input_style(theme))
            .accessibility("notion-comment-input", "Write a comment")
            .on_change(self.notion_comment_on_change(cx))
            .on_submit(self.notion_comment_on_submit(cx))
            .on_escape(self.notion_comment_on_escape(cx));
        if let Some(input) = self.input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.input.borrow_mut().replace(input.clone());
        input
    }

    fn notion_comment_input_style(&self, theme: Theme) -> TextInputStyle {
        TextInputStyle {
            height: px(34.0),
            min_height: px(34.0),
            padding_x: px(8.0),
            padding_y: px(5.0),
            radius: px(6.0),
            background: alpha(theme.text_primary, 0.04),
            border: alpha(theme.text_primary, 0.10),
            focused_border: rgb(0x2383e2).into(),
            text: rgb(theme.text_primary).into(),
            placeholder: rgb(theme.text_muted).into(),
            selection: alpha(0x2383e2, 0.28),
            caret: rgb(theme.text_primary).into(),
            font_size: px(13.0),
            line_height: px(18.0),
            font_family: None,
        }
    }

    fn notion_comment_on_change(&self, cx: &mut Context<SurfaceState>) -> TextInputChange {
        let surface = cx.entity();
        Rc::new(move |value, _window, cx| {
            surface.update(cx, |surface, cx| {
                if let Some(panel) = surface.comments.panel.as_mut() {
                    panel.set_plain_tail(value);
                    cx.notify();
                }
            });
        })
    }

    fn notion_comment_on_submit(&self, cx: &mut Context<SurfaceState>) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.submit_notion_comment(cx);
            });
        })
    }

    fn notion_comment_on_escape(&self, cx: &mut Context<SurfaceState>) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.comments.dismiss_notion_comments_panel(cx);
            });
        })
    }

    fn render_notion_comment_submit_button(
        &self,
        theme: Theme,
        cx: &mut Context<SurfaceState>,
    ) -> AnyElement {
        let enabled = !self.mutation_in_flight && self.draft().is_ok();
        div()
            .id("notion-comment-submit")
            .h(px(28.0))
            .px(px(9.0))
            .rounded(px(5.0))
            .bg(if enabled {
                alpha(0x2383e2, 1.0)
            } else {
                alpha(theme.text_primary, 0.08)
            })
            .text_size(px(11.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(if enabled {
                rgb(0xffffff)
            } else {
                rgb(theme.text_muted)
            })
            .when(enabled, |button| {
                button
                    .role(Role::Button)
                    .aria_label("Submit comment")
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.submit_notion_comment(cx);
                        }),
                    )
            })
            .flex()
            .items_center()
            .justify_center()
            .child("Send")
            .into_any_element()
    }
}

fn panel_can_remove_mention(panel: &NotionCommentsPanelState) -> bool {
    !panel.mutation_in_flight
        && panel
            .committed_segments
            .iter()
            .any(|segment| matches!(segment, NotionCommentDraftSegment::Mention(_)))
}
