use crate::ui::board_workspace::dialogs::filter::{
    controller::DatabaseFilterController, helpers::*, prelude::*,
};

mod relation_search;
pub(super) use relation_search::execute_database_filter_relation_search;

impl DatabaseFilterController<'_> {
    pub(super) fn move_property_highlight(&mut self, direction: isize) {
        let properties = self
            .state
            .filtered_properties(&self.board.database_properties);
        if properties.is_empty() {
            return;
        }
        self.state.property_highlighted_index = move_wrapped_index(
            self.state.property_highlighted_index,
            direction,
            properties.len(),
        );
    }

    pub(super) fn activate_property_highlight(&mut self) {
        let properties = self
            .state
            .filtered_properties(&self.board.database_properties);
        let Some(property) = properties
            .get(self.state.property_highlighted_index)
            .filter(|property| database_filter_property_type(property.filter_type()))
            .cloned()
        else {
            return;
        };
        self.begin_filter(property);
    }

    pub(super) fn begin_filter(&mut self, property: DatabaseProperty) {
        let property_picker_target = self.state.property_picker_target.clone();
        if !matches!(
            property_picker_target,
            DatabaseFilterPropertyPickerTarget::Simple
        ) {
            self.insert_advanced_property(property, property_picker_target);
            return;
        }
        let apply_default = property.filter_type() == "date";
        self.state.draft = Some(DatabaseFilterDraft::new(property));
        self.state.draft_advanced_path = None;
        self.state.stage = DatabaseFilterDialogStage::Editor;
        self.state.value_query.clear();
        self.state.value_input.borrow_mut().take();
        self.state.value_focus_requested.set(true);
        self.load_picker_values();
        if apply_default {
            self.apply_draft();
        }
    }

    pub(super) fn begin_advanced_filter(&mut self) {
        let current = self.state.effective_state(self.board);
        match current.advanced() {
            DatabaseAdvancedFilterState::None => {
                if !self
                    .board
                    .database_properties
                    .iter()
                    .any(|property| default_database_property_filter(property).is_some())
                {
                    self.report("This Notion database has no filterable properties".to_string());
                    return;
                }
                self.state.open_advanced_property_picker(
                    DatabaseFilterPropertyPickerTarget::StartAdvanced,
                );
            }
            DatabaseAdvancedFilterState::Editable(_) => {
                self.state.stage = DatabaseFilterDialogStage::AdvancedEditor;
                self.state.draft = None;
                self.state.draft_advanced_path = None;
            }
            DatabaseAdvancedFilterState::Unsupported => {
                self.report("Unsupported Notion advanced filters are not editable".to_string());
            }
        }
    }

    pub(super) fn set_text_operator(&mut self, operator: DatabaseTextFilterOperator) {
        self.state
            .draft
            .as_mut()
            .expect("setting an operator requires a filter draft")
            .operator = operator;
        self.state.stage = DatabaseFilterDialogStage::Editor;
        self.state.value_input.borrow_mut().take();
        self.state
            .value_focus_requested
            .set(operator.requires_value());
        self.apply_draft();
    }

    pub(super) fn set_filter_value(&mut self, value: String) {
        let uses_text_input = self
            .state
            .draft
            .as_ref()
            .expect("setting a filter value requires a draft")
            .operator
            .uses_text_input();
        if uses_text_input {
            self.state
                .draft
                .as_mut()
                .expect("setting a filter value requires a draft")
                .value = value;
            self.apply_draft();
            return;
        }
        self.state.value_query = value;
        if self
            .state
            .draft
            .as_ref()
            .is_some_and(|draft| draft.property.filter_type() == "relation")
        {
            self.effects
                .push(DatabaseFilterEffect::ScheduleRelationSearch);
        }
    }

    pub(super) fn toggle_selection(&mut self, value: String) {
        let draft = self
            .state
            .draft
            .as_mut()
            .expect("selecting a filter value requires a draft");
        if draft.property.filter_type() == "checkbox" {
            let Some(checked) = database_checkbox_filter_value(&value) else {
                return;
            };
            draft.checkbox_value = Some(checked);
            self.apply_draft();
            return;
        }
        if let Some(index) = draft
            .selected_values
            .iter()
            .position(|selected| selected == &value)
        {
            draft.selected_values.remove(index);
        } else {
            draft.selected_values.push(value);
        }
        self.apply_draft();
    }

    pub(super) fn open_operator_picker(&mut self) {
        let draft = self
            .state
            .draft
            .as_ref()
            .expect("operator picker requires a filter draft");
        let operators = DatabaseTextFilterOperator::for_property(draft.property.filter_type());
        self.state.operator_highlighted_index = operators
            .iter()
            .position(|candidate| *candidate == draft.operator)
            .expect("active operator must be in the property picker");
        self.state.stage = DatabaseFilterDialogStage::OperatorPicker;
    }

    pub(super) fn move_operator_highlight(&mut self, direction: isize) {
        let operator_count = self
            .state
            .draft
            .as_ref()
            .map(|draft| {
                DatabaseTextFilterOperator::for_property(draft.property.filter_type()).len()
            })
            .expect("moving the operator highlight requires a filter draft");
        self.state.operator_highlighted_index = move_wrapped_index(
            self.state.operator_highlighted_index,
            direction,
            operator_count,
        );
    }
}

impl DatabaseFilterUiState {
    pub(super) fn filtered_properties(
        &self,
        database_properties: &[DatabaseProperty],
    ) -> Vec<DatabaseProperty> {
        let query = self.property_query.trim().to_lowercase();
        database_properties
            .iter()
            .filter(|property| query.is_empty() || property.label.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }
}
