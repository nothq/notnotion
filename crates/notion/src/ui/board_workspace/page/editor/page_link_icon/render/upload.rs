mod footer;
mod preview;

use gpui::{
    AnyElement, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Role, StatefulInteractiveElement, Styled,
};

use super::super::{PageLinkIconAction, PageLinkIconUploadAction};
use crate::ui::board_workspace::{
    alpha, div,
    page::editor::{PageLinkIconPickerState, PageLinkIconView},
    px, rgb,
};

impl PageLinkIconView {
    pub(super) fn render_page_link_icon_upload_tab(
        &self,
        state: &PageLinkIconPickerState,
    ) -> AnyElement {
        if state.upload_preview.is_some() || state.upload_pending {
            return self.render_page_link_icon_upload_preview_panel(state);
        }
        div()
            .id("notion-page-link-icon-upload")
            .flex_grow(1.0)
            .px(px(16.0))
            .pt(px(20.0))
            .pb(px(12.0))
            .flex()
            .flex_col()
            .child(self.render_page_link_icon_upload_button())
            .child(
                div()
                    .mt(px(10.0))
                    .h(px(18.0))
                    .text_size(px(12.0))
                    .line_height(px(18.0))
                    .text_align(gpui::TextAlign::Center)
                    .text_color(rgb(self.theme.text_muted))
                    .child("or ⌘+V to paste an image or link"),
            )
            .child(self.render_page_link_icon_upload_footer(state))
            .into_any_element()
    }

    fn render_page_link_icon_upload_button(&self) -> AnyElement {
        div()
            .id("notion-page-link-icon-upload-button")
            .role(Role::Button)
            .aria_label("Upload an image")
            .focusable()
            .tab_stop(true)
            .w_full()
            .h(px(48.0))
            .rounded(px(6.0))
            .bg(alpha(self.theme.text_primary, 0.055))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_muted))
            .on_mouse_down(
                MouseButton::Left,
                self.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.emit(
                        PageLinkIconAction::Upload(PageLinkIconUploadAction::PromptUpload),
                        window,
                        cx,
                    );
                }),
            )
            .on_key_down(self.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.modifiers.modified()
                    || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                this.emit(
                    PageLinkIconAction::Upload(PageLinkIconUploadAction::PromptUpload),
                    window,
                    cx,
                );
            }))
            .child(div().text_size(px(17.0)).child("▧"))
            .child("Upload an image")
            .into_any_element()
    }
}
