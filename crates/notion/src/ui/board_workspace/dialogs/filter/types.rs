use gpui::{Bounds, Pixels};

use crate::model::{
    DatabaseDateFilterMode, DatabaseFilterGroupOperator, DatabaseFilterNode, DatabaseProperty,
    DatabaseSimpleFilter, DatabaseViewFilterState, NotionDatabaseFilterId, PageShellIcon,
};
use crate::ui::surface::{DatabaseFilterDialogStage, DatabaseTextFilterOperator};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ui::board_workspace::dialogs) enum DatabaseFilterHost {
    FullPage,
    Inline,
}

#[derive(Clone)]
pub(super) enum DatabaseFilterAction {
    Reset,
    ShowPropertyPicker(DatabaseFilterHost),
    ShowAdvanced(DatabaseFilterHost),
    OpenChip {
        chip: Box<DatabaseFilterChip>,
        host: DatabaseFilterHost,
    },
    Save(DatabaseFilterHost),
    Dismiss(DatabaseFilterHost),
    Outside {
        action: DatabaseFilterOutsideAction,
        host: DatabaseFilterHost,
    },
    SetStage(DatabaseFilterDialogStage),
    SetAdvancedAddPath(Vec<usize>),
    SetPropertyQuery(String),
    MovePropertyHighlight(isize),
    ActivatePropertyHighlight,
    BeginFilter(Box<DatabaseProperty>),
    BeginAdvanced,
    SetValue(String),
    ToggleSelection(String),
    SetTextOperator(DatabaseTextFilterOperator),
    OpenOperatorPicker,
    MoveOperatorHighlight(isize),
    CloseValueEditor(DatabaseFilterHost),
    SetDate(chrono::NaiveDate),
    SetDateMode(DatabaseDateFilterMode),
    SetDateValueChoice(DatabaseDateValueChoice),
    MoveCalendarMonth(i32),
    CycleRelativeDirection,
    CycleRelativeUnit,
    AdjustRelativeCount(i32),
    RemoveActive(DatabaseFilterHost),
    PromoteActive,
    OpenAdvancedRule(Vec<usize>),
    OpenAdvancedPropertyPicker(Vec<usize>),
    AddAdvancedRule,
    AddAdvancedGroup,
    SetAdvancedOperator(DatabaseFilterGroupOperator),
    ToggleAdvancedGroup(Vec<usize>),
    RemoveAdvancedNode {
        path: Vec<usize>,
        host: DatabaseFilterHost,
    },
    ClearAdvanced(DatabaseFilterHost),
    MeasureAnchor {
        kind: DatabaseFilterAnchorKind,
        bounds: Bounds<Pixels>,
    },
    MeasureEditorAnchor {
        filter_id: Option<NotionDatabaseFilterId>,
        bounds: Bounds<Pixels>,
    },
}

pub(super) enum DatabaseFilterEffect {
    QueryProjection {
        filter_state: DatabaseViewFilterState,
        revision: u64,
    },
    LoadUsers,
    ScheduleRelationSearch,
    Save(DatabaseFilterHost),
    Show(DatabaseFilterHost),
    Dismiss {
        host: DatabaseFilterHost,
        close_local: bool,
    },
    Report(String),
}

pub(super) enum DatabaseFilterPresentationEffect {
    Show(DatabaseFilterHost),
    Dismiss {
        host: DatabaseFilterHost,
        close_local: bool,
    },
}

