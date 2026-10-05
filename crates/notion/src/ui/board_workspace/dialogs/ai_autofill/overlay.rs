use super::{
    ai_autofill_shadow, alpha, div, img, px, rgb, AiAutofillAction, AiAutofillLayout,
    AiAutofillView, AnyElement, App, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Role, StatefulInteractiveElement,
    Styled, MAIN_HEIGHT, MAIN_WIDTH,
};
use gpui_components::backdrop::dismissible_backdrop_with_handler;

impl AiAutofillView<'_> {
    pub(super) fn render(&self, cx: &mut App) -> AnyElement {
        let layout = AiAutofillLayout::new(
            self.state,
            self.viewport.app_width(),
            self.viewport.app_height(),
        );
        div()
            .id("notion-ai-autofill-overlay")
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(px(self.viewport.app_width()))
            .h(px(self.viewport.app_height()))
            .child(self.render_ai_autofill_click_away())
            .child(self.render_ai_autofill_main_dialog(layout, cx))
            .when(self.state.property_picker_open, |overlay| {
                overlay.child(self.render_ai_autofill_property_menu(layout, cx))
            })
            .when_some(
                self.state.selected_property_index,
                |overlay, property_index| {
                    overlay.child(self.render_ai_autofill_mode_menu(property_index, layout, cx))
                },
            )
            .when(self.state.trial_open, |overlay| {
                overlay.child(self.render_ai_autofill_trial_modal(cx))
            })
            .into_any_element()
    }

    fn render_ai_autofill_click_away(&self) -> AnyElement {
        let actions = self.actions.clone();
        dismissible_backdrop_with_handler(
            div().absolute().inset_0().bg(alpha(0x000000, 0.001)),
            move |_, window, cx| {
                actions.emit(AiAutofillAction::Dismiss, window, cx);
            },
        )
        .into_any_element()
    }

    fn render_ai_autofill_main_dialog(&self, layout: AiAutofillLayout, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-autofill-dialog")
            .absolute()
            .left(px(layout.main_left))
            .top(px(layout.main_top))
            .w(px(MAIN_WIDTH))
            .h(px(MAIN_HEIGHT))
            .overflow_hidden()
            .rounded(px(10.0))
            .bg(rgb(self.theme.elevated_surface_bg))
            .shadow(ai_autofill_shadow())
            .role(Role::Dialog)
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .flex()
            .flex_col()
            .child(self.render_ai_autofill_header(cx))
            .child(self.render_ai_autofill_empty_state(cx))
            .child(self.render_ai_autofill_footer(cx))
            .into_any_element()
    }

    fn render_ai_autofill_header(&self, cx: &mut App) -> AnyElement {
        div()
            .h(px(42.0))
            .flex_none()
            .pt(px(14.0))
            .pb(px(6.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .child(
                div()
                    .flex_grow(1.0)
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(self.theme.text_primary))
                    .child("AI Autofill"),
            )
            .child(self.render_ai_autofill_feedback(cx))
            .child(self.render_ai_autofill_close_button(cx))
            .into_any_element()
    }

    fn render_ai_autofill_feedback(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-autofill-feedback")
            .h(px(20.0))
            .mr(px(8.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .cursor_pointer()
            .role(Role::Button)
            .aria_label("Send AI Autofill feedback")
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child(
                div()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_muted))
                    .child("Feedback"),
            )
            .child(img(self.icons.ai_autofill_external_link.render(cx)).size(px(14.0)))
            .into_any_element()
    }

    fn render_ai_autofill_close_button(&self, cx: &mut App) -> AnyElement {
        div()
            .id("notion-ai-autofill-close")
            .size(px(20.0))
            .rounded_full()
            .bg(alpha(self.theme.text_primary, 0.08))
            .role(Role::Button)
            .aria_label("Close AI Autofill")
            .cursor_pointer()
            .hover(|style| style.bg(alpha(self.theme.text_primary, 0.14)))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    AiAutofillAction::Dismiss
                }),
            )
            .child(img(self.icons.close.render(cx)).size(px(14.0)))
            .into_any_element()
    }

    fn render_ai_autofill_empty_state(&self, cx: &mut App) -> AnyElement {
        div()
            .h(px(258.0))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .child(img(self.icons.ai_autofill_magic_wand.render(cx)).size(px(24.0)))
            .child(
                div()
                    .text_size(px(17.0))
                    .line_height(px(22.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(self.theme.text_primary))
                    .child("AI Autofill"),
            )
            .child(self.render_ai_autofill_description())
            .into_any_element()
    }

    fn render_ai_autofill_description(&self) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .text_size(px(14.0))
            .line_height(px(19.6))
            .text_color(rgb(self.theme.text_muted))
            .child("Automatically fills properties with")
            .child("context from your workspace and the")
            .child("web.")
            .child(
                div()
                    .id("notion-ai-autofill-learn-more")
                    .cursor_pointer()
                    .role(Role::Button)
                    .aria_label("Learn more about AI Autofill")
                    .underline()
                    .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                        cx.stop_propagation()
                    })
                    .child("Learn more."),
            )
            .into_any_element()
    }

    fn render_ai_autofill_footer(&self, cx: &mut App) -> AnyElement {
        div()
            .h(px(52.0))
            .flex_none()
            .p(px(12.0))
            .child(
                div()
                    .id("notion-ai-autofill-new")
                    .w_full()
                    .h(px(28.0))
                    .rounded(px(6.0))
                    .bg(rgb(0x2783de))
                    .role(Role::Button)
                    .aria_label("New AI Autofill")
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .on_mouse_down(
                        MouseButton::Left,
                        self.actions.listener(|_: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            AiAutofillAction::OpenPropertyPicker
                        }),
                    )
                    .child(img(self.icons.ai_autofill_plus.render(cx)).size(px(16.0)))
                    .child(self.render_ai_autofill_footer_label()),
            )
            .into_any_element()
    }

    fn render_ai_autofill_footer_label(&self) -> Div {
        div()
            .text_size(px(14.0))
            .line_height(px(20.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xf3f9fd))
            .child("New AI Autofill")
    }
}
