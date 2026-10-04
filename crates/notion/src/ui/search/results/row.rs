use crate::model::{PageShellIcon, PageShellSearchBadge, PageShellSearchResult};
use crate::ui::{
    alpha, div, img, px, rgb, AnyElement, AppearanceMode, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled,
};
use gpui::App;

use super::super::QuickFindAction;

use super::{
    NotionSearchResultIconArtwork, QuickFindView, NOTION_SEARCH_LOADING_SKELETON_ROW_COUNT,
    NOTION_SEARCH_LOADING_SKELETON_ROW_GAP, NOTION_SEARCH_LOADING_SKELETON_ROW_HEIGHT,
    NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH, NOTION_SEARCH_RECENT_TITLE_MAX_WIDTH,
};
impl QuickFindView {
    pub(super) fn render_notion_search_recent_result(
        &self,
        result_index: usize,
        result: &PageShellSearchResult,
        cx: &mut App,
    ) -> AnyElement {
        let row = self
            .notion_search_result_shell(result_index, 36.0)
            .child(self.render_notion_search_result_icon(result_index, result, cx))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px(14.0))
                    .line_height(px(16.8))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .min_w(px(0.0))
                            .max_w(px(if result.highlight.is_some() {
                                NOTION_SEARCH_RECENT_TITLE_MAX_WIDTH
                            } else {
                                NOTION_SEARCH_QUERY_TITLE_LINE_WIDTH
                            }))
                            .flex_initial()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(notion_search_result_title_color(self.appearance_mode))
                            .child(result.title.clone()),
                    )
                    .when_some(result.highlight.as_ref(), |this, caption| {
                        this.child(
                            div()
                                .min_w(px(0.0))
                                .flex_grow(1.0)
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_color(notion_search_result_metadata_color(
                                    self.appearance_mode,
                                ))
                                .child(format!(" — {caption}")),
                        )
                    }),
            );
        div()
            .w_full()
            .h(px(36.0))
            .px(px(13.0))
            .child(row)
            .into_any_element()
    }

    pub(super) fn render_notion_search_result_badge(
        &self,
        result_index: usize,
        badge: PageShellSearchBadge,
    ) -> Div {
        let (label, selector_segment) = notion_search_result_badge_label(badge);
        div()
            .debug_selector(move || {
                format!("notion-search-result-{result_index}-badge-{selector_segment}")
            })
            .h(px(18.0))
            .flex_none()
            .px(px(5.0))
            .rounded(px(4.0))
            .bg(notion_search_neutral_tint(self.appearance_mode, 0.08))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_muted))
            .child(label)
    }

    pub(super) fn render_notion_search_result_icon(
        &self,
        result_index: usize,
        result: &PageShellSearchResult,
        cx: &mut App,
    ) -> Div {
        let icon = match notion_search_result_icon_artwork(&result.badges, &result.icon) {
            NotionSearchResultIconArtwork::Database => {
                img(self.icons.quick_find_database.render(cx))
                    .size(px(20.0))
                    .into_any_element()
            }
            NotionSearchResultIconArtwork::DefaultPage => {
                img(self.icons.quick_find_page.render(cx))
                    .size(px(20.0))
                    .into_any_element()
            }
            NotionSearchResultIconArtwork::Explicit => {
                self.page_icons.render(&result.icon, 20.0, cx)
            }
        };
        div()
            .debug_selector(move || format!("notion-search-result-{result_index}-icon-slot"))
            .size(px(20.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(icon)
    }

    pub(super) fn notion_search_result_shell(
        &self,
        result_index: usize,
        height: f32,
    ) -> gpui::Stateful<gpui::Div> {
        let selected_background = notion_search_neutral_tint(self.appearance_mode, 0.054902);
        div()
            .id(format!("notion-search-result-{result_index}"))
            .debug_selector(move || format!("notion-search-result-{result_index}"))
            .w_full()
            .h(px(height))
            .px(px(8.0))
            .rounded(px(12.0))
            .cursor_pointer()
            .when(result_index == self.state.selected_index, |this| {
                this.bg(selected_background)
            })
            .hover(move |style| style.bg(selected_background))
            .flex()
            .items_center()
            .gap(px(8.0))
            .on_hover({
                let actions = self.actions.clone();
                move |hovered: &bool, window, cx| {
                    if *hovered {
                        actions.emit(QuickFindAction::SelectResult(result_index), window, cx);
                    }
                }
            })
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(move |_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    QuickFindAction::SelectAndOpen(result_index)
                }),
            )
    }

    pub(super) fn render_notion_search_skeleton(&self) -> AnyElement {
        div()
            .debug_selector(|| "notion-search-loading-skeleton".to_string())
            .size_full()
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(NOTION_SEARCH_LOADING_SKELETON_ROW_GAP))
            .children((0..NOTION_SEARCH_LOADING_SKELETON_ROW_COUNT).map(|index| {
                div()
                    .debug_selector(move || format!("notion-search-loading-skeleton-row-{index}"))
                    .w(px(notion_search_loading_skeleton_row_width(index)))
                    .h(px(NOTION_SEARCH_LOADING_SKELETON_ROW_HEIGHT))
                    .rounded(px(6.0))
                    .bg(alpha(self.theme.text_primary, 0.10))
            }))
            .into_any_element()
    }
}

fn notion_search_neutral_tint(appearance_mode: AppearanceMode, opacity: f32) -> gpui::Hsla {
    match appearance_mode {
        AppearanceMode::Light => alpha(0x37352f, opacity),
        AppearanceMode::Dark => alpha(0xffffff, opacity),
    }
}

pub(super) fn notion_search_result_title_color(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(match appearance_mode {
        AppearanceMode::Light => 0x37352f,
        AppearanceMode::Dark => 0xbcbab6,
    })
    .into()
}

pub(super) fn notion_search_result_metadata_color(appearance_mode: AppearanceMode) -> gpui::Hsla {
    rgb(match appearance_mode {
        AppearanceMode::Light => 0x787774,
        AppearanceMode::Dark => 0x7d7a75,
    })
    .into()
}
pub(super) fn notion_search_result_icon_artwork(
    badges: &[PageShellSearchBadge],
    icon: &PageShellIcon,
) -> NotionSearchResultIconArtwork {
    if badges.contains(&PageShellSearchBadge::Database) {
        NotionSearchResultIconArtwork::Database
    } else if icon.kind == "named" && icon.value == "page" {
        NotionSearchResultIconArtwork::DefaultPage
    } else {
        NotionSearchResultIconArtwork::Explicit
    }
}

pub(super) const fn notion_search_loading_skeleton_row_width(index: usize) -> f32 {
    if index.is_multiple_of(2) {
        280.0
    } else {
        220.0
    }
}

pub(super) const fn notion_search_result_badge_label(
    badge: PageShellSearchBadge,
) -> (&'static str, &'static str) {
    match badge {
        PageShellSearchBadge::CurrentPage => ("Current Page", "current-page"),
        PageShellSearchBadge::Database => ("Database", "database"),
    }
}
