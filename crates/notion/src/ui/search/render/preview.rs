use gpui::{list, App, ListSizingBehavior, ScrollWheelEvent};

use crate::ui::{
    div, px, rgb, AnyElement, AppearanceMode, InteractiveElement, IntoElement,
    NotionSearchPreviewState, ParentElement, StatefulInteractiveElement, Styled,
};

use super::{
    controls::notion_search_shadows, preview_style::notion_search_preview_body_fill, QuickFindView,
    SEARCH_DARK_BORDER_FILL, SEARCH_PREVIEW_CARD_HEIGHT_PX,
};

impl QuickFindView {
    pub(super) fn render_notion_search_preview(&self, cx: &mut App) -> AnyElement {
        let card = self.notion_search_preview_card();
        let Some(result) = self.state.visible_results().get(self.state.selected_index) else {
            return card.into_any_element();
        };
        if let NotionSearchPreviewState::Loaded { block_id, page } = &self.state.preview {
            if block_id.as_ref() == result.block_id {
                return card
                    .child(self.render_loaded_notion_search_preview(result.clone(), page))
                    .into_any_element();
            }
        }
        let body = match &self.state.preview {
            NotionSearchPreviewState::Failed { block_id }
                if block_id.as_ref() == result.block_id =>
            {
                div().size_full().into_any_element()
            }
            _ => self.render_notion_search_preview_skeleton(),
        };
        card.child(self.render_pending_notion_search_preview(result, body, cx))
            .into_any_element()
    }

    fn notion_search_preview_card(&self) -> crate::ui::Div {
        div()
            .debug_selector(|| "notion-search-preview".to_string())
            .w(px(378.0))
            .h(px(SEARCH_PREVIEW_CARD_HEIGHT_PX as f32))
            .rounded(px(12.0))
            .overflow_hidden()
            .bg(rgb(notion_search_preview_body_fill(self.appearance_mode)))
            .border_1()
            .border_color(rgb(match self.appearance_mode {
                AppearanceMode::Light => 0xe6e6e4,
                AppearanceMode::Dark => SEARCH_DARK_BORDER_FILL,
            }))
            .shadow(notion_search_shadows(self.appearance_mode))
            .relative()
            .flex()
            .flex_col()
    }

    fn render_pending_notion_search_preview(
        &self,
        result: &crate::model::PageShellSearchResult,
        body: AnyElement,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(format!("notion-search-preview-scroll-{}", result.block_id))
            .debug_selector(|| "notion-search-preview-scroll".to_string())
            .size_full()
            .relative()
            .overflow_y_scroll()
            .track_scroll(&self.state.preview_scroll_handle)
            .on_scroll_wheel(|_: &ScrollWheelEvent, _, cx| cx.stop_propagation())
            .child(self.render_notion_search_preview_header())
            .child(self.render_notion_search_preview_actions(cx))
            .child(self.render_notion_search_preview_identity(result, body, true, cx))
            .into_any_element()
    }

    fn render_loaded_notion_search_preview(
        &self,
        result: crate::model::PageShellSearchResult,
        page: &crate::ui::LoadedCardPage,
    ) -> AnyElement {
        let view = self.clone();
        let data = page.data.clone();
        let row_count = data.visible_rows.len();
        div()
            .id(format!("notion-search-preview-scroll-{}", result.block_id))
            .debug_selector(|| "notion-search-preview-scroll".to_string())
            .size_full()
            .relative()
            .overflow_hidden()
            .on_scroll_wheel(|_: &ScrollWheelEvent, _, cx| cx.stop_propagation())
            .child(
                list(
                    page.list_state.clone(),
                    move |list_row_index, _window, cx| {
                        if list_row_index == 0 {
                            return view.render_notion_search_preview_intro(
                                &result,
                                row_count == 0,
                                cx,
                            );
                        }
                        view.render_notion_search_preview_virtual_block(
                            list_row_index - 1,
                            row_count,
                            &data,
                        )
                    },
                )
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .size_full(),
            )
            .into_any_element()
    }
}
