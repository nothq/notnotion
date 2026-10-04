use crate::model::PageShellSearchResult;
use crate::ui::{
    div, px, rgb, AnyElement, AppearanceMode, Div, FluentBuilder, FontWeight, InteractiveElement,
    MouseButton, MouseDownEvent, ParentElement, Styled,
};
use gpui::App;

use super::super::QuickFindAction;

use super::{
    controls::notion_search_preview_control,
    preview_style::{notion_search_preview_has_explicit_icon, notion_search_preview_header_fill},
    QuickFindView,
};

impl QuickFindView {
    pub(super) fn render_notion_search_preview_header(&self) -> Div {
        div()
            .debug_selector(|| "notion-search-preview-header".to_string())
            .w_full()
            .h(px(80.0))
            .flex_none()
            .bg(rgb(notion_search_preview_header_fill(self.appearance_mode)))
    }

    pub(super) fn render_notion_search_preview_actions(&self, cx: &mut App) -> Div {
        div()
            .debug_selector(|| "notion-search-preview-actions".to_string())
            .absolute()
            .top(px(6.0))
            .right(px(7.0))
            .w(px(54.0))
            .h(px(30.0))
            .p(px(2.0))
            .rounded(px(7.0))
            .bg(rgb(notion_search_preview_header_fill(self.appearance_mode)))
            .border_1()
            .border_color(rgb(match self.appearance_mode {
                AppearanceMode::Light => 0xe6e6e4,
                AppearanceMode::Dark => 0x383836,
            }))
            .flex()
            .items_center()
            .child(self.render_notion_search_preview_link_control(cx))
            .child(self.render_notion_search_preview_open_control(cx))
    }

    fn render_notion_search_preview_link_control(&self, cx: &mut App) -> gpui::Stateful<Div> {
        notion_search_preview_control(
            self.icons.quick_find_preview_link.render(cx),
            11.93,
            "notion-search-preview-link",
            self.appearance_mode,
        )
        .on_mouse_down(
            MouseButton::Left,
            self.actions.listener(|_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                QuickFindAction::CopySelectedLink
            }),
        )
    }

    fn render_notion_search_preview_open_control(&self, cx: &mut App) -> gpui::Stateful<Div> {
        notion_search_preview_control(
            self.icons.quick_find_preview_open.render(cx),
            8.922,
            "notion-search-preview-open",
            self.appearance_mode,
        )
        .on_mouse_down(
            MouseButton::Left,
            self.actions.listener(|_: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                QuickFindAction::OpenSelectedInNewTab
            }),
        )
    }

    pub(super) fn render_notion_search_preview_identity(
        &self,
        result: &PageShellSearchResult,
        body: AnyElement,
        pad_bottom: bool,
        cx: &mut App,
    ) -> Div {
        div()
            .relative()
            .px(px(24.0))
            .pt(px(28.0))
            .when(pad_bottom, |this| this.pb(px(24.0)))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .when(
                notion_search_preview_has_explicit_icon(&result.icon),
                |this| this.child(self.render_notion_search_preview_icon_slot(result, cx)),
            )
            .when_some(result.highlight.as_ref(), |this, breadcrumb| {
                this.child(self.render_notion_search_preview_breadcrumb(breadcrumb.clone()))
            })
            .child(self.render_notion_search_preview_title(result))
            .child(body)
    }

    fn render_notion_search_preview_icon_slot(
        &self,
        result: &PageShellSearchResult,
        cx: &mut App,
    ) -> Div {
        div()
            .debug_selector(|| "notion-search-preview-icon-slot".to_string())
            .absolute()
            .top(px(-28.0))
            .left(px(24.0))
            .size(px(48.0))
            .flex()
            .items_center()
            .justify_center()
            .child(self.page_icons.render(&result.icon, 36.0, cx))
    }

    fn render_notion_search_preview_breadcrumb(&self, breadcrumb: String) -> Div {
        div()
            .h(px(18.0))
            .flex_none()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(12.0))
            .line_height(px(18.0))
            .text_color(rgb(self.theme.text_muted))
            .child(breadcrumb)
    }

    fn render_notion_search_preview_title(&self, result: &PageShellSearchResult) -> Div {
        div()
            .debug_selector(|| "notion-search-preview-title".to_string())
            .flex_none()
            .overflow_hidden()
            .text_size(px(20.0))
            .line_height(px(24.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(self.theme.text_primary))
            .child(result.title.clone())
    }
}
