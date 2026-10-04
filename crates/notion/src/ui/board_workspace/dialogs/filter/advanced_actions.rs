use crate::ui::board_workspace::dialogs::filter::{
    controller::DatabaseFilterController, helpers::*, prelude::*,
};

impl DatabaseFilterController<'_> {
    pub(super) fn remove_active_filter(&mut self, host: DatabaseFilterHost) {
        if let Some(path) = self.state.draft_advanced_path.clone() {
            self.remove_advanced_node(path, host);
            return;
        }
        let filter_id = self
            .state
            .draft
            .as_ref()
            .expect("deleting a filter requires an active draft")
            .filter_id
            .clone();
        let current = self.state.effective_state(self.board);
        let next = match current.applying(&DatabaseViewFilterMutation::Simple(
            DatabaseSimpleFilterMutation::Remove(filter_id),
        )) {
            Ok(next) => next,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        self.state.temporary = Some(next.clone());
        self.state.draft = None;
        self.state.stage = DatabaseFilterDialogStage::PropertyPicker;
        self.dismiss_local(host);
        let revision = self.state.advance_revision();
        self.query_projection(next, revision);
    }

    pub(super) fn promote_active_filter(&mut self) {
        if self.state.draft_advanced_path.is_some() {
            self.state.stage = DatabaseFilterDialogStage::AdvancedEditor;
            return;
        }
        let draft = self
            .state
            .draft
            .clone()
            .expect("promoting a filter requires an active draft");
        let Some(simple_filter) = database_simple_filter_from_draft(&draft) else {
            return;
        };
        let current = self.state.effective_state(self.board);
        let without_simple = match current.applying(&DatabaseViewFilterMutation::Simple(
            DatabaseSimpleFilterMutation::Remove(draft.filter_id),
        )) {
            Ok(next) => next,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        let advanced_group = match current.advanced() {
            DatabaseAdvancedFilterState::None => DatabaseFilterGroup::new(
                DatabaseFilterGroupOperator::And,
                vec![DatabaseFilterNode::Property(simple_filter.filter().clone())],
            ),
            DatabaseAdvancedFilterState::Editable(group) => {
                let mut filters = group.filters().to_vec();
                filters.push(DatabaseFilterNode::Property(simple_filter.filter().clone()));
                DatabaseFilterGroup::new(group.operator(), filters)
            }
            DatabaseAdvancedFilterState::Unsupported => {
                self.report("Unsupported Notion advanced filters are not editable".to_string());
                return;
            }
        };
        let next = without_simple
            .applying(&DatabaseViewFilterMutation::Advanced(
                DatabaseAdvancedFilterMutation::Set(advanced_group),
            ))
            .expect("validated advanced filter promotion must apply");
        self.state.temporary = Some(next.clone());
        self.state.draft = None;
        self.state.stage = DatabaseFilterDialogStage::AdvancedEditor;
        let revision = self.state.advance_revision();
        self.query_projection(next, revision);
    }

    pub(super) fn open_advanced_rule(&mut self, path: Vec<usize>) {
        let current = self.state.effective_state(self.board);
        let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
            return;
        };
        let Some(DatabaseFilterNode::Property(filter)) =
            database_advanced_filter_node(group, &path)
        else {
            return;
        };
        let Some(property) = self
            .board
            .database_properties
            .iter()
            .find(|property| property.property_id == filter.property_id().as_str())
            .cloned()
        else {
            return;
        };
        let synthetic = DatabaseSimpleFilter::new(new_database_filter_id(), filter.clone());
        self.state.draft = Some(DatabaseFilterDraft::from_filter(property, &synthetic));
        self.state.draft_advanced_path = Some(path);
        self.state.stage = DatabaseFilterDialogStage::Editor;
        self.state.value_query.clear();
        self.state.value_input.borrow_mut().take();
        self.state.value_focus_requested.set(true);
        self.load_picker_values();
    }

    pub(super) fn insert_advanced_property(
        &mut self,
        property: DatabaseProperty,
        target: DatabaseFilterPropertyPickerTarget,
    ) {
        let Some(filter) = default_database_property_filter(&property) else {
            self.report(format!(
                "Notion property {} cannot be filtered",
                property.label
            ));
            return;
        };
        let current = self.state.effective_state(self.board);
        let Some((next_group, node_path)) =
            database_advanced_filter_insertion(&current, target, &filter)
        else {
            return;
        };
        self.apply_advanced_mutation(DatabaseAdvancedFilterMutation::Set(next_group));
        let synthetic = DatabaseSimpleFilter::new(new_database_filter_id(), filter);
        self.state.draft = Some(DatabaseFilterDraft::from_filter(property, &synthetic));
        self.state.draft_advanced_path = Some(node_path);
        self.state.property_picker_target = DatabaseFilterPropertyPickerTarget::Simple;
        self.state.stage = DatabaseFilterDialogStage::Editor;
        self.state.value_query.clear();
        self.state.value_input.borrow_mut().take();
        self.state.value_focus_requested.set(true);
        self.load_picker_values();
    }

    pub(super) fn set_advanced_operator(&mut self, operator: DatabaseFilterGroupOperator) {
        self.mutate_advanced_group(move |group| {
            DatabaseFilterGroup::new(operator, group.filters().to_vec())
        });
    }

    pub(super) fn toggle_advanced_group(&mut self, path: Vec<usize>) {
        self.mutate_advanced_group(move |group| {
            database_advanced_filter_replacing_group_operator(group, &path)
                .unwrap_or_else(|| group.clone())
        });
    }

    pub(super) fn remove_advanced_node(&mut self, path: Vec<usize>, host: DatabaseFilterHost) {
        let current = self.state.effective_state(self.board);
        let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
            return;
        };
        let Some(next_group) = database_advanced_filter_removing_node(group, &path) else {
            return;
        };
        let cleared = next_group.filters().is_empty();
        let mutation = if cleared {
            DatabaseAdvancedFilterMutation::Clear
        } else {
            DatabaseAdvancedFilterMutation::Set(next_group)
        };
        self.apply_advanced_mutation(mutation);
        self.state.draft = None;
        self.state.draft_advanced_path = None;
        if cleared {
            self.state.stage = DatabaseFilterDialogStage::PropertyPicker;
            self.dismiss_local(host);
        } else {
            self.state.stage = DatabaseFilterDialogStage::AdvancedEditor;
        }
    }

    pub(super) fn clear_advanced_filter(&mut self, host: DatabaseFilterHost) {
        self.apply_advanced_mutation(DatabaseAdvancedFilterMutation::Clear);
        self.state.draft = None;
        self.state.draft_advanced_path = None;
        self.dismiss_local(host);
    }

    fn mutate_advanced_group(
        &mut self,
        mutate: impl FnOnce(&DatabaseFilterGroup) -> DatabaseFilterGroup,
    ) {
        let current = self.state.effective_state(self.board);
        let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
            return;
        };
        self.apply_advanced_mutation(DatabaseAdvancedFilterMutation::Set(mutate(group)));
        self.state.stage = DatabaseFilterDialogStage::AdvancedEditor;
    }

    pub(super) fn apply_advanced_mutation(&mut self, mutation: DatabaseAdvancedFilterMutation) {
        let current = self.state.effective_state(self.board);
        let next = match current.applying(&DatabaseViewFilterMutation::Advanced(mutation)) {
            Ok(next) => next,
            Err(error) => {
                self.report(error);
                return;
            }
        };
        self.state.temporary = Some(next.clone());
        let revision = self.state.advance_revision();
        self.query_projection(next, revision);
    }

    pub(super) fn open_advanced_property_picker(&mut self, node_path: Vec<usize>) {
        self.state.open_advanced_property_picker(
            DatabaseFilterPropertyPickerTarget::ReplaceAdvancedRule { node_path },
        );
    }

    pub(super) fn add_advanced_rule(&mut self) {
        let group_path = self.state.advanced_add_parent_path.clone();
        self.state.open_advanced_property_picker(
            DatabaseFilterPropertyPickerTarget::AppendAdvancedRule { group_path },
        );
    }

    pub(super) fn add_advanced_group(&mut self) {
        let group_path = self.state.advanced_add_parent_path.clone();
        self.state.open_advanced_property_picker(
            DatabaseFilterPropertyPickerTarget::AppendAdvancedGroup { group_path },
        );
    }
}

