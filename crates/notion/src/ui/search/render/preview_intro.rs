use crate::model::PageShellSearchResult;
use crate::ui::{div, px, AnyElement, IntoElement, ParentElement, Styled};
use gpui::App;

use super::QuickFindView;

impl QuickFindView {
    pub(super) fn render_notion_search_preview_intro(
        &self,
        result: &PageShellSearchResult,
        preview_is_empty: bool,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .w_full()
            .relative()
            .child(self.render_notion_search_preview_header())
            .child(self.render_notion_search_preview_actions(cx))
            .child(self.render_notion_search_preview_identity(
                result,
                div().w_full().h(px(12.0)).flex_none().into_any_element(),
                preview_is_empty,
                cx,
            ))
            .into_any_element()
    }
}
