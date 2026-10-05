use crate::ui::board_workspace::dialogs::filter::prelude::*;
use crate::ui::board_workspace::dialogs::filter::{
    controller::DatabaseFilterController, execute_database_filter_effect,
};

impl SurfaceState {
    pub(crate) fn handle_database_filter_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.database_filter.stage != DatabaseFilterDialogStage::OperatorPicker
            || event.keystroke.modifiers.modified()
        {
            return false;
        }
        let action = match event.keystroke.key.as_str() {
            "up" => DatabaseFilterAction::MoveOperatorHighlight(-1),
            "down" => DatabaseFilterAction::MoveOperatorHighlight(1),
            "enter" | "space" => {
                let draft = self
                    .database_filter
                    .draft
                    .as_ref()
                    .expect("operator picker requires a filter draft");
                let operator =
                    DatabaseTextFilterOperator::for_property(draft.property.filter_type())
                        [self.database_filter.operator_highlighted_index];
                DatabaseFilterAction::SetTextOperator(operator)
            }
            "escape" => DatabaseFilterAction::Outside {
                action: super::types::DatabaseFilterOutsideAction::ReturnToEditorAndFocusValue,
                host: DatabaseFilterHost::FullPage,
            },
            _ => return false,
        };
        let effects =
            DatabaseFilterController::new(&mut self.database_filter, &self.board).apply(action);
        for effect in effects {
            execute_database_filter_effect(self, effect, cx);
        }
        cx.notify();
        true
    }
}
