use gpui::App;
use gpui_components::backdrop::dismissible_backdrop_with_handler;

use crate::ui::{
    alpha, div, img, px, rgb, AnyElement, AppearanceMode, Div, InteractiveElement, IntoElement,
    MouseButton, NotionSearchResultsState, ParentElement, Styled,
};

use super::super::QuickFindAction;

use super::{
    controls::{
        notion_search_header_control, notion_search_loading_separator, notion_search_shadows,
    },
    QuickFindView, SEARCH_BODY_HEIGHT, SEARCH_DARK_BORDER_FILL, SEARCH_HEADER_HEIGHT,
    SEARCH_LOADED_HEADER_LEFT_INSET, SEARCH_LOADING_BODY_HEIGHT, SEARCH_LOADING_HEADER_HEIGHT,
    SEARCH_LOADING_HEADER_HORIZONTAL_INSET, SEARCH_LOADING_SEARCH_ICON_SIZE,
    SEARCH_MODAL_CONTENT_WIDTH, SEARCH_MODAL_OUTER_HEIGHT, SEARCH_MODAL_OUTER_WIDTH,
    SEARCH_PREVIEW_WIDTH, SEARCH_RESULTS_WIDTH, SEARCH_SEARCH_ICON_SIZE,
};

impl QuickFindView {
    pub(in crate::ui::search) fn render(&self, cx: &mut App) -> AnyElement {
        let loading = matches!(&self.state.results, NotionSearchResultsState::Loading);
        let modal = self.notion_search_modal_shell(loading);
        let modal = if loading {
            modal
                .child(self.render_notion_search_loading_header(cx))
                .child(self.render_notion_search_loading_filters())
                .child(self.render_notion_search_loading_body())
                .child(self.render_notion_search_loading_footer())
        } else {
            modal
                .child(self.render_notion_search_header(cx))
                .child(self.render_notion_search_filters(cx))
                .child(self.render_notion_search_body(cx))
                .child(self.render_notion_search_footer(cx))
        };
        div()
            .id("notion-search-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(self.viewport.logical_height as f32 * 0.09 - 1.0))
            .child(dismissible_backdrop_with_handler(
                div().absolute().inset_0(),
                self.actions.listener(|_, _, _| QuickFindAction::Close),
            ))
            .child(modal)
            .into_any_element()
    }

    fn notion_search_modal_shell(&self, loading: bool) -> gpui::Stateful<Div> {
        div()
            .id("notion-search-modal")
            .debug_selector(|| "notion-search-modal".to_string())
            .w(px(SEARCH_MODAL_OUTER_WIDTH))
            .min_w(px(SEARCH_MODAL_OUTER_WIDTH))
            .max_w(px(SEARCH_MODAL_OUTER_WIDTH))
            .h(px(SEARCH_MODAL_OUTER_HEIGHT))
            .rounded(px(20.0))
            .overflow_hidden()
            .bg(match (self.appearance_mode, loading) {
                (AppearanceMode::Light, true) | (AppearanceMode::Dark, true) => {
                    rgb(self.theme.elevated_surface_bg).into()
                }
                (AppearanceMode::Light, false) => alpha(0xffffff, 0.94),
                (AppearanceMode::Dark, false) => alpha(0x202020, 0.90),
            })
            .border_1()
            .border_color(rgb(match (self.appearance_mode, loading) {
                (AppearanceMode::Light, _) => 0xe6e6e4,
                (AppearanceMode::Dark, true) => 0x323232,
                (AppearanceMode::Dark, false) => SEARCH_DARK_BORDER_FILL,
            }))
            .shadow(notion_search_shadows(self.appearance_mode))
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    }

    fn render_notion_search_header(&self, cx: &mut App) -> Div {
        let hide_highlight_pane = self.icons.quick_find_hide_highlight_pane.render(cx);
        let hide_filters = self.icons.quick_find_hide_filters.render(cx);
        div()
            .debug_selector(|| "notion-search-header".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_HEADER_HEIGHT))
            .flex_none()
            .pl(px(SEARCH_LOADED_HEADER_LEFT_INSET))
            .pr(px(10.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(img(self.icons.quick_find_search.render(cx)).size(px(SEARCH_SEARCH_ICON_SIZE)))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .child(self.input.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(notion_search_header_control(
                        hide_highlight_pane,
                        "notion-search-hide-highlight-pane",
                    ))
                    .child(notion_search_header_control(
                        hide_filters,
                        "notion-search-hide-filters",
                    )),
            )
    }

    fn render_notion_search_loading_header(&self, cx: &mut App) -> Div {
        div()
            .debug_selector(|| "notion-search-loading-header".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_LOADING_HEADER_HEIGHT))
            .flex_none()
            .px(px(SEARCH_LOADING_HEADER_HORIZONTAL_INSET))
            .border_b_1()
            .border_color(notion_search_loading_separator(self.appearance_mode))
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(
                div()
                    .debug_selector(|| "notion-search-loading-search-icon".to_string())
                    .size(px(SEARCH_LOADING_SEARCH_ICON_SIZE))
                    .flex_none()
                    .child(
                        img(self.icons.quick_find_search.render(cx))
                            .size(px(SEARCH_LOADING_SEARCH_ICON_SIZE)),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "notion-search-loading-input".to_string())
                    .min_w(px(0.0))
                    .flex_grow(1.0)
                    .child(self.input.clone()),
            )
    }

    fn render_notion_search_body(&self, cx: &mut App) -> Div {
        let preview_top_padding = if self.state.query.trim().is_empty() {
            33.0
        } else {
            0.0
        };
        div()
            .debug_selector(|| "notion-search-body".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_BODY_HEIGHT))
            .flex_none()
            .flex()
            .child(
                div()
                    .debug_selector(|| "notion-search-results".to_string())
                    .w(px(SEARCH_RESULTS_WIDTH))
                    .min_w(px(SEARCH_RESULTS_WIDTH))
                    .max_w(px(SEARCH_RESULTS_WIDTH))
                    .h_full()
                    .flex_none()
                    .child(self.render_notion_search_results()),
            )
            .child(
                div()
                    .w(px(SEARCH_PREVIEW_WIDTH))
                    .min_w(px(SEARCH_PREVIEW_WIDTH))
                    .max_w(px(SEARCH_PREVIEW_WIDTH))
                    .h_full()
                    .flex_none()
                    .overflow_hidden()
                    .pt(px(preview_top_padding))
                    .pl(px(15.0))
                    .pr(px(13.0))
                    .child(self.render_notion_search_preview(cx)),
            )
    }

    fn render_notion_search_loading_body(&self) -> Div {
        div()
            .debug_selector(|| "notion-search-loading-body".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_LOADING_BODY_HEIGHT))
            .flex_none()
            .flex()
            .child(
                div()
                    .debug_selector(|| "notion-search-loading-results".to_string())
                    .w(px(SEARCH_RESULTS_WIDTH))
                    .min_w(px(SEARCH_RESULTS_WIDTH))
                    .max_w(px(SEARCH_RESULTS_WIDTH))
                    .h_full()
                    .flex_none()
                    .border_r_1()
                    .border_color(notion_search_loading_separator(self.appearance_mode))
                    .child(self.render_notion_search_results()),
            )
            .child(
                div()
                    .debug_selector(|| "notion-search-loading-preview-pane".to_string())
                    .w(px(SEARCH_PREVIEW_WIDTH))
                    .min_w(px(SEARCH_PREVIEW_WIDTH))
                    .max_w(px(SEARCH_PREVIEW_WIDTH))
                    .h_full()
                    .flex_none(),
            )
    }
}