/// The filter group after an insertion, and the path of the inserted node.
type AdvancedFilterInsertion = (DatabaseFilterGroup, Vec<usize>);

fn database_advanced_filter_insertion(
    current: &DatabaseViewFilterState,
    target: DatabaseFilterPropertyPickerTarget,
    filter: &DatabasePropertyFilter,
) -> Option<AdvancedFilterInsertion> {
    match target {
        DatabaseFilterPropertyPickerTarget::Simple => None,
        DatabaseFilterPropertyPickerTarget::StartAdvanced => Some((
            DatabaseFilterGroup::new(
                DatabaseFilterGroupOperator::And,
                vec![DatabaseFilterNode::Property(filter.clone())],
            ),
            vec![0],
        )),
        DatabaseFilterPropertyPickerTarget::AppendAdvancedRule { group_path } => {
            let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
                return None;
            };
            database_advanced_filter_append_rule(group, group_path, filter)
        }
        DatabaseFilterPropertyPickerTarget::AppendAdvancedGroup { group_path } => {
            let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
                return None;
            };
            database_advanced_filter_append_group(group, group_path, filter)
        }
        DatabaseFilterPropertyPickerTarget::ReplaceAdvancedRule { node_path } => {
            let DatabaseAdvancedFilterState::Editable(group) = current.advanced() else {
                return None;
            };
            let next_group = database_advanced_filter_replacing_node(
                group,
                &node_path,
                DatabaseFilterNode::Property(filter.clone()),
            )?;
            Some((next_group, node_path))
        }
    }
}

fn database_advanced_filter_append_rule(
    group: &DatabaseFilterGroup,
    group_path: Vec<usize>,
    filter: &DatabasePropertyFilter,
) -> Option<AdvancedFilterInsertion> {
    let parent_group = database_advanced_filter_group(group, &group_path)?;
    let mut node_path = group_path.clone();
    node_path.push(parent_group.filters().len());
    let next_group = database_advanced_filter_appending_node(
        group,
        &group_path,
        DatabaseFilterNode::Property(filter.clone()),
    )?;
    Some((next_group, node_path))
}

fn database_advanced_filter_append_group(
    group: &DatabaseFilterGroup,
    group_path: Vec<usize>,
    filter: &DatabasePropertyFilter,
) -> Option<AdvancedFilterInsertion> {
    let parent_group = database_advanced_filter_group(group, &group_path)?;
    let mut node_path = group_path.clone();
    node_path.push(parent_group.filters().len());
    let child = DatabaseFilterGroup::new(
        DatabaseFilterGroupOperator::And,
        vec![DatabaseFilterNode::Property(filter.clone())],
    );
    let next_group = database_advanced_filter_appending_node(
        group,
        &group_path,
        DatabaseFilterNode::Group(child),
    )?;
    node_path.push(0);
    Some((next_group, node_path))
}
