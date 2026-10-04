use std::sync::Arc;

use super::{div, px, AnyElement, FluentBuilder, IntoElement, ParentElement, Styled};
use crate::ui::board_workspace::page::editor::PageBlockRenderer;
use crate::ui::{LoadedCardPageData, PageBlockDragScrollTarget};
use gpui::{App, InteractiveElement};

pub(super) enum SelectedPageBodyRenderer {
    Empty,
    Loaded(Box<SelectedPageLoadedBodyRenderer>),
}

pub(super) struct SelectedPageLoadedBodyRenderer {
    blocks: PageBlockRenderer,
    data: Arc<LoadedCardPageData>,
    document: AnyElement,
    drop_overlay: Option<AnyElement>,
}

impl SelectedPageBodyRenderer {
    pub(super) fn empty() -> Self {
        Self::Empty
    }

    pub(super) fn loaded(
        blocks: PageBlockRenderer,
        data: Arc<LoadedCardPageData>,
        document: AnyElement,
        drop_overlay: Option<AnyElement>,
    ) -> Self {
        Self::Loaded(Box::new(SelectedPageLoadedBodyRenderer {
            blocks,
            data,
            document,
            drop_overlay,
        }))
    }

    pub(super) fn render(self, cx: &mut App) -> AnyElement {
        let Self::Loaded(loaded) = self else {
            return div().size_full().into_any_element();
        };
        let SelectedPageLoadedBodyRenderer {
            blocks,
            data,
            document,
            drop_overlay,
        } = *loaded;
        let container = blocks
            .observe_page_block_drag_container_bounds(
                div(),
                PageBlockDragScrollTarget::SelectedPage,
            )
            .id("notion-selected-page-scroll")
            .debug_selector(|| "notion-selected-page-scroll".to_string())
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .relative()
            .flex()
            .flex_col()
            .overflow_hidden();
        let scroll = blocks
            .wire_page_block_drag_container(
                container,
                data,
                PageBlockDragScrollTarget::SelectedPage,
                cx,
            )
            .child(div().absolute().inset_0())
            .child(document)
            .when_some(drop_overlay, |page, overlay| page.child(overlay));
        div().size_full().flex().child(scroll).into_any_element()
    }
}
