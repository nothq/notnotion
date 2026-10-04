use std::sync::Arc;

use super::super::super::{PageLinkIconAction, PageLinkIconUploadAction};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Div, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, RenderImage, Role, StatefulInteractiveElement, Styled,
    StyledImage,
};

use crate::ui::board_workspace::{
    alpha, div, img,
    page::editor::{PageLinkIconPickerState, PageLinkIconView},
    px, rgb, rgba,
};

impl PageLinkIconView {
    pub(super) fn render_page_link_icon_upload_preview_panel(
        &self,
        state: &PageLinkIconPickerState,
    ) -> AnyElement {
        div()
            .id("notion-page-link-icon-upload")
            .flex_grow(1.0)
            .px(px(16.0))
            .pt(px(20.0))
            .pb(px(12.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .mb(px(4.0))
                    .text_size(px(12.0))
                    .text_color(rgb(self.theme.text_muted))
                    .child("Preview"),
            )
            .child(self.render_page_link_icon_upload_preview_pair(state))
            .when(state.custom_emoji_creation_allowed, |panel| {
                panel.child(self.render_page_link_icon_upload_library_toggle(state))
            })
            .when(state.upload_add_to_library, |panel| {
                panel.child(self.render_page_link_icon_upload_name(state))
            })
            .child(self.render_page_link_icon_upload_footer(state))
            .into_any_element()
    }

    fn render_page_link_icon_upload_preview_pair(&self, state: &PageLinkIconPickerState) -> Div {
        let Some(preview) = state.upload_preview.as_ref() else {
            return page_link_icon_pending_preview_pair(self.theme.text_primary);
        };
        div()
            .h(px(80.0))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .child(page_link_icon_preview_card(
                Arc::clone(&preview.rendered),
                0xffffff,
                self.theme.text_primary,
                state.upload_add_to_library,
            ))
            .child(page_link_icon_preview_card(
                Arc::clone(&preview.rendered),
                0x191919,
                self.theme.text_primary,
                state.upload_add_to_library,
            ))
    }

    fn render_page_link_icon_upload_library_toggle(
        &self,
        state: &PageLinkIconPickerState,
    ) -> AnyElement {
        let checked = state.upload_add_to_library;
        let disabled = state.upload_committed || state.custom_emoji_limit_reached();
        div()
            .id("notion-page-link-icon-upload-library")
            .role(Role::CheckBox)
            .aria_label(page_link_icon_upload_library_label(checked, disabled))
            .focusable()
            .tab_stop(true)
            .h(px(21.0))
            .mt(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .opacity(if disabled { 0.4 } else { 1.0 })
            .when(!disabled, |toggle| toggle.cursor_pointer())
            .text_size(px(14.0))
            .text_color(rgb(self.theme.text_primary))
            .on_mouse_down(
                MouseButton::Left,
                self.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.emit(
                        PageLinkIconAction::Upload(PageLinkIconUploadAction::ToggleUploadLibrary),
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
                    PageLinkIconAction::Upload(PageLinkIconUploadAction::ToggleUploadLibrary),
                    window,
                    cx,
                );
            }))
            .child(page_link_icon_upload_library_checkbox(
                checked,
                self.theme.text_primary,
            ))
            .child("Add to workspace emoji library")
            .into_any_element()
    }

    fn render_page_link_icon_upload_name(&self, state: &PageLinkIconPickerState) -> AnyElement {
        let name_field = div()
            .h(px(28.0))
            .w_full()
            .rounded(px(6.0))
            .bg(rgba(self.theme.dialog_input_bg))
            .px(px(10.0))
            .py(px(4.0))
            .flex()
            .items_center()
            .when(state.upload_committed, |field| {
                field
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(rgb(self.theme.text_primary))
                    .child(state.upload_name.clone())
            })
            .when(!state.upload_committed, |field| {
                field.child(state.upload_name_input.clone())
            });
        div()
            .mt(px(20.0))
            .child(
                div()
                    .mb(px(6.0))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_muted))
                    .child("Emoji name"),
            )
            .child(name_field)
            .into_any_element()
    }
}

fn page_link_icon_preview_card(
    rendered: Arc<RenderImage>,
    background: u32,
    text_primary: u32,
    add_to_library: bool,
) -> Div {
    let object_fit = if add_to_library {
        gpui::ObjectFit::Contain
    } else {
        gpui::ObjectFit::Cover
    };
    div()
        .size(px(80.0))
        .p(px(6.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(alpha(text_primary, 0.12))
        .bg(rgb(background))
        .child(img(rendered).size(px(66.0)).object_fit(object_fit))
}

fn page_link_icon_pending_preview_pair(text_primary: u32) -> Div {
    div()
        .h(px(80.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .child(
            div()
                .size(px(80.0))
                .rounded(px(8.0))
                .bg(alpha(text_primary, 0.055)),
        )
        .child(
            div()
                .size(px(80.0))
                .rounded(px(8.0))
                .bg(alpha(text_primary, 0.08)),
        )
}

fn page_link_icon_upload_library_label(checked: bool, disabled: bool) -> &'static str {
    if disabled {
        "Add to workspace emoji library, unavailable because the workspace limit is reached"
    } else if checked {
        "Add to workspace emoji library, checked"
    } else {
        "Add to workspace emoji library, unchecked"
    }
}

fn page_link_icon_upload_library_checkbox(checked: bool, text_primary: u32) -> Div {
    div()
        .size(px(14.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(alpha(text_primary, 0.35))
        .when(checked, |checkbox| {
            checkbox
                .bg(rgb(0x2383e2))
                .border_color(rgb(0x2383e2))
                .child(
                    div()
                        .w_full()
                        .text_align(gpui::TextAlign::Center)
                        .text_size(px(11.0))
                        .text_color(rgb(0xffffff))
                        .child("✓"),
                )
        })
}
