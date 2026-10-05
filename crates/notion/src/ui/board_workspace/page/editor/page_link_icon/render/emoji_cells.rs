use gpui::App;
use std::sync::Arc;

use gpui::{
    AnyElement, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, RenderImage, Role, Stateful, StatefulInteractiveElement, Styled, StyledImage,
};

use super::super::{PageLinkIconAction, PageLinkIconSelectionAction, PageLinkIconTarget};
use crate::{
    model::{NotionCustomEmoji, PageShellIcon},
    ui::board_workspace::{alpha, div, img, page::editor::PageLinkIconView, px, rgb},
};

use super::{
    catalog::PageLinkPickerEmojiSearchMatch, emoji_grid::PageLinkEmojiListRender,
    PAGE_LINK_EMOJI_COLUMN_COUNT, PAGE_LINK_EMOJI_ROW_HEIGHT,
};

impl PageLinkIconView {
    pub(super) fn render_page_link_custom_emoji_row(
        &self,
        row_index: usize,
        context: &PageLinkEmojiListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let PageLinkEmojiListRender {
            target,
            custom_emojis,
            custom_emojis_pending: pending,
            ..
        } = *context;
        let start = row_index * PAGE_LINK_EMOJI_COLUMN_COUNT;
        let end = (start + PAGE_LINK_EMOJI_COLUMN_COUNT).min(custom_emojis.len());
        let row = custom_emojis[start..end]
            .iter()
            .fold(page_link_custom_emoji_row(row_index), |row, emoji| {
                row.child(self.render_page_link_custom_emoji_cell(target, emoji, cx))
            });
        self.render_empty_page_link_custom_emoji_row(
            row,
            custom_emojis.is_empty() && row_index == 0,
            pending,
        )
        .into_any_element()
    }

    fn render_empty_page_link_custom_emoji_row(
        &self,
        row: Stateful<Div>,
        empty: bool,
        pending: bool,
    ) -> Stateful<Div> {
        if !empty {
            return row;
        }
        if pending {
            return row.child(
                div()
                    .w(px(112.0))
                    .h(px(12.0))
                    .rounded(px(4.0))
                    .bg(alpha(self.theme.text_primary, 0.055)),
            );
        }
        row.text_size(px(12.0))
            .text_color(rgb(self.theme.text_muted))
            .child("No custom emoji yet")
    }

