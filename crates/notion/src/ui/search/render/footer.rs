use gpui::App;

use crate::ui::{div, px, rgb, Div, InteractiveElement, ParentElement, Styled};

use super::{
    controls::{
        notion_search_border_tint, notion_search_footer_control, notion_search_footer_key_color,
        notion_search_footer_label_color, notion_search_loading_separator, notion_search_shortcut,
    },
    QuickFindView, SEARCH_FOOTER_HEIGHT, SEARCH_MODAL_CONTENT_WIDTH,
};

impl QuickFindView {
    pub(super) fn render_notion_search_footer(&self, cx: &mut App) -> Div {
        div()
            .debug_selector(|| "notion-search-footer".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_FOOTER_HEIGHT))
            .flex_none()
            .pt(px(8.0))
            .pr(px(12.0))
            .pb(px(8.0))
            .pl(px(16.0))
            .border_t_1()
            .border_color(notion_search_border_tint(self.appearance_mode))
            .flex()
            .items_center()
            .justify_between()
            .child(self.render_notion_search_footer_shortcuts())
            .child(self.render_notion_search_footer_controls(cx))
    }

    fn render_notion_search_footer_shortcuts(&self) -> Div {
        let key_color = notion_search_footer_key_color(self.appearance_mode);
        let label_color = notion_search_footer_label_color(self.appearance_mode);
        div()
            .flex()
            .items_center()
            .gap(px(20.0))
            .child(notion_search_shortcut(
                "⌘ ↵",
                "Open in new tab",
                key_color,
                label_color,
            ))
            .child(notion_search_shortcut(
                "⌘ L",
                "Copy link",
                key_color,
                label_color,
            ))
            .child(notion_search_shortcut(
                "⇧ ⌘ K",
                "Command Search",
                key_color,
                label_color,
            ))
    }

    fn render_notion_search_footer_controls(&self, cx: &mut App) -> Div {
        let thumb_up = self.icons.quick_find_thumb_up.render(cx);
        let thumb_down = self.icons.quick_find_thumb_down.render(cx);
        let settings = self.icons.toolbar_settings.render(cx);
        div()
            .flex()
            .items_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(notion_search_footer_control(
                        thumb_up,
                        20.0,
                        16.0,
                        "notion-search-helpful",
                    ))
                    .child(notion_search_footer_control(
                        thumb_down,
                        20.0,
                        16.0,
                        "notion-search-not-helpful",
                    )),
            )
            .child(notion_search_footer_control(
                settings,
                24.0,
                16.0,
                "notion-search-settings",
            ))
    }

    pub(super) fn render_notion_search_loading_footer(&self) -> Div {
        div()
            .debug_selector(|| "notion-search-loading-footer".to_string())
            .w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .min_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .max_w(px(SEARCH_MODAL_CONTENT_WIDTH))
            .h(px(SEARCH_FOOTER_HEIGHT))
            .flex_none()
            .px(px(16.0))
            .border_t_1()
            .border_color(notion_search_loading_separator(self.appearance_mode))
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(12.0))
            .line_height(px(18.0))
            .text_color(rgb(self.theme.text_muted))
            .child(
                div()
                    .debug_selector(|| "notion-search-loading-footer-actions".to_string())
                    .child("↑↓ Select   ↵ Open   ⌘↵ New tab"),
            )
            .child(
                div()
                    .debug_selector(|| "notion-search-loading-footer-close".to_string())
                    .child("esc Close"),
            )
    }
}
