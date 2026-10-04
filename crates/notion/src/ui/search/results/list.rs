use gpui::{list, App, ListScrollEvent, ListSizingBehavior};

use crate::model::PageShellSearchResult;
use crate::ui::surface::NotionSearchListRow;
use crate::ui::{
    div, img, px, rgb, AnyElement, FontWeight, InteractiveElement, IntoElement,
    NotionSearchResultsState, ParentElement, Styled, Window,
};

use super::super::QuickFindAction;

use super::{
    query_text::notion_search_should_paginate, QuickFindView, NOTION_SEARCH_QUERY_HEADER_HEIGHT,
    NOTION_SEARCH_RECENCY_HEADER_HEIGHT,
};

/// The result total and results a loaded search list renders its rows from.
struct LoadedNotionSearchResults<'a> {
    total: u32,
    results: &'a [PageShellSearchResult],
}

impl QuickFindView {
    pub(in crate::ui::search) fn render_notion_search_results(&self) -> AnyElement {
        match &self.state.results {
            NotionSearchResultsState::Loading => self.render_notion_search_skeleton(),
            NotionSearchResultsState::Idle | NotionSearchResultsState::Failed => {
                div().size_full().into_any_element()
            }
            NotionSearchResultsState::Loaded {
                total,
                has_more,
                results,
            } => self.render_loaded_notion_search_results(*total, *has_more, results.clone()),
        }
    }

    fn render_loaded_notion_search_results(
        &self,
        total: u32,
        _has_more: bool,
        results: std::sync::Arc<[PageShellSearchResult]>,
    ) -> AnyElement {
        let view = self.clone();
        let list_state = self.state.list_state.clone();
        let actions = self.actions.clone();
        list_state.set_scroll_handler(move |event: &ListScrollEvent, window, cx| {
            if notion_search_should_paginate(event.visible_range.end, event.count) {
                actions.emit(QuickFindAction::LoadMore, window, cx);
            }
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                list(list_state, move |list_row_index, window, cx| {
                    view.render_notion_search_list_row(
                        list_row_index,
                        LoadedNotionSearchResults {
                            total,
                            results: &results,
                        },
                        window,
                        cx,
                    )
                })
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .flex_grow(1.0)
                .min_h(px(0.0)),
            )
            .into_any_element()
    }

    fn render_notion_search_list_row(
        &self,
        list_row_index: usize,
        loaded: LoadedNotionSearchResults<'_>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let LoadedNotionSearchResults { total, results } = loaded;
        match self.state.list_rows.get(list_row_index).copied() {
            Some(NotionSearchListRow::QueryHeader) => {
                self.render_notion_search_query_header(total, cx)
            }
            Some(NotionSearchListRow::RecencyHeader(bucket)) => {
                self.render_notion_search_recency_header(bucket.label(), bucket.selector_segment())
            }
            Some(NotionSearchListRow::QueryResult(result_index)) => self
                .render_notion_search_query_result(
                    result_index,
                    results
                        .get(result_index)
                        .expect("Notion query result row should map to a result"),
                    window,
                    cx,
                ),
            Some(NotionSearchListRow::RecentResult(result_index)) => self
                .render_notion_search_recent_result(
                    result_index,
                    results
                        .get(result_index)
                        .expect("Notion recent result row should map to a result"),
                    cx,
                ),
            None => panic!("Notion search list row index should exist"),
        }
    }

    fn render_notion_search_query_header(&self, total: u32, cx: &mut App) -> AnyElement {
        let chevron = self.icons.quick_find_filter_chevron.render(cx);
        div()
            .debug_selector(|| "notion-search-query-header".to_string())
            .w_full()
            .h(px(NOTION_SEARCH_QUERY_HEADER_HEIGHT))
            .flex_none()
            .px(px(21.0))
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(12.0))
            .line_height(px(14.4))
            .text_color(rgb(self.theme.text_muted))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(format!("Search results ({total})")),
            )
            .child(
                div()
                    .pr(px(4.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child("Best matches")
                    .child(img(chevron).w(px(8.64)).h(px(14.0))),
            )
            .into_any_element()
    }

    fn render_notion_search_recency_header(
        &self,
        label: &'static str,
        selector_segment: &'static str,
    ) -> AnyElement {
        div()
            .debug_selector(move || format!("notion-search-recency-{selector_segment}"))
            .w_full()
            .h(px(NOTION_SEARCH_RECENCY_HEADER_HEIGHT))
            .flex_none()
            .px(px(21.0))
            .flex()
            .items_center()
            .text_size(px(14.0))
            .line_height(px(20.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_muted))
            .child(label)
            .into_any_element()
    }
}