    fn custom_emoji_key_down(
        &self,
        key_target: PageLinkIconTarget,
        key_pointer: String,
        key_rendered: Option<Arc<RenderImage>>,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Selection(PageLinkIconSelectionAction::SelectCustomEmoji {
                    target: key_target.clone(),
                    pointer: key_pointer.clone(),
                    rendered: key_rendered.clone(),
                }),
                window,
                cx,
            );
        })
    }

    pub(super) fn render_page_link_custom_emoji_cell(
        &self,
        target: &PageLinkIconTarget,
        emoji: &NotionCustomEmoji,
        cx: &mut App,
    ) -> AnyElement {
        let rendered = self
            .resources
            .external_icon_image(&emoji.render_url, &self.notifier, cx);
        let target = target.clone();
        let key_target = target.clone();
        let pointer = emoji.pointer.clone();
        let key_pointer = pointer.clone();
        let click_rendered = rendered.clone();
        let key_rendered = rendered.clone();
        div()
            .id(ElementId::Name(
                format!("notion-page-link-custom-emoji-{}", emoji.id).into(),
            ))
            .role(Role::GridCell)
            .aria_label(emoji.name.clone())
            .focusable()
            .tab_stop(true)
            .size(px(32.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .on_click(self.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Selection(PageLinkIconSelectionAction::SelectCustomEmoji {
                        target: target.clone(),
                        pointer: pointer.clone(),
                        rendered: click_rendered.clone(),
                    }),
                    window,
                    cx,
                );
            }))
            .on_key_down(self.custom_emoji_key_down(key_target, key_pointer, key_rendered))
            .child(page_link_custom_emoji_image(
                rendered,
                self.theme.text_primary,
            ))
            .into_any_element()
    }

    pub(super) fn render_page_link_emoji_search_row(
        &self,
        row_index: usize,
        emojis: &[PageLinkPickerEmojiSearchMatch],
        context: &PageLinkEmojiListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let PageLinkEmojiListRender {
            target, skin_tone, ..
        } = *context;
        let start = row_index * PAGE_LINK_EMOJI_COLUMN_COUNT;
        let end = (start + PAGE_LINK_EMOJI_COLUMN_COUNT).min(emojis.len());
        emojis[start..end]
            .iter()
            .enumerate()
            .fold(
                page_link_emoji_search_row(row_index),
                |row, (column, emoji)| {
                    row.child(match emoji {
                        PageLinkPickerEmojiSearchMatch::BuiltIn(emoji) => self
                            .render_page_link_emoji_cell(target, start + column, emoji, skin_tone),
                        PageLinkPickerEmojiSearchMatch::Custom(emoji) => {
                            self.render_page_link_custom_emoji_cell(target, emoji, cx)
                        }
                    })
                },
            )
            .into_any_element()
    }

    pub(super) fn render_page_link_emoji_cell(
        &self,
        target: &PageLinkIconTarget,
        index: usize,
        emoji: &'static emojis::Emoji,
        skin_tone: emojis::SkinTone,
    ) -> AnyElement {
        let PageLinkIconTarget { page_id, block_id } = target;
        let glyph = emoji
            .with_skin_tone(skin_tone)
            .unwrap_or(emoji)
            .as_str()
            .to_string();
        let click_glyph = glyph.clone();
        let key_glyph = glyph.clone();
        div()
            .id(ElementId::Name(
                format!("notion-page-link-emoji-{block_id}-{index}").into(),
            ))
            .role(Role::GridCell)
            .aria_label(emoji.name().to_string())
            .focusable()
            .tab_stop(true)
            .size(px(32.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .text_size(px(24.0))
            .line_height(px(36.0))
            .on_click(self.page_link_emoji_click(
                page_id.to_string(),
                block_id.to_string(),
                click_glyph,
            ))
            .on_key_down(self.page_link_emoji_key_down(
                page_id.to_string(),
                block_id.to_string(),
                key_glyph,
            ))
            .child(glyph)
            .into_any_element()
    }

    fn page_link_emoji_click(
        &self,
        page_id: String,
        block_id: String,
        glyph: String,
    ) -> impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Selection(PageLinkIconSelectionAction::SetIcon {
                    target: PageLinkIconTarget {
                        page_id: page_id.clone(),
                        block_id: block_id.clone(),
                    },
                    icon: Some(PageShellIcon::emoji(glyph.clone())),
                }),
                window,
                cx,
            );
        })
    }

    fn page_link_emoji_key_down(
        &self,
        page_id: String,
        block_id: String,
        glyph: String,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            this.emit(
                PageLinkIconAction::Selection(PageLinkIconSelectionAction::SetIcon {
                    target: PageLinkIconTarget {
                        page_id: page_id.clone(),
                        block_id: block_id.clone(),
                    },
                    icon: Some(PageShellIcon::emoji(glyph.clone())),
                }),
                window,
                cx,
            );
        })
    }
}

fn page_link_custom_emoji_row(row_index: usize) -> Stateful<Div> {
    div()
        .id(ElementId::Name(
            format!("notion-page-link-custom-emoji-row-{row_index}").into(),
        ))
        .role(Role::Row)
        .w_full()
        .h(px(PAGE_LINK_EMOJI_ROW_HEIGHT))
        .px(px(12.0))
        .flex()
        .items_center()
}

fn page_link_emoji_search_row(row_index: usize) -> Stateful<Div> {
    div()
        .id(ElementId::Name(
            format!("notion-page-link-emoji-search-row-{row_index}").into(),
        ))
        .role(Role::Row)
        .w_full()
        .h(px(PAGE_LINK_EMOJI_ROW_HEIGHT))
        .px(px(12.0))
        .flex()
        .items_start()
}

fn page_link_custom_emoji_image(
    rendered: Option<Arc<RenderImage>>,
    text_primary: u32,
) -> AnyElement {
    match rendered {
        Some(rendered) => img(rendered)
            .size(px(24.0))
            .object_fit(gpui::ObjectFit::Contain)
            .into_any_element(),
        None => div()
            .size(px(24.0))
            .rounded(px(4.0))
            .bg(alpha(text_primary, 0.055))
            .into_any_element(),
    }
}
