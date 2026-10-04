use crate::model::{BoardSnapshot, DatabaseDateFilterMode, NotionDatabaseFilterId};
use crate::ui::surface::{DatabaseFilterDialogStage, DatabaseFilterUiState};
use chrono::Datelike;

use super::types::{
    DatabaseDateValueChoice, DatabaseFilterAction, DatabaseFilterAnchorKind, DatabaseFilterEffect,
    DatabaseFilterHost, DatabaseFilterOutsideAction,
};

pub(super) struct DatabaseFilterController<'a> {
    pub(super) state: &'a mut DatabaseFilterUiState,
    pub(super) board: &'a BoardSnapshot,
    pub(super) effects: Vec<DatabaseFilterEffect>,
}

impl<'a> DatabaseFilterController<'a> {
    pub(super) fn new(state: &'a mut DatabaseFilterUiState, board: &'a BoardSnapshot) -> Self {
        Self {
            state,
            board,
            effects: Vec::new(),
        }
    }

    pub(super) fn apply(mut self, action: DatabaseFilterAction) -> Vec<DatabaseFilterEffect> {
        match action {
            DatabaseFilterAction::Reset => self.reset_temporary(),
            DatabaseFilterAction::ShowPropertyPicker(host) => self.show_property_picker(host),
            DatabaseFilterAction::ShowAdvanced(host) => self.show_advanced(host),
            DatabaseFilterAction::OpenChip { chip, host } => self.open_filter_chip(*chip, host),
            DatabaseFilterAction::Save(host) => self.effects.push(DatabaseFilterEffect::Save(host)),
            DatabaseFilterAction::Dismiss(host) => self.dismiss(host),
            DatabaseFilterAction::Outside { action, host } => self.handle_outside(action, host),
            DatabaseFilterAction::SetStage(stage) => self.state.stage = stage,
            DatabaseFilterAction::SetAdvancedAddPath(path) => self.set_advanced_add_path(path),
            DatabaseFilterAction::SetPropertyQuery(query) => self.set_property_query(query),
            DatabaseFilterAction::MovePropertyHighlight(direction) => {
                self.move_property_highlight(direction)
            }
            DatabaseFilterAction::ActivatePropertyHighlight => self.activate_property_highlight(),
            DatabaseFilterAction::BeginFilter(property) => self.begin_filter(*property),
            DatabaseFilterAction::BeginAdvanced => self.begin_advanced_filter(),
            DatabaseFilterAction::SetValue(value) => self.set_filter_value(value),
            DatabaseFilterAction::ToggleSelection(value) => self.toggle_selection(value),
            DatabaseFilterAction::SetTextOperator(operator) => self.set_text_operator(operator),
            DatabaseFilterAction::OpenOperatorPicker => self.open_operator_picker(),
            DatabaseFilterAction::MoveOperatorHighlight(direction) => {
                self.move_operator_highlight(direction)
            }
            DatabaseFilterAction::CloseValueEditor(host) => self.close_value_editor(host),
            DatabaseFilterAction::SetDate(date) => self.apply_date(date),
            DatabaseFilterAction::SetDateMode(mode) => self.apply_date_mode(mode),
            DatabaseFilterAction::SetDateValueChoice(choice) => self.apply_date_choice(choice),
            DatabaseFilterAction::MoveCalendarMonth(direction) => {
                self.state.move_calendar_month(direction)
            }
            DatabaseFilterAction::CycleRelativeDirection => self.apply_relative_direction(),
            DatabaseFilterAction::CycleRelativeUnit => self.apply_relative_unit(),
            DatabaseFilterAction::AdjustRelativeCount(direction) => {
                self.apply_relative_count(direction)
            }
            DatabaseFilterAction::RemoveActive(host) => self.remove_active_filter(host),
            DatabaseFilterAction::PromoteActive => self.promote_active_filter(),
            DatabaseFilterAction::OpenAdvancedRule(path) => self.open_advanced_rule(path),
            DatabaseFilterAction::OpenAdvancedPropertyPicker(path) => {
                self.open_advanced_property_picker(path)
            }
            DatabaseFilterAction::AddAdvancedRule => self.add_advanced_rule(),
            DatabaseFilterAction::AddAdvancedGroup => self.add_advanced_group(),
            DatabaseFilterAction::SetAdvancedOperator(operator) => {
                self.set_advanced_operator(operator)
            }
            DatabaseFilterAction::ToggleAdvancedGroup(path) => self.toggle_advanced_group(path),
            DatabaseFilterAction::RemoveAdvancedNode { path, host } => {
                self.remove_advanced_node(path, host)
            }
            DatabaseFilterAction::ClearAdvanced(host) => self.clear_advanced_filter(host),
            DatabaseFilterAction::MeasureAnchor { kind, bounds } => self.set_anchor(kind, bounds),
            DatabaseFilterAction::MeasureEditorAnchor { filter_id, bounds } => {
                self.set_editor_anchor(filter_id, bounds)
            }
        }
        self.effects
    }

    fn show_property_picker(&mut self, host: DatabaseFilterHost) {
        self.state.open_property_picker();
        self.show(host);
    }

    fn show_advanced(&mut self, host: DatabaseFilterHost) {
        self.state.stage = DatabaseFilterDialogStage::AdvancedEditor;
        self.show(host);
    }

    fn set_advanced_add_path(&mut self, path: Vec<usize>) {
        self.state.advanced_add_parent_path = path;
        self.state.stage = DatabaseFilterDialogStage::AdvancedAddMenu;
    }

