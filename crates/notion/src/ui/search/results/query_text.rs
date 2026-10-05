use std::ops::Range;

use gpui::{HighlightStyle, StyledText, TextRun};

use crate::model::PageShellEditedAt;
use crate::ui::{px, rgb, FontWeight, Window};

use super::{
    NotionSearchQuerySnippetLayout, NOTION_SEARCH_PAGINATION_PREFETCH_ROWS,
    NOTION_SEARCH_QUERY_MATCH_COLOR, NOTION_SEARCH_QUERY_ROW_HEIGHT,
    NOTION_SEARCH_QUERY_SNIPPET_ROW_HEIGHT, NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH,
    NOTION_SEARCH_QUERY_TWO_LINE_SNIPPET_ROW_HEIGHT,
};
pub(super) fn notion_search_text_width(
    text: &str,
    font_size: f32,
    font_weight: FontWeight,
    window: &mut Window,
) -> f32 {
    let text_style = window.text_style();
    let mut font = text_style.font();
    font.weight = font_weight;
    let run = TextRun {
        len: text.len(),
        font,
        color: text_style.color,
        ..Default::default()
    };
    f32::from(
        window
            .text_system()
            .layout_line(text, px(font_size), &[run], None)
            .width,
    )
}

pub(super) fn notion_search_query_snippet_layout(
    match_snippet: &str,
    measured_width: f32,
) -> NotionSearchQuerySnippetLayout {
    if match_snippet.contains('\n') || measured_width > NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH {
        NotionSearchQuerySnippetLayout::TwoLines
    } else {
        NotionSearchQuerySnippetLayout::OneLine
    }
}

pub(super) const fn notion_search_query_result_height(
    layout: NotionSearchQuerySnippetLayout,
) -> f32 {
    match layout {
        NotionSearchQuerySnippetLayout::None => NOTION_SEARCH_QUERY_ROW_HEIGHT,
        NotionSearchQuerySnippetLayout::OneLine => NOTION_SEARCH_QUERY_SNIPPET_ROW_HEIGHT,
        NotionSearchQuerySnippetLayout::TwoLines => NOTION_SEARCH_QUERY_TWO_LINE_SNIPPET_ROW_HEIGHT,
    }
}

pub(super) const fn notion_search_query_result_stride(
    layout: NotionSearchQuerySnippetLayout,
) -> f32 {
    match layout {
        NotionSearchQuerySnippetLayout::None => NOTION_SEARCH_QUERY_ROW_HEIGHT,
        NotionSearchQuerySnippetLayout::OneLine => NOTION_SEARCH_QUERY_SNIPPET_ROW_HEIGHT,
        NotionSearchQuerySnippetLayout::TwoLines => {
            NOTION_SEARCH_QUERY_TWO_LINE_SNIPPET_ROW_HEIGHT + 1.0
        }
    }
}

pub(super) const fn notion_search_should_paginate(
    visible_row_end: usize,
    row_count: usize,
) -> bool {
    row_count > 0
        && row_count.saturating_sub(visible_row_end) <= NOTION_SEARCH_PAGINATION_PREFETCH_ROWS
}

pub(super) fn notion_search_query_result_metadata(
    breadcrumb: Option<&str>,
    editor_display_name: Option<&str>,
    edited_label: Option<&str>,
) -> Option<String> {
    let parts = [breadcrumb, editor_display_name, edited_label]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

pub(super) fn notion_search_result_edited_label(
    edited_at: Option<&PageShellEditedAt>,
    legacy_label: Option<&str>,
    now_unix_millis: u64,
) -> Option<String> {
    let Some(edited_at) = edited_at else {
        return legacy_label.map(str::to_string);
    };
    let diff_hours = (now_unix_millis.saturating_sub(edited_at.unix_millis) / 3_600_000).max(1);
    if diff_hours < 24 {
        return Some(format!("Edited {diff_hours}h ago"));
    }

    let diff_days = diff_hours / 24;
    if diff_days < 7 {
        return Some(format!("Edited {diff_days}d ago"));
    }

    Some(format!("Edited {}", edited_at.date_label))
}

pub(super) fn notion_search_styled_query_text(
    text: String,
    match_ranges: Vec<Range<usize>>,
) -> StyledText {
    StyledText::new(text).with_highlights(match_ranges.into_iter().map(|range| {
        (
            range,
            HighlightStyle {
                color: Some(rgb(NOTION_SEARCH_QUERY_MATCH_COLOR).into()),
                ..Default::default()
            },
        )
    }))
}

pub(super) fn notion_search_styled_query_title(
    text: String,
    match_ranges: Vec<Range<usize>>,
    match_color: u32,
) -> StyledText {
    StyledText::new(text).with_highlights(match_ranges.into_iter().map(|range| {
        (
            range,
            HighlightStyle {
                color: Some(rgb(match_color).into()),
                ..Default::default()
            },
        )
    }))
}

pub(super) fn notion_search_query_match_ranges(value: &str, query: &str) -> Vec<Range<usize>> {
    let query = query.trim();
    let query_character_count = query.chars().count();
    if query_character_count == 0 {
        return Vec::new();
    }

    let boundaries = value
        .char_indices()
        .map(|(byte_offset, _)| byte_offset)
        .chain(std::iter::once(value.len()))
        .collect::<Vec<_>>();
    if query_character_count >= boundaries.len() {
        return Vec::new();
    }

    let lowercase_query = query.to_lowercase();
    let mut ranges = Vec::new();
    let mut boundary_index = 0;
    while boundary_index + query_character_count < boundaries.len() {
        let start = boundaries[boundary_index];
        let end = boundaries[boundary_index + query_character_count];
        if value[start..end].to_lowercase() == lowercase_query {
            ranges.push(start..end);
            boundary_index += query_character_count;
        } else {
            boundary_index += 1;
        }
    }
    ranges
}
