use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

use chrono::{Datelike, Local};
use gpui::{Bounds, Entity, Pixels};
use gpui_components::text_input::TextInput;

use crate::model::{
    DatabaseDateFilterMode, DatabaseDatePoint, DatabaseDateRange, DatabaseFilterRelationPage,
    DatabaseFilterUser, DatabasePersonFilter, DatabaseProperty, DatabasePropertyFilterCondition,
    DatabaseRelativeDateUnit, DatabaseSimpleFilter, DatabaseStatusFilterValue, DatabaseTextFilter,
    DatabaseViewFilterState, NotionCollectionViewId, NotionDatabaseFilterId,
};
use crate::ui::BoardSnapshot;

pub(crate) const CURRENT_USER_FILTER_VALUE: &str = "__notion_current_user__";
pub(crate) const STATUS_OPTION_FILTER_VALUE_PREFIX: &str = "option:";
pub(crate) const STATUS_GROUP_FILTER_VALUE_PREFIX: &str = "group:";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum DatabaseFilterDialogStage {
    #[default]
    PropertyPicker,
    Editor,
    OperatorPicker,
    DateModePicker,
    DateValuePicker,
    MoreActions,
    AdvancedEditor,
    AdvancedAddMenu,
    AdvancedCombinerPicker,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum DatabaseFilterPropertyPickerTarget {
    #[default]
    Simple,
    StartAdvanced,
    AppendAdvancedRule {
        group_path: Vec<usize>,
    },
    AppendAdvancedGroup {
        group_path: Vec<usize>,
    },
    ReplaceAdvancedRule {
        node_path: Vec<usize>,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum DatabaseTextFilterOperator {
    TextIs,
    TextIsNot,
    #[default]
    TextContains,
    TextDoesNotContain,
    TextStartsWith,
    TextEndsWith,
    NumberEquals,
    NumberDoesNotEqual,
    NumberGreaterThan,
    NumberGreaterThanOrEqualTo,
    NumberLessThan,
    NumberLessThanOrEqualTo,
    CheckboxIs,
    CheckboxIsNot,
    Contains,
    DoesNotContain,
    DateIs,
    DateIsBefore,
    DateIsAfter,
    DateIsOnOrBefore,
    DateIsOnOrAfter,
    DateIsBetween,
    DateIsRelativeToToday,
    SelectIs,
    SelectIsNot,
    SelectContains,
    SelectDoesNotContain,
    StatusIs,
    StatusIsNot,
    IsEmpty,
    IsNotEmpty,
}

impl DatabaseTextFilterOperator {
    const TEXT: [Self; 8] = [
        Self::TextIs,
        Self::TextIsNot,
        Self::TextContains,
        Self::TextDoesNotContain,
        Self::TextStartsWith,
        Self::TextEndsWith,
        Self::IsEmpty,
        Self::IsNotEmpty,
    ];
    const CONTAINS: [Self; 4] = [
        Self::Contains,
        Self::DoesNotContain,
        Self::IsEmpty,
        Self::IsNotEmpty,
    ];
    const NUMBER: [Self; 8] = [
        Self::NumberEquals,
        Self::NumberDoesNotEqual,
        Self::NumberGreaterThan,
        Self::NumberGreaterThanOrEqualTo,
        Self::NumberLessThan,
        Self::NumberLessThanOrEqualTo,
        Self::IsEmpty,
        Self::IsNotEmpty,
    ];
    const CHECKBOX: [Self; 2] = [Self::CheckboxIs, Self::CheckboxIsNot];
    const DATE: [Self; 9] = [
        Self::DateIs,
        Self::DateIsBefore,
        Self::DateIsAfter,
        Self::DateIsOnOrBefore,
        Self::DateIsOnOrAfter,
        Self::DateIsBetween,
        Self::DateIsRelativeToToday,
        Self::IsEmpty,
        Self::IsNotEmpty,
    ];
    const SELECT: [Self; 4] = [
        Self::SelectIs,
        Self::SelectIsNot,
        Self::IsEmpty,
        Self::IsNotEmpty,
    ];
    const MULTI_SELECT: [Self; 4] = [
        Self::SelectContains,
        Self::SelectDoesNotContain,
        Self::IsEmpty,
        Self::IsNotEmpty,
    ];
    const STATUS: [Self; 2] = [Self::StatusIs, Self::StatusIsNot];

    pub(crate) fn for_property(property_type: &str) -> &'static [Self] {
        match property_type {
            "number" => &Self::NUMBER,
            "checkbox" => &Self::CHECKBOX,
            "person" | "relation" => &Self::CONTAINS,
            "date" => &Self::DATE,
            "select" => &Self::SELECT,
            "multi_select" => &Self::MULTI_SELECT,
            "status" => &Self::STATUS,
            _ => &Self::TEXT,
        }
    }

    pub(crate) fn default_for_property(property_type: &str) -> Self {
        match property_type {
            "number" => Self::NumberEquals,
            "checkbox" => Self::CheckboxIs,
            "person" | "relation" => Self::Contains,
            "date" => Self::DateIsRelativeToToday,
            "select" => Self::SelectIs,
            "multi_select" => Self::SelectContains,
            "status" => Self::StatusIs,
            _ => Self::TextContains,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::TextIs | Self::DateIs | Self::SelectIs | Self::StatusIs => "Is",
            Self::TextIsNot | Self::SelectIsNot | Self::StatusIsNot => "Is not",
            Self::TextContains | Self::Contains | Self::SelectContains => "Contains",
            Self::TextDoesNotContain | Self::DoesNotContain | Self::SelectDoesNotContain => {
                "Does not contain"
            }
            Self::TextStartsWith => "Starts with",
            Self::TextEndsWith => "Ends with",
            Self::NumberEquals => "=",
            Self::NumberDoesNotEqual => "≠",
            Self::NumberGreaterThan => ">",
            Self::NumberGreaterThanOrEqualTo => "≥",
            Self::NumberLessThan => "<",
            Self::NumberLessThanOrEqualTo => "≤",
            Self::CheckboxIs => "Is",
            Self::CheckboxIsNot => "Is not",
            Self::DateIsBefore => "Is before",
            Self::DateIsAfter => "Is after",
            Self::DateIsOnOrBefore => "Is on or before",
            Self::DateIsOnOrAfter => "Is on or after",
            Self::DateIsBetween => "Is between",
            Self::DateIsRelativeToToday => "Is relative to today",
            Self::IsEmpty => "Is empty",
            Self::IsNotEmpty => "Is not empty",
        }
    }

    pub(crate) const fn header_label(self) -> &'static str {
        match self {
            Self::TextIs | Self::DateIs | Self::SelectIs | Self::StatusIs => "is",
            Self::TextIsNot | Self::SelectIsNot | Self::StatusIsNot => "is not",
            Self::TextContains | Self::Contains | Self::SelectContains => "contains",
            Self::TextDoesNotContain | Self::DoesNotContain | Self::SelectDoesNotContain => {
                "does not contain"
            }
            Self::TextStartsWith => "starts with",
            Self::TextEndsWith => "ends with",
            Self::NumberEquals => "=",
            Self::NumberDoesNotEqual => "≠",
            Self::NumberGreaterThan => ">",
            Self::NumberGreaterThanOrEqualTo => "≥",
            Self::NumberLessThan => "<",
            Self::NumberLessThanOrEqualTo => "≤",
            Self::CheckboxIs => "is",
            Self::CheckboxIsNot => "is not",
            Self::DateIsBefore => "is before",
            Self::DateIsAfter => "is after",
            Self::DateIsOnOrBefore => "is on or before",
            Self::DateIsOnOrAfter => "is on or after",
            Self::DateIsBetween => "is between",
            Self::DateIsRelativeToToday => "is relative to today",
            Self::IsEmpty => "is empty",
            Self::IsNotEmpty => "is not empty",
        }
    }

    pub(crate) const fn requires_value(self) -> bool {
        !matches!(self, Self::IsEmpty | Self::IsNotEmpty)
    }

    pub(crate) const fn uses_text_input(self) -> bool {
        matches!(
            self,
            Self::TextIs
                | Self::TextIsNot
                | Self::TextContains
                | Self::TextDoesNotContain
                | Self::TextStartsWith
                | Self::TextEndsWith
                | Self::NumberEquals
                | Self::NumberDoesNotEqual
                | Self::NumberGreaterThan
                | Self::NumberGreaterThanOrEqualTo
                | Self::NumberLessThan
                | Self::NumberLessThanOrEqualTo
        )
    }

    pub(crate) fn from_condition(condition: &DatabasePropertyFilterCondition) -> Self {
        match condition {
            DatabasePropertyFilterCondition::Text(filter) => match filter {
                DatabaseTextFilter::Is(_) => Self::TextIs,
                DatabaseTextFilter::IsNot(_) => Self::TextIsNot,
                DatabaseTextFilter::Contains(_) => Self::TextContains,
                DatabaseTextFilter::DoesNotContain(_) => Self::TextDoesNotContain,
                DatabaseTextFilter::StartsWith(_) => Self::TextStartsWith,
                DatabaseTextFilter::EndsWith(_) => Self::TextEndsWith,
                DatabaseTextFilter::IsEmpty => Self::IsEmpty,
                DatabaseTextFilter::IsNotEmpty => Self::IsNotEmpty,
            },
            DatabasePropertyFilterCondition::NumberEquals(_) => Self::NumberEquals,
            DatabasePropertyFilterCondition::NumberDoesNotEqual(_) => Self::NumberDoesNotEqual,
            DatabasePropertyFilterCondition::NumberGreaterThan(_) => Self::NumberGreaterThan,
            DatabasePropertyFilterCondition::NumberGreaterThanOrEqualTo(_) => {
                Self::NumberGreaterThanOrEqualTo
            }
            DatabasePropertyFilterCondition::NumberLessThan(_) => Self::NumberLessThan,
            DatabasePropertyFilterCondition::NumberLessThanOrEqualTo(_) => {
                Self::NumberLessThanOrEqualTo
            }
            DatabasePropertyFilterCondition::NumberIsEmpty => Self::IsEmpty,
            DatabasePropertyFilterCondition::NumberIsNotEmpty => Self::IsNotEmpty,
            DatabasePropertyFilterCondition::CheckboxIs(_) => Self::CheckboxIs,
            DatabasePropertyFilterCondition::CheckboxIsNot(_) => Self::CheckboxIsNot,
            DatabasePropertyFilterCondition::PersonContains(_)
            | DatabasePropertyFilterCondition::RelationContains(_) => Self::Contains,
            DatabasePropertyFilterCondition::PersonDoesNotContain(_)
            | DatabasePropertyFilterCondition::RelationDoesNotContain(_) => Self::DoesNotContain,
            DatabasePropertyFilterCondition::PersonIsEmpty
            | DatabasePropertyFilterCondition::RelationIsEmpty
            | DatabasePropertyFilterCondition::DateIsEmpty
            | DatabasePropertyFilterCondition::SelectIsEmpty => Self::IsEmpty,
            DatabasePropertyFilterCondition::PersonIsNotEmpty
            | DatabasePropertyFilterCondition::RelationIsNotEmpty
            | DatabasePropertyFilterCondition::DateIsNotEmpty
            | DatabasePropertyFilterCondition::SelectIsNotEmpty => Self::IsNotEmpty,
            DatabasePropertyFilterCondition::DateIs(_) => Self::DateIs,
            DatabasePropertyFilterCondition::DateIsBefore(_) => Self::DateIsBefore,
            DatabasePropertyFilterCondition::DateIsAfter(_) => Self::DateIsAfter,
            DatabasePropertyFilterCondition::DateIsOnOrBefore(_) => Self::DateIsOnOrBefore,
            DatabasePropertyFilterCondition::DateIsOnOrAfter(_) => Self::DateIsOnOrAfter,
            DatabasePropertyFilterCondition::DateIsWithin(_) => Self::DateIsBetween,
            DatabasePropertyFilterCondition::DateIsRelativeTo(_) => Self::DateIsRelativeToToday,
            DatabasePropertyFilterCondition::SelectIs(_) => Self::SelectIs,
            DatabasePropertyFilterCondition::SelectIsNot(_) => Self::SelectIsNot,
            DatabasePropertyFilterCondition::SelectContains(_) => Self::SelectContains,
            DatabasePropertyFilterCondition::SelectDoesNotContain(_) => Self::SelectDoesNotContain,
            DatabasePropertyFilterCondition::StatusIs(_) => Self::StatusIs,
            DatabasePropertyFilterCondition::StatusIsNot(_) => Self::StatusIsNot,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DatabaseFilterDraft {
    pub(crate) filter_id: NotionDatabaseFilterId,
    pub(crate) property: DatabaseProperty,
    pub(crate) operator: DatabaseTextFilterOperator,
    pub(crate) value: String,
    pub(crate) checkbox_value: Option<bool>,
    pub(crate) selected_values: Vec<String>,
    pub(crate) date_point: DatabaseDatePoint,
    pub(crate) date_range: DatabaseDateRange,
    pub(crate) date_mode: DatabaseDateFilterMode,
    pub(crate) present_in_effective_state: bool,
}

mod draft;
mod ui_state;

pub(crate) use ui_state::DatabaseFilterUiState;
