use super::super::super::{PageLinkIconAction, PageLinkIconPickerAction, PageLinkIconUploadAction};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, ClickEvent, Div, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Role, Stateful, StatefulInteractiveElement, Styled,
};

use crate::{
    model::CreateCustomEmojiPageIconRequest,
    ui::board_workspace::{
        alpha, div,
        page::editor::{PageLinkIconPickerState, PageLinkIconView},
        px, rgb,
    },
};

#[derive(Clone, Copy)]
struct PageLinkIconUploadFooterState {
    preview_visible: bool,
    secondary_enabled: bool,
    save_enabled: bool,
    secondary_label: &'static str,
}

impl PageLinkIconView {
    pub(super) fn render_page_link_icon_upload_footer(
        &self,
        state: &PageLinkIconPickerState,
    ) -> AnyElement {
        let footer = PageLinkIconUploadFooterState::new(state, self.page_mutation_idle);
        div()
            .mt_auto()
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_between()
            .child(self.render_page_link_icon_upload_secondary(footer))
            .child(self.render_page_link_icon_upload_save(footer.save_enabled))
            .into_any_element()
    }

    fn render_page_link_icon_upload_secondary(
        &self,
        state: PageLinkIconUploadFooterState,
    ) -> Stateful<Div> {
        div()
            .id("notion-page-link-icon-upload-secondary")
            .role(Role::Button)
            .aria_label(state.secondary_label)
            .focusable()
            .tab_stop(state.secondary_enabled)
            .h(px(28.0))
            .px(px(12.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .opacity(if state.secondary_enabled { 1.0 } else { 0.4 })
            .when(state.secondary_enabled, |button| button.cursor_pointer())
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.08)))
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_muted))
            .on_click(self.page_link_icon_upload_secondary_click(state.preview_visible))
            .on_key_down(self.page_link_icon_upload_secondary_key_down(state.preview_visible))
            .child(state.secondary_label)
    }

    fn page_link_icon_upload_secondary_click(
        &self,
        preview_visible: bool,
    ) -> impl Fn(&ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            let action = if preview_visible {
                PageLinkIconAction::Upload(PageLinkIconUploadAction::ClearUploadPreview)
            } else {
                PageLinkIconAction::Picker(PageLinkIconPickerAction::Dismiss)
            };
            this.emit(action, window, cx);
        })
    }

    fn page_link_icon_upload_secondary_key_down(
        &self,
        preview_visible: bool,
    ) -> impl Fn(&KeyDownEvent, &mut gpui::Window, &mut gpui::App) + 'static {
        self.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(event.keystroke.key.as_str(), "enter" | "space")
            {
                return;
            }
            window.prevent_default();
            cx.stop_propagation();
            let action = if preview_visible {
                PageLinkIconAction::Upload(PageLinkIconUploadAction::ClearUploadPreview)
            } else {
                PageLinkIconAction::Picker(PageLinkIconPickerAction::Dismiss)
            };
            this.emit(action, window, cx);
        })
    }

    fn render_page_link_icon_upload_save(&self, save_enabled: bool) -> Stateful<Div> {
        div()
            .id("notion-page-link-icon-upload-save")
            .role(Role::Button)
            .aria_label("Save")
            .h(px(28.0))
            .px(px(12.0))
            .rounded(px(6.0))
            .bg(rgb(0x2383e2))
            .opacity(if save_enabled { 1.0 } else { 0.4 })
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.0))
            .text_color(rgb(0xffffff))
            .when(save_enabled, |button| {
                button
                    .focusable()
                    .tab_stop(true)
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0x1f75cc)))
                    .on_click(self.listener(|this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.emit(
                            PageLinkIconAction::Upload(PageLinkIconUploadAction::SaveUpload),
                            window,
                            cx,
                        );
                    }))
                    .on_key_down(self.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.modifiers.modified()
                            || !matches!(event.keystroke.key.as_str(), "enter" | "space")
                        {
                            return;
                        }
                        window.prevent_default();
                        cx.stop_propagation();
                        this.emit(
                            PageLinkIconAction::Upload(PageLinkIconUploadAction::SaveUpload),
                            window,
                            cx,
                        );
                    }))
            })
            .child("Save")
    }
}

impl PageLinkIconUploadFooterState {
    fn new(state: &PageLinkIconPickerState, queue_idle: bool) -> Self {
        let preview_visible = state.upload_preview.is_some() || state.upload_pending;
        let save_enabled = state.upload_preview.is_some()
            && !state.upload_pending
            && queue_idle
            && (!state.upload_add_to_library
                || (CreateCustomEmojiPageIconRequest::name_is_valid(&state.upload_name)
                    && !state.custom_emoji_name_is_taken(&state.upload_name)));
        PageLinkIconUploadFooterState {
            preview_visible,
            secondary_enabled: !state.upload_committed,
            save_enabled,
            secondary_label: if preview_visible { "Back" } else { "Cancel" },
        }
    }
}