pub(super) const FILTER_PROPERTY_DIALOG_WIDTH: f32 = 290.0;
pub(super) const FILTER_PROPERTY_INPUT_HEIGHT: f32 = 48.0;
pub(super) const FILTER_PROPERTY_ROW_HEIGHT: f32 = 29.0;
pub(super) const FILTER_PROPERTY_ROW_CONTENT_HEIGHT: f32 = 28.0;
pub(super) const FILTER_PROPERTY_LIST_PADDING: f32 = 7.0;
pub(super) const FILTER_FULL_PAGE_PROPERTY_LIST_HEIGHT: f32 = 297.0;
pub(super) const FILTER_INLINE_PROPERTY_LIST_MAX_HEIGHT: f32 = 238.0;
pub(super) const FILTER_FOOTER_HEIGHT: f32 = 37.0;
pub(super) const FILTER_TEXT_EDITOR_WIDTH: f32 = 220.0;
pub(super) const FILTER_EDITOR_WIDTH: f32 = 260.0;
pub(super) const FILTER_EDITOR_HEIGHT: f32 = 69.0;
pub(super) const FILTER_PICKER_EDITOR_MAX_HEIGHT: f32 = 509.5;
pub(super) const FILTER_OPERATOR_WIDTH: f32 = 190.0;
pub(super) const FILTER_OPERATOR_ROW_HEIGHT: f32 = 28.0;
pub(super) const FILTER_OPERATOR_ROW_GAP: f32 = 1.0;
pub(super) const FILTER_DIALOG_MARGIN: f32 = 12.0;
pub(super) const CHECKBOX_FILTER_CHECKED_VALUE: &str = "checked";
pub(super) const CHECKBOX_FILTER_UNCHECKED_VALUE: &str = "unchecked";
pub(crate) const DATABASE_FILTER_BAR_HEIGHT: f32 = 44.0;

#[derive(Clone, Copy)]
pub(crate) enum DatabaseFilterDialogPlacement {
    FullPage,
    Inline {
        toolbar_anchor: Bounds<Pixels>,
        viewport_width: f32,
        viewport_height: f32,
    },
}

#[derive(Clone, Copy)]
pub(super) struct DatabaseFilterDialogSize {
    pub(super) width: f32,
    pub(super) height: f32,
}

impl DatabaseFilterDialogSize {
    pub(super) const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy)]
pub(super) struct DatabaseFilterDialogBehavior {
    pub(super) preferred_anchor: Option<Bounds<Pixels>>,
    pub(super) outside_action: Option<DatabaseFilterOutsideAction>,
}

impl DatabaseFilterDialogBehavior {
    pub(super) const fn new(
        preferred_anchor: Option<Bounds<Pixels>>,
        outside_action: Option<DatabaseFilterOutsideAction>,
    ) -> Self {
        Self {
            preferred_anchor,
            outside_action,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum DatabaseFilterOutsideAction {
    CloseToolbar,
    ReturnToEditor,
    ReturnToAdvancedEditor,
    ReturnToEditorAndFocusValue,
}

#[derive(Clone)]
pub(super) struct DatabaseFilterChip {
    pub(super) filter: Option<DatabaseSimpleFilter>,
    pub(super) property: DatabaseProperty,
    pub(super) label: String,
    pub(super) draft_filter_id: Option<NotionDatabaseFilterId>,
    pub(super) dirty: bool,
}

#[derive(Clone)]
pub(super) struct DatabaseFilterChoice {
    pub(super) key: Option<String>,
    pub(super) label: String,
    pub(super) secondary: Option<String>,
    pub(super) visual: Option<DatabaseFilterChoiceVisual>,
}

#[derive(Clone)]
pub(super) enum DatabaseFilterChoiceVisual {
    Avatar {
        profile_photo: Option<String>,
        fallback: String,
    },
    Page(PageShellIcon),
    OptionColor(String),
    Checkbox(bool),
}

#[derive(Clone, Copy)]
pub(super) enum DatabaseDateValueChoice {
    Point(crate::model::DatabaseRelativeDatePreset),
    CustomPoint,
    ThisWeek,
    Past(crate::model::DatabaseRelativeDateUnit),
    Future(crate::model::DatabaseRelativeDateUnit),
    CustomRange,
}

#[derive(Clone)]
pub(super) struct DatabaseAdvancedFilterRow {
    pub(super) path: Vec<usize>,
    pub(super) node: DatabaseFilterNode,
}

#[derive(Clone, Copy)]
pub(super) struct DatabaseCalendarSelection {
    pub(super) selected_date: Option<chrono::NaiveDate>,
    pub(super) range_start: Option<chrono::NaiveDate>,
    pub(super) range_end: Option<chrono::NaiveDate>,
}

#[derive(Clone, Copy)]
pub(super) enum DatabaseFilterAnchorKind {
    Operator,
    DateMode,
    DateValue,
    Actions,
}
