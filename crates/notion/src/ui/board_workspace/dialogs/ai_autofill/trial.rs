use super::trial_icons::{timeline_reminder_icon, timeline_today_icon, timeline_upgrade_icon};
use super::{
    ai_autofill_modal_shadow, alpha, div, px, rgb, AiAutofillAction, AiAutofillTimelineKind,
    AiAutofillView, AnyElement, App, AppearanceMode, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Role, StatefulInteractiveElement,
    Styled, Toggled,
};
use gpui_components::backdrop::blocking_backdrop;

impl AiAutofillView<'_> {
    pub(super) fn render_ai_autofill_trial_modal(&self, cx: &mut App) -> AnyElement {
        let modal_background = match self.appearance_mode {
            AppearanceMode::Light => 0xffffff,
            AppearanceMode::Dark => 0x202020,
        };
        div()
            .id("notion-ai-autofill-trial-layer")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(blocking_backdrop(
                div().absolute().inset_0().bg(alpha(0x000000, 0.64)),
                cx,
            ))
            .child(
                div()
                    .id("notion-ai-autofill-trial")
                    .w(px(860.0))
                    .h(px(577.0))
                    .overflow_hidden()
                    .rounded(px(12.0))
                    .bg(rgb(modal_background))
                    .shadow(ai_autofill_modal_shadow())
                    .role(Role::Dialog)
                    .p(px(24.0))
                    .flex()
                    .gap(px(32.0))
                    .child(self.render_ai_autofill_trial_details())
                    .child(self.render_ai_autofill_trial_marketing()),
            )
            .into_any_element()
    }

    fn render_ai_autofill_trial_details(&self) -> AnyElement {
        let consent_checked = self.state.trial_consent_checked;
        div()
            .w(px(300.0))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .child(self.render_ai_autofill_trial_heading())
            .child(self.render_ai_autofill_trial_timeline())
            .child(
                div()
                    .mt(px(22.0))
                    .h(px(81.0))
                    .flex_none()
                    .border_t_1()
                    .border_color(alpha(self.theme.text_primary, 0.12))
                    .pt(px(16.0))
                    .child(self.render_ai_autofill_trial_consent(consent_checked)),
            )
            .child(self.render_ai_autofill_trial_actions(consent_checked))
            .into_any_element()
    }

    fn render_ai_autofill_trial_heading(&self) -> AnyElement {
        div()
            .h(px(64.0))
            .flex_none()
            .flex()
            .flex_col()
            .text_size(px(24.0))
            .line_height(px(30.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(self.theme.text_primary))
            .child("Use 30 days of free")
            .child("Notion AI and more!")
            .into_any_element()
    }

    fn render_ai_autofill_trial_timeline(&self) -> AnyElement {
        div()
            .mt(px(30.0))
            .h(px(244.0))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(self.render_ai_autofill_trial_timeline_row(
                AiAutofillTimelineKind::Today,
                "Today",
                "Full access to Notion AI and the best features in our Business Plan",
                84.0,
            ))
            .child(self.render_ai_autofill_trial_timeline_row(
                AiAutofillTimelineKind::Reminder,
                "Day 25",
                "We’ll remind you of your trial ending",
                64.0,
            ))
            .child(self.render_ai_autofill_trial_timeline_row(
                AiAutofillTimelineKind::Upgrade,
                "Day 30",
                "Your subscription will automatically upgrade to Business",
                84.0,
            ))
            .into_any_element()
    }

    fn render_ai_autofill_trial_actions(&self, consent_checked: bool) -> AnyElement {
        div()
            .mt(px(16.0))
            .h(px(72.0))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(self.render_ai_autofill_start_trial_button(consent_checked))
            .child(self.render_ai_autofill_trial_later_button())
            .into_any_element()
    }

    fn render_ai_autofill_start_trial_button(&self, consent_checked: bool) -> AnyElement {
        div()
            .id("notion-ai-autofill-start-trial")
            .w_full()
            .h(px(32.0))
            .rounded(px(6.0))
            .bg(rgb(0x2783de))
            .opacity(if consent_checked { 1.0 } else { 0.4 })
            .role(Role::Button)
            .aria_label("Start free trial")
            .when(consent_checked, |button| button.cursor_pointer())
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xf3f9fd))
            .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                cx.stop_propagation()
            })
            .child("Start free trial")
            .into_any_element()
    }

    fn render_ai_autofill_trial_later_button(&self) -> AnyElement {
        div()
            .id("notion-ai-autofill-trial-later")
            .w_full()
            .h(px(32.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.14))
            .bg(rgb(self.theme.elevated_surface_bg))
            .role(Role::Button)
            .aria_label("Maybe later")
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(self.theme.text_primary))
            .on_mouse_down(
                MouseButton::Left,
                self.actions.listener(|_: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    AiAutofillAction::CloseTrial
                }),
            )
            .child("Maybe later")
            .into_any_element()
    }

    fn render_ai_autofill_trial_timeline_row(
        &self,
        kind: AiAutofillTimelineKind,
        day: &'static str,
        description: &'static str,
        height: f32,
    ) -> AnyElement {
        div()
            .h(px(height))
            .flex_none()
            .flex()
            .gap(px(4.0))
            .child(
                div()
                    .w(px(32.0))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(self.render_ai_autofill_trial_timeline_icon(kind))
                    .when(!matches!(kind, AiAutofillTimelineKind::Upgrade), |column| {
                        column.child(
                            div()
                                .mt(px(3.0))
                                .w(px(1.0))
                                .flex_grow(1.0)
                                .bg(alpha(0x5e9fe8, 0.28)),
                        )
                    }),
            )
            .child(self.render_ai_autofill_trial_timeline_copy(kind, day, description))
            .into_any_element()
    }

    fn render_ai_autofill_trial_timeline_copy(
        &self,
        kind: AiAutofillTimelineKind,
        day: &'static str,
        description: &'static str,
    ) -> AnyElement {
        let active = matches!(kind, AiAutofillTimelineKind::Today);
        let color = rgb(if active {
            self.theme.text_primary
        } else {
            self.theme.text_muted
        });
        div()
            .pt(px(5.0))
            .flex_grow(1.0)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color)
                    .child(day),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .line_height(px(20.0))
                    .text_color(color)
                    .child(description),
            )
            .into_any_element()
    }

    fn render_ai_autofill_trial_timeline_icon(&self, kind: AiAutofillTimelineKind) -> AnyElement {
        match kind {
            AiAutofillTimelineKind::Today => timeline_today_icon(),
            AiAutofillTimelineKind::Reminder => timeline_reminder_icon(),
            AiAutofillTimelineKind::Upgrade => timeline_upgrade_icon(),
        }
    }

    fn render_ai_autofill_trial_consent(&self, checked: bool) -> AnyElement {
        div()
            .id("notion-ai-autofill-trial-consent")
            .w_full()
            .h(px(64.0))
            .role(Role::CheckBox)
            .aria_label("Accept Notion Business trial renewal terms")
            .aria_toggled(if checked {
                Toggled::True
            } else {
                Toggled::False
            })
            .cursor_pointer()
            .flex()
            .items_start()
            .gap(px(8.0))
            .on_mouse_down(
                MouseButton::Left,
                self.actions
                    .listener(|_: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        AiAutofillAction::ToggleTrialConsent
                    }),
            )
            .child(self.render_ai_autofill_trial_consent_box(checked))
            .child(
                div()
                    .flex_grow(1.0)
                    .text_size(px(12.0))
                    .line_height(px(15.0))
                    .text_color(rgb(self.theme.text_primary))
                    .child("At the end of your trial, Notion Business will auto-renew each month at $24/seat plus taxes unless canceled. Cancel via the Billing tab prior to renewal to remain on the Plus plan."),
            )
            .into_any_element()
    }

    fn render_ai_autofill_trial_consent_box(&self, checked: bool) -> AnyElement {
        div()
            .mt(px(2.0))
            .size(px(14.0))
            .flex_none()
            .rounded(px(3.0))
            .border_1()
            .border_color(alpha(self.theme.text_primary, 0.36))
            .when(checked, |box_element| {
                box_element
                    .bg(rgb(0x2783de))
                    .border_color(rgb(0x2783de))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.0))
                    .text_color(rgb(0xffffff))
                    .child("✓")
            })
            .into_any_element()
    }
}
