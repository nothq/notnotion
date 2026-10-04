use gpui::{App, Role};

use crate::model::SearchWorkspaceScope;
use crate::ui::{
    alpha, div, img, px, rgb, Div, IconAsset, InteractiveElement, MouseButton, MouseDownEvent,
    ParentElement, StatefulInteractiveElement, Styled,
};

use super::super::QuickFindAction;

use super::{
    controls::{notion_search_loading_separator, notion_search_neutral_tint},
    QuickFindView, SEARCH_FILTERS_HEIGHT, SEARCH_LOADING_FILTERS_HEIGHT,
    SEARCH_MODAL_CONTENT_WIDTH,
};

/// A search filter chip's label, test selector, sizes, and leading icon.
struct NotionSearchFilterControl<'a> {
    label: &'static str,
    selector: &'static str,
    width: f32,
    icon_width: f32,
    icon: &'a IconAsset,
}

impl QuickFindView {
    pub(super) fn render_notion_search_filters(&self, cx: &mut App) -> Div {
        div()
            .debug_selector(|| "notion-search-filters".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_FILTERS_HEIGHT))
            .flex_none()
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(self.render_notion_search_title_filter(cx))
            .child(self.render_notion_search_filter_control(
                NotionSearchFilterControl {
                    label: "Created by",
                    selector: "notion-search-created-by-filter",
                    width: 120.094,
                    icon_width: 12.64,
                    icon: &self.icons.quick_find_person,
                },
                cx,
            ))
            .child(self.render_notion_search_filter_control(
                NotionSearchFilterControl {
                    label: "In",
                    selector: "notion-search-in-filter",
                    width: 60.008,
                    icon_width: 11.75,
                    icon: &self.icons.quick_find_page,
                },
                cx,
            ))
            .child(self.render_notion_search_add_filter_control(cx))
    }

    pub(super) fn render_notion_search_loading_filters(&self) -> Div {
        div()
            .debug_selector(|| "notion-search-loading-filters".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_LOADING_FILTERS_HEIGHT))
            .flex_none()
            .px(px(16.0))
            .border_b_1()
            .border_color(notion_search_loading_separator(self.appearance_mode))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(self.render_notion_search_loading_title_filter())
            .children(
                [
                    ("Created by", "notion-search-loading-created-by-filter"),
                    ("In", "notion-search-loading-in-filter"),
                    ("Filter", "notion-search-loading-add-filter"),
                ]
                .map(|(label, selector)| {
                    div()
                        .debug_selector(move || selector.to_string())
                        .h(px(28.0))
                        .px(px(10.0))
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(alpha(self.theme.text_primary, 0.10))
                        .flex()
                        .items_center()
                        .text_size(px(13.0))
                        .text_color(rgb(self.theme.text_secondary))
                        .child(label)
                }),
            )
    }

    fn render_notion_search_title_filter(&self, cx: &mut App) -> gpui::Stateful<Div> {
        let title_only = self.state.scope == SearchWorkspaceScope::TitleOnly;
        let inactive_background = notion_search_neutral_tint(self.appearance_mode, 0.0);
        let inactive_hover_background = notion_search_neutral_tint(self.appearance_mode, 0.08);
        let title_icon = if title_only {
            &self.icons.quick_find_title_only_active
        } else {
            &self.icons.quick_find_title_only
        }
        .render(cx);
        div()
            .id("notion-search-title-only")
            .debug_selector(|| "notion-search-title-only".to_string())
            .role(Role::Button)
            .aria_label("Search titles only")
            .w(px(97.398))
            .min_w(px(97.398))
            .max_w(px(97.398))
            .h(px(24.0))
            .flex_none()
            .px(px(8.0))
            .rounded_full()
            .bg(if title_only {
                alpha(0x2383e2, 0.14)
            } else {
                inactive_background
            })
            .cursor_pointer()
            .hover(move |style| {
                style.bg(if title_only {
                    alpha(0x2383e2, 0.18)
                } else {
                    inactive_hover_background
                })
            })
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    QuickFindAction::ToggleTitleOnly
                }),
            )
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .line_height(px(24.0))
            .text_color(rgb(if title_only {
                0x2383e2
            } else {
                self.theme.text_muted
            }))
            .child(img(title_icon).w(px(16.56)).h(px(20.0)))
            .child("Title only")
    }

    fn render_notion_search_loading_title_filter(&self) -> gpui::Stateful<Div> {
        let title_only = self.state.scope == SearchWorkspaceScope::TitleOnly;
        div()
            .id("notion-search-loading-title-only")
            .debug_selector(|| "notion-search-loading-title-only".to_string())
            .role(Role::Button)
            .aria_label("Search titles only")
            .h(px(28.0))
            .flex_none()
            .px(px(10.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(if title_only {
                alpha(0x2383e2, 0.48)
            } else {
                alpha(self.theme.text_primary, 0.10)
            })
            .bg(if title_only {
                alpha(0x2383e2, 0.14)
            } else {
                alpha(self.theme.elevated_surface_bg, 0.0)
            })
            .cursor_pointer()
            .hover(|style| style.bg(alpha(0x2383e2, 0.12)))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    QuickFindAction::ToggleTitleOnly
                }),
            )
            .flex()
            .items_center()
            .text_size(px(13.0))
            .text_color(rgb(if title_only {
                0x2383e2
            } else {
                self.theme.text_secondary
            }))
            .child("Title only")
    }

    fn render_notion_search_filter_control(
        &self,
        control: NotionSearchFilterControl<'_>,
        cx: &mut App,
    ) -> Div {
        let NotionSearchFilterControl {
            label,
            selector,
            width,
            icon_width,
            icon,
        } = control;
        let icon = icon.render(cx);
        let chevron = self.icons.quick_find_filter_chevron.render(cx);
        div()
            .debug_selector(move || selector.to_string())
            .w(px(width))
            .min_w(px(width))
            .max_w(px(width))
            .h(px(24.0))
            .flex_none()
            .px(px(8.0))
            .rounded_full()
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .line_height(px(24.0))
            .text_color(rgb(self.theme.text_muted))
            .child(img(icon).w(px(icon_width)).h(px(20.0)))
            .child(label)
            .child(img(chevron).w(px(8.64)).h(px(14.0)))
    }

    fn render_notion_search_add_filter_control(&self, cx: &mut App) -> Div {
        let add_filter = self.icons.quick_find_add_filter.render(cx);
        div()
            .debug_selector(|| "notion-search-add-filter".to_string())
            .w(px(61.633))
            .min_w(px(61.633))
            .max_w(px(61.633))
            .h(px(24.0))
            .flex_none()
            .pl(px(5.0))
            .pr(px(9.0))
            .rounded(px(12.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(14.0))
            .line_height(px(24.0))
            .text_color(rgb(self.theme.text_muted))
            .child(img(add_filter).w(px(9.203)).h(px(14.0)))
            .child("Filter")
    }
}
