use gpui::{AnyElement, App, IntoElement};

use super::{PageLinkIconPickerState, PageLinkIconView};

/// A picker snapshot with only its rendering resources and typed icon actions.
#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageLinkIconPresentation {
    pub(super) view: PageLinkIconView,
    pub(super) state: PageLinkIconPickerState,
}

impl PageLinkIconPresentation {
    pub(in crate::ui::board_workspace::page::editor) fn render(
        &self,
        page_id: &str,
        block_id: &str,
        cx: &mut App,
    ) -> AnyElement {
        if self.state.page_id != page_id || self.state.block_id != block_id {
            return crate::ui::div().into_any_element();
        }
        self.view
            .render_page_link_icon_picker(&self.state, page_id, block_id, cx)
    }
}
