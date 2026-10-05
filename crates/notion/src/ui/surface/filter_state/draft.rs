use super::{
    DatabaseDateFilterMode, DatabaseDatePoint, DatabaseDateRange, DatabaseFilterDraft,
    DatabasePersonFilter, DatabaseProperty, DatabasePropertyFilterCondition,
    DatabaseRelativeDateUnit, DatabaseSimpleFilter, DatabaseStatusFilterValue, DatabaseTextFilter,
    DatabaseTextFilterOperator, Local, NotionDatabaseFilterId, CURRENT_USER_FILTER_VALUE,
    STATUS_GROUP_FILTER_VALUE_PREFIX, STATUS_OPTION_FILTER_VALUE_PREFIX,
};

impl DatabaseFilterDraft {
    pub(crate) fn new(property: DatabaseProperty) -> Self {
        let operator = DatabaseTextFilterOperator::default_for_property(property.filter_type());
        Self {
            filter_id: NotionDatabaseFilterId::try_from(uuid::Uuid::new_v4().to_string())
                .expect("generated Notion filter IDs are non-empty"),
            property,
            operator,
            value: String::new(),
            checkbox_value: None,
            selected_values: Vec::new(),
            date_point: DatabaseDatePoint::Exact(Local::now().date_naive()),
            date_range: DatabaseDateRange::Surrounding {
                unit: DatabaseRelativeDateUnit::Week,
            },
            date_mode: DatabaseDateFilterMode::StartDate,
            present_in_effective_state: false,
        }
    }

    pub(crate) fn from_filter(property: DatabaseProperty, filter: &DatabaseSimpleFilter) -> Self {
        let condition = filter.filter().condition();
        let mut draft = Self::new(property);
        draft.filter_id = filter.filter_id().clone();
        draft.operator = DatabaseTextFilterOperator::from_condition(condition);
        draft.present_in_effective_state = true;
        draft.apply_condition(condition);
        draft
    }

    fn apply_condition(&mut self, condition: &DatabasePropertyFilterCondition) {
        if self.apply_scalar_condition(condition)
            || self.apply_people_condition(condition)
            || self.apply_selection_condition(condition)
        {
            return;
        }
        self.apply_date_condition(condition);
    }

    fn apply_scalar_condition(&mut self, condition: &DatabasePropertyFilterCondition) -> bool {
        match condition {
            DatabasePropertyFilterCondition::Text(filter) => {
                self.value = text_filter_value(filter);
            }
            DatabasePropertyFilterCondition::NumberEquals(value)
            | DatabasePropertyFilterCondition::NumberDoesNotEqual(value)
            | DatabasePropertyFilterCondition::NumberGreaterThan(value)
            | DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(value)
            | DatabasePropertyFilterCondition::NumberLessThan(value)
            | DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(value) => {
                self.value = value.to_string();
            }
            DatabasePropertyFilterCondition::CheckboxIs(checked)
            | DatabasePropertyFilterCondition::CheckboxIsNot(checked) => {
                self.checkbox_value = Some(*checked);
            }
            _ => return false,
        }
        true
    }

    fn apply_people_condition(&mut self, condition: &DatabasePropertyFilterCondition) -> bool {
        match condition {
            DatabasePropertyFilterCondition::PersonContains(value)
            | DatabasePropertyFilterCondition::PersonDoesNotContain(value) => {
                self.selected_values = person_filter_values(value);
            }
            DatabasePropertyFilterCondition::RelationContains(values)
            | DatabasePropertyFilterCondition::RelationDoesNotContain(values) => {
                self.selected_values = values
                    .as_slice()
                    .iter()
                    .map(|page_id| page_id.as_str().to_string())
                    .collect();
            }
            _ => return false,
        }
        true
    }

    fn apply_selection_condition(&mut self, condition: &DatabasePropertyFilterCondition) -> bool {
        match condition {
            DatabasePropertyFilterCondition::SelectIs(values)
            | DatabasePropertyFilterCondition::SelectIsNot(values)
            | DatabasePropertyFilterCondition::SelectContains(values)
            | DatabasePropertyFilterCondition::SelectDoesNotContain(values) => {
                self.selected_values = values.as_slice().to_vec();
            }
            DatabasePropertyFilterCondition::StatusIs(values)
            | DatabasePropertyFilterCondition::StatusIsNot(values) => {
                self.selected_values = status_filter_values(values.as_slice());
            }
            _ => return false,
        }
        true
    }

    fn apply_date_condition(&mut self, condition: &DatabasePropertyFilterCondition) {
        match condition {
            DatabasePropertyFilterCondition::DateIs(value)
            | DatabasePropertyFilterCondition::DateIsBefore(value)
            | DatabasePropertyFilterCondition::DateIsAfter(value)
            | DatabasePropertyFilterCondition::DateIsOnOrBefore(value)
            | DatabasePropertyFilterCondition::DateIsOnOrAfter(value) => {
                self.date_point = value.value().clone();
                self.date_mode = value.mode();
            }
            DatabasePropertyFilterCondition::DateIsWithin(value)
            | DatabasePropertyFilterCondition::DateIsRelativeTo(value) => {
                self.date_range = value.value().clone();
                self.date_mode = value.mode();
            }
            _ => {}
        }
    }
}

fn text_filter_value(filter: &DatabaseTextFilter) -> String {
    match filter {
        DatabaseTextFilter::Is(value)
        | DatabaseTextFilter::IsNot(value)
        | DatabaseTextFilter::Contains(value)
        | DatabaseTextFilter::DoesNotContain(value)
        | DatabaseTextFilter::StartsWith(value)
        | DatabaseTextFilter::EndsWith(value) => value.as_str().to_string(),
        DatabaseTextFilter::IsEmpty | DatabaseTextFilter::IsNotEmpty => String::new(),
    }
}

fn person_filter_values(filter: &DatabasePersonFilter) -> Vec<String> {
    match filter {
        DatabasePersonFilter::CurrentUser => vec![CURRENT_USER_FILTER_VALUE.to_string()],
        DatabasePersonFilter::Users(users) => users
            .as_slice()
            .iter()
            .map(|user_id| user_id.as_str().to_string())
            .collect(),
        DatabasePersonFilter::CurrentUserAndUsers(users) => {
            let mut values = vec![CURRENT_USER_FILTER_VALUE.to_string()];
            values.extend(
                users
                    .as_slice()
                    .iter()
                    .map(|user_id| user_id.as_str().to_string()),
            );
            values
        }
    }
}

fn status_filter_values(values: &[DatabaseStatusFilterValue]) -> Vec<String> {
    values
        .iter()
        .map(|value| match value {
            DatabaseStatusFilterValue::Option(value) => {
                format!("{STATUS_OPTION_FILTER_VALUE_PREFIX}{value}")
            }
            DatabaseStatusFilterValue::Group(value) => {
                format!("{STATUS_GROUP_FILTER_VALUE_PREFIX}{value}")
            }
        })
        .collect()
}