    fn set_property_query(&mut self, query: String) {
        self.state.property_query = query;
        self.state.property_highlighted_index = 0;
    }

    fn apply_date(&mut self, date: chrono::NaiveDate) {
        self.state.set_date(date);
        self.apply_draft();
    }

    fn apply_date_mode(&mut self, mode: DatabaseDateFilterMode) {
        self.state.set_date_mode(mode);
        self.apply_draft();
    }

    fn apply_date_choice(&mut self, choice: DatabaseDateValueChoice) {
        self.state.set_date_value_choice(choice);
        self.apply_draft();
    }

    fn apply_relative_direction(&mut self) {
        self.state.cycle_relative_direction();
        self.apply_draft();
    }

    fn apply_relative_unit(&mut self) {
        self.state.cycle_relative_unit();
        self.apply_draft();
    }

    fn apply_relative_count(&mut self, direction: i32) {
        self.state.adjust_relative_count(direction);
        self.apply_draft();
    }

    fn set_editor_anchor(
        &mut self,
        filter_id: Option<NotionDatabaseFilterId>,
        bounds: gpui::Bounds<gpui::Pixels>,
    ) {
        let active_filter_id = self.state.draft.as_ref().map(|draft| &draft.filter_id);
        if active_filter_id == filter_id.as_ref() {
            self.state.editor_anchor = Some(bounds);
        }
    }

    pub(super) fn query_projection(
        &mut self,
        filter_state: crate::model::DatabaseViewFilterState,
        revision: u64,
    ) {
        self.effects.push(DatabaseFilterEffect::QueryProjection {
            filter_state,
            revision,
        });
    }

    pub(super) fn load_picker_values(&mut self) {
        let Some(property) = self
            .state
            .draft
            .as_ref()
            .map(|draft| draft.property.clone())
        else {
            return;
        };
        match property.filter_type() {
            "person" if self.state.users.is_none() && !self.state.users_in_flight => {
                self.effects.push(DatabaseFilterEffect::LoadUsers);
            }
            "relation" => self
                .effects
                .push(DatabaseFilterEffect::ScheduleRelationSearch),
            "date" => self.set_initial_visible_date(),
            _ => {}
        }
    }

    pub(super) fn report(&mut self, error: String) {
        self.effects.push(DatabaseFilterEffect::Report(error));
    }

    pub(super) fn dismiss(&mut self, host: DatabaseFilterHost) {
        self.effects.push(DatabaseFilterEffect::Dismiss {
            host,
            close_local: false,
        });
    }

    pub(super) fn dismiss_local(&mut self, host: DatabaseFilterHost) {
        self.effects.push(DatabaseFilterEffect::Dismiss {
            host,
            close_local: true,
        });
    }

    fn show(&mut self, host: DatabaseFilterHost) {
        self.effects.push(DatabaseFilterEffect::Show(host));
    }

    fn handle_outside(&mut self, action: DatabaseFilterOutsideAction, host: DatabaseFilterHost) {
        match action {
            DatabaseFilterOutsideAction::CloseToolbar => self.dismiss(host),
            DatabaseFilterOutsideAction::ReturnToEditor => {
                self.state.stage = DatabaseFilterDialogStage::Editor
            }
            DatabaseFilterOutsideAction::ReturnToAdvancedEditor => {
                self.state.stage = DatabaseFilterDialogStage::AdvancedEditor
            }
            DatabaseFilterOutsideAction::ReturnToEditorAndFocusValue => {
                self.state.stage = DatabaseFilterDialogStage::Editor;
                self.state.value_focus_requested.set(true);
            }
        }
    }

    fn close_value_editor(&mut self, host: DatabaseFilterHost) {
        if self.state.stage == DatabaseFilterDialogStage::OperatorPicker {
            self.state.stage = DatabaseFilterDialogStage::Editor;
            self.state.value_focus_requested.set(true);
        } else {
            self.dismiss(host);
        }
    }

    fn set_anchor(&mut self, kind: DatabaseFilterAnchorKind, bounds: gpui::Bounds<gpui::Pixels>) {
        match kind {
            DatabaseFilterAnchorKind::Operator => self.state.operator_anchor = Some(bounds),
            DatabaseFilterAnchorKind::DateMode => self.state.date_mode_anchor = Some(bounds),
            DatabaseFilterAnchorKind::DateValue => self.state.date_value_anchor = Some(bounds),
            DatabaseFilterAnchorKind::Actions => self.state.actions_anchor = Some(bounds),
        }
    }

    fn set_initial_visible_date(&mut self) {
        let date = self.state.draft.as_ref().and_then(initial_filter_date);
        if let Some(date) = date {
            self.state.visible_date_month = date
                .with_day(1)
                .expect("the first day exists in every Gregorian month");
        }
    }
}

fn initial_filter_date(
    draft: &crate::ui::surface::DatabaseFilterDraft,
) -> Option<chrono::NaiveDate> {
    use crate::model::{DatabaseDatePoint, DatabaseDateRange};
    use crate::ui::surface::DatabaseTextFilterOperator;
    if draft.operator == DatabaseTextFilterOperator::DateIsBetween {
        return match &draft.date_range {
            DatabaseDateRange::Exact {
                start_date,
                end_date,
            } => (*start_date).or(*end_date),
            DatabaseDateRange::Relative { .. } | DatabaseDateRange::Surrounding { .. } => None,
        };
    }
    match &draft.date_point {
        DatabaseDatePoint::Exact(date) => Some(*date),
        DatabaseDatePoint::Relative(_) => None,
    }
}
