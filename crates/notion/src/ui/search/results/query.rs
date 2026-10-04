use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::PageShellSearchResult;
use gpui::{App, StyledText};

use crate::ui::{
    div, px, AnyElement, Div, FluentBuilder, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Styled, Window,
};

use super::{
    query_text::{
        notion_search_query_match_ranges, notion_search_query_result_height,
        notion_search_query_result_metadata, notion_search_query_result_stride,
        notion_search_query_snippet_layout, notion_search_result_edited_label,
        notion_search_styled_query_text, notion_search_styled_query_title,
        notion_search_text_width,
    },
    row::{
        notion_search_result_badge_label, notion_search_result_metadata_color,
        notion_search_result_title_color,
    },
    NotionSearchQuerySnippetLayout, QuickFindView, NOTION_SEARCH_QUERY_BADGE_HORIZONTAL_PADDING,
    NOTION_SEARCH_QUERY_TITLE_BADGE_GAP, NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH,
};
impl QuickFindView {
    pub(super) fn render_notion_search_query_result(
        &self,
        result_index: usize,
        result: &PageShellSearchResult,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let query = self.state.query.trim();
        let presentation =
            query_result_presentation(result, query, self.theme.text_primary, window);
        let row_stride = presentation.row_stride;
        let row = self
            .notion_search_result_shell(result_index, presentation.row_height)
            .items_start()
            .py(px(8.0))
            .child(self.render_notion_search_result_icon(result_index, result, cx))
            .child(self.render_query_result_content(result_index, result, presentation));
        div()
            .w_full()
            .h(px(row_stride))
            .px(px(14.0))
            .child(row)
            .into_any_element()
    }

    fn render_query_result_content(
        &self,
        result_index: usize,
        result: &PageShellSearchResult,
        presentation: QueryResultPresentation,
    ) -> Div {
        let QueryResultPresentation {
            title,
            snippet_layout,
            metadata,
            match_snippet,
            title_width,
            ..
        } = presentation;
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.render_query_result_title(result_index, result, title_width, title))
            .when_some(metadata, |this, metadata| {
                this.child(query_result_metadata_element(
                    result_index,
                    metadata,
                    notion_search_result_metadata_color(self.appearance_mode),
                ))
            })
            .when_some(match_snippet, |this, match_snippet| {
                this.child(query_result_snippet_element(
                    result_index,
                    match_snippet,
                    snippet_layout,
                    notion_search_result_metadata_color(self.appearance_mode),
                ))
            })
    }

    fn render_query_result_title(
        &self,
        result_index: usize,
        result: &PageShellSearchResult,
        title_width: f32,
        title: StyledText,
    ) -> Div {
        div()
            .debug_selector(move || format!("notion-search-result-{result_index}-title"))
            .w_full()
            .h(px(20.0))
            .flex_none()
            .min_w(px(0.0))
            .overflow_hidden()
            .flex()
            .items_center()
            .child(
                div()
                    .debug_selector(move || {
                        format!("notion-search-result-{result_index}-title-inline")
                    })
                    .min_w(px(0.0))
                    .flex_initial()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap(px(NOTION_SEARCH_QUERY_TITLE_BADGE_GAP))
                    .child(query_result_title_text(
                        result_index,
                        title_width,
                        title,
                        notion_search_result_title_color(self.appearance_mode),
                    ))
                    .children(
                        result.badges.iter().copied().map(|badge| {
                            self.render_notion_search_result_badge(result_index, badge)
                        }),
                    ),
            )
    }
}

struct QueryResultPresentation {
    title: StyledText,
    snippet_layout: NotionSearchQuerySnippetLayout,
    row_height: f32,
    row_stride: f32,
    metadata: Option<StyledText>,
    match_snippet: Option<StyledText>,
    title_width: f32,
}

