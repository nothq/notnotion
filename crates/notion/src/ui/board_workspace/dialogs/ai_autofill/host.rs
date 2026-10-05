use super::{AiAutofillAction, AiAutofillView};
use crate::ui::surface::NotionChromeState;
use crate::ui::view_actions::ViewActionSink;
use crate::ui::{AnyElement, Context, KeyDownEvent, SurfaceState};

impl SurfaceState {
    pub(crate) fn render_ai_autofill_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self
            .notion_chrome
            .ai_autofill_dialog
            .as_ref()
            .expect("AI Autofill overlay requires dialog state");
        let actions = ViewActionSink::new(cx, |surface, action, _, cx| {
            surface.notion_chrome.apply_ai_autofill_action(action);
            cx.notify();
        });
        AiAutofillView {
            state,
            theme: &self.theme,
            icons: &self.icons,
            appearance_mode: self.appearance_mode,
            viewport: &self.viewport,
            actions,
        }
        .render(cx)
    }
}

impl NotionChromeState {
    pub(crate) fn handle_ai_autofill_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<SurfaceState>,
    ) -> bool {
        if event.keystroke.key != "escape" || self.ai_autofill_dialog.is_none() {
            return false;
        }
        self.apply_ai_autofill_action(AiAutofillAction::Escape);
        cx.notify();
        true
    }

    fn apply_ai_autofill_action(&mut self, action: AiAutofillAction) {
        match action {
            AiAutofillAction::Dismiss => self.ai_autofill_dialog = None,
            AiAutofillAction::OpenPropertyPicker => self.open_ai_autofill_property_picker(),
            AiAutofillAction::SelectProperty(property_index) => {
                self.select_ai_autofill_property(property_index)
            }
            AiAutofillAction::OpenTrial => self.open_ai_autofill_trial(),
            AiAutofillAction::CloseTrial => self.close_ai_autofill_trial(),
            AiAutofillAction::ToggleTrialConsent => self.toggle_ai_autofill_trial_consent(),
            AiAutofillAction::Escape => self.escape_ai_autofill(),
        }
    }

    fn open_ai_autofill_property_picker(&mut self) {
        let state = self
            .ai_autofill_dialog
            .as_mut()
            .expect("AI Autofill property picker requires dialog state");
        state.property_picker_open = true;
        state.selected_property_index = None;
    }

    fn select_ai_autofill_property(&mut self, property_index: usize) {
        let state = self
            .ai_autofill_dialog
            .as_mut()
            .expect("AI Autofill property selection requires dialog state");
        state
            .properties
            .get(property_index)
            .expect("AI Autofill property selection must reference a visible property");
        state.selected_property_index = Some(property_index);
    }

    fn open_ai_autofill_trial(&mut self) {
        self.ai_autofill_dialog
            .as_mut()
            .expect("AI Autofill trial requires dialog state")
            .trial_open = true;
    }

    fn close_ai_autofill_trial(&mut self) {
        self.ai_autofill_dialog
            .as_mut()
            .expect("AI Autofill trial close requires dialog state")
            .trial_open = false;
    }

    fn toggle_ai_autofill_trial_consent(&mut self) {
        let state = self
            .ai_autofill_dialog
            .as_mut()
            .expect("AI Autofill trial consent requires dialog state");
        state.trial_consent_checked = !state.trial_consent_checked;
    }

    fn escape_ai_autofill(&mut self) {
        let state = self
            .ai_autofill_dialog
            .as_mut()
            .expect("AI Autofill escape requires dialog state");
        if state.trial_open {
            state.trial_open = false;
        } else if state.selected_property_index.take().is_none() {
            if state.property_picker_open {
                state.property_picker_open = false;
            } else {
                self.ai_autofill_dialog = None;
            }
        }
    }
}
