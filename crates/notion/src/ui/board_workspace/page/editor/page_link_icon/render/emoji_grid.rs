use gpui::App;
mod categories;

use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    list, AnyElement, Div, ElementId, InteractiveElement, IntoElement, ListSizingBehavior,
    ParentElement, Role, Stateful, StatefulInteractiveElement, Styled,
};

use crate::{
    model::NotionCustomEmoji,
    ui::board_workspace::{
        div,
        page::editor::{PageLinkIconPickerState, PageLinkIconTarget, PageLinkIconView},
        px, rgb,
    },
};

use super::{
    catalog::{
        emoji_list_item, emoji_search_matches, emoji_sections, PageLinkPickerEmojiListItem,
        PageLinkPickerEmojiSearchMatch,
    },
    PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX, PAGE_LINK_EMOJI_COLUMN_COUNT, PAGE_LINK_EMOJI_ROW_HEIGHT,
    PAGE_LINK_EMOJI_SECTION_HEADER_HEIGHT,
};

pub(super) struct PageLinkEmojiListRender<'a> {
    pub(super) target: &'a PageLinkIconTarget,
    pub(super) skin_tone: emojis::SkinTone,
    pub(super) custom_emojis: &'a [NotionCustomEmoji],
    pub(super) custom_emojis_pending: bool,
}

#[derive(Clone, Copy)]
struct PageLinkEmojiCatalogRow {
    section_index: usize,
    row_index: usize,
    emoji_start_index: usize,
}

impl PageLinkIconView {
    pub(super) fn render_page_link_emoji_grid(
        &self,
        state: &PageLinkIconPickerState,
        page_id: &str,
        block_id: &str,
        cx: &mut App,
    ) -> AnyElement {
        let query_is_empty = state.query.trim().is_empty();
        let target = PageLinkIconTarget {
            page_id: page_id.to_string(),
            block_id: block_id.to_string(),
        };
        let viewport = div()
            .id("notion-page-link-emoji-grid")
            .role(Role::Grid)
            .aria_label("Emoji")
            .size_full()
            .child(self.render_page_link_emoji_rows(state, target, query_is_empty));
        div()
            .relative()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .child(viewport)
            .when(query_is_empty, |picker| {
                picker.child(self.render_page_link_emoji_category_nav(state, cx))
            })
            .into_any_element()
    }

    fn render_page_link_emoji_rows(
        &self,
        state: &PageLinkIconPickerState,
        target: PageLinkIconTarget,
        query_is_empty: bool,
    ) -> AnyElement {
        let skin_tone = state.skin_tone;
        let search_emojis = (!query_is_empty).then(|| {
            Arc::<[PageLinkPickerEmojiSearchMatch]>::from(emoji_search_matches(
                state.query.trim(),
                &state.custom_emojis,
            ))
        });
        let custom_emojis = Arc::clone(&state.custom_emojis);
        let custom_emojis_pending = state.custom_emoji_library_pending;
        let view = self.clone();
        list(
            state.emoji_list_state.clone(),
            move |item_index, _window, cx| {
                let target = target.clone();
                let search_emojis = search_emojis.clone();
                let custom_emojis = Arc::clone(&custom_emojis);
                view.render_page_link_emoji_row(
                    item_index,
                    search_emojis.as_deref(),
                    PageLinkEmojiListRender {
                        target: &target,
                        skin_tone,
                        custom_emojis: &custom_emojis,
                        custom_emojis_pending,
                    },
                    cx,
                )
            },
        )
        .with_sizing_behavior(ListSizingBehavior::Auto)
        .size_full()
        // Reserve the fixed category rail so the final virtual row can scroll
        // fully above it.
        .when(query_is_empty, |list| list.pb(px(51.0)))
        .into_any_element()
    }

    fn render_page_link_emoji_row(
        &self,
        item_index: usize,
        search_emojis: Option<&[PageLinkPickerEmojiSearchMatch]>,
        context: PageLinkEmojiListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        if let Some(emojis) = search_emojis {
            return self.render_page_link_emoji_search_row(item_index, emojis, &context, cx);
        }
        self.render_page_link_emoji_list_item(item_index, context, cx)
    }

    fn render_page_link_emoji_list_item(
        &self,
        item_index: usize,
        context: PageLinkEmojiListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let item = emoji_list_item(item_index, context.custom_emojis.len())
            .expect("emoji picker list index must exist");
        match item {
            PageLinkPickerEmojiListItem::SectionHeader { section_index } => {
                self.render_page_link_emoji_section_header(section_index)
            }
            PageLinkPickerEmojiListItem::EmojiRow {
                section_index,
                row_index,
                emoji_start_index,
            } => self.render_page_link_emoji_catalog_row(
                PageLinkEmojiCatalogRow {
                    section_index,
                    row_index,
                    emoji_start_index,
                },
                context,
                cx,
            ),
        }
    }

    fn render_page_link_emoji_section_header(&self, section_index: usize) -> AnyElement {
        let section = &emoji_sections()[section_index];
        div()
            .id(ElementId::Name(
                format!("notion-page-link-emoji-section-{}", section.id).into(),
            ))
            .h(px(PAGE_LINK_EMOJI_SECTION_HEADER_HEIGHT))
            .px(px(8.0))
            .pt(px(6.0))
            .text_size(px(12.0))
            .line_height(px(12.0))
            .text_color(rgb(self.theme.text_muted))
            .child(section.label)
            .into_any_element()
    }

    fn render_page_link_emoji_catalog_row(
        &self,
        catalog_row: PageLinkEmojiCatalogRow,
        context: PageLinkEmojiListRender<'_>,
        cx: &mut App,
    ) -> AnyElement {
        let PageLinkEmojiCatalogRow {
            section_index,
            row_index,
            emoji_start_index,
        } = catalog_row;
        if section_index == PAGE_LINK_CUSTOM_EMOJI_SECTION_INDEX {
            return self.render_page_link_custom_emoji_row(row_index, &context, cx);
        }
        let section = &emoji_sections()[section_index];
        let start = row_index * PAGE_LINK_EMOJI_COLUMN_COUNT;
        let end = (start + PAGE_LINK_EMOJI_COLUMN_COUNT).min(section.emojis.len());
        section.emojis[start..end]
            .iter()
            .enumerate()
            .fold(
                page_link_emoji_catalog_row(section.id, row_index),
                |row, (column, emoji)| {
                    row.child(self.render_page_link_emoji_cell(
                        context.target,
                        emoji_start_index + column,
                        emoji,
                        context.skin_tone,
                    ))
                },
            )
            .when(section.emojis.is_empty(), |row| {
                row.text_size(px(12.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child("No custom emoji yet")
            })
            .into_any_element()
    }
}

fn page_link_emoji_catalog_row(section_id: &str, row_index: usize) -> Stateful<Div> {
    div()
        .id(ElementId::Name(
            format!("notion-page-link-emoji-row-{section_id}-{row_index}").into(),
        ))
        .role(Role::Row)
        .w_full()
        .h(px(PAGE_LINK_EMOJI_ROW_HEIGHT))
        .px(px(12.0))
        .flex()
        .items_center()
}