fn query_result_presentation(
    result: &PageShellSearchResult,
    query: &str,
    title_color: u32,
    window: &mut Window,
) -> QueryResultPresentation {
    let title = notion_search_styled_query_title(
        result.title.clone(),
        notion_search_query_match_ranges(&result.title, query),
        title_color,
    );
    let snippet_layout = query_result_snippet_layout(result, window);
    let edited_label = notion_search_result_edited_label(
        result.edited_at.as_ref(),
        result.edited_label.as_deref(),
        query_result_now(result),
    );
    let metadata = notion_search_query_result_metadata(
        result.highlight.as_deref(),
        result.editor_display_name.as_deref(),
        edited_label.as_deref(),
    );
    let metadata_ranges = result
        .highlight
        .as_deref()
        .map(|breadcrumb| notion_search_query_match_ranges(breadcrumb, query))
        .unwrap_or_default();
    QueryResultPresentation {
        title,
        snippet_layout,
        row_height: notion_search_query_result_height(snippet_layout),
        row_stride: notion_search_query_result_stride(snippet_layout),
        metadata: metadata.map(|value| notion_search_styled_query_text(value, metadata_ranges)),
        match_snippet: result.match_snippet.as_ref().map(|snippet| {
            notion_search_styled_query_text(
                snippet.clone(),
                notion_search_query_match_ranges(snippet, query),
            )
        }),
        title_width: query_result_title_width(result, window),
    }
}

fn query_result_snippet_layout(
    result: &PageShellSearchResult,
    window: &mut Window,
) -> NotionSearchQuerySnippetLayout {
    result
        .match_snippet
        .as_deref()
        .map_or(NotionSearchQuerySnippetLayout::None, |snippet| {
            if snippet.contains('\n') {
                return NotionSearchQuerySnippetLayout::TwoLines;
            }
            notion_search_query_snippet_layout(
                snippet,
                notion_search_text_width(snippet, 12.0, FontWeight::NORMAL, window),
            )
        })
}

fn query_result_now(result: &PageShellSearchResult) -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_else(|_| {
            result
                .edited_at
                .as_ref()
                .map_or(0, |edited_at| edited_at.unix_millis)
        })
}

fn query_result_title_width(result: &PageShellSearchResult, window: &mut Window) -> f32 {
    let badge_width = result
        .badges
        .iter()
        .map(|badge| {
            let (label, _) = notion_search_result_badge_label(*badge);
            (notion_search_text_width(label, 12.0, FontWeight::MEDIUM, window)
                + NOTION_SEARCH_QUERY_BADGE_HORIZONTAL_PADDING)
                .ceil()
        })
        .sum::<f32>();
    let badge_gaps = NOTION_SEARCH_QUERY_TITLE_BADGE_GAP * result.badges.len() as f32;
    notion_search_text_width(result.title.as_ref(), 14.0, FontWeight::MEDIUM, window)
        .ceil()
        .min((NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH - badge_width - badge_gaps).max(0.0))
}

fn query_result_title_text(
    result_index: usize,
    title_width: f32,
    title: StyledText,
    color: gpui::Hsla,
) -> Div {
    div()
        .debug_selector(move || format!("notion-search-result-{result_index}-title-text"))
        .w(px(title_width))
        .flex_none()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(14.0))
        .line_height(px(20.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .child(title)
}

fn query_result_metadata_element(
    result_index: usize,
    metadata: StyledText,
    color: gpui::Hsla,
) -> Div {
    div()
        .mt(px(2.0))
        .debug_selector(move || format!("notion-search-result-{result_index}-metadata"))
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(12.0))
        .line_height(px(16.8))
        .text_color(color)
        .child(metadata)
}

fn query_result_snippet_element(
    result_index: usize,
    match_snippet: StyledText,
    layout: NotionSearchQuerySnippetLayout,
    color: gpui::Hsla,
) -> Div {
    let height = match layout {
        NotionSearchQuerySnippetLayout::TwoLines => 33.6,
        NotionSearchQuerySnippetLayout::OneLine | NotionSearchQuerySnippetLayout::None => 16.8,
    };
    div()
        .mt(px(4.0))
        .debug_selector(move || format!("notion-search-result-{result_index}-snippet"))
        .h(px(height))
        .flex_none()
        .overflow_hidden()
        .line_clamp(2)
        .text_size(px(12.0))
        .line_height(px(16.8))
        .text_color(color)
        .child(match_snippet)
}
