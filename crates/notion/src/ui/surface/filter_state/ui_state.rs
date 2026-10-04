use super::{
    Arc, BoardSnapshot, Bounds, Cell, DatabaseFilterDialogStage, DatabaseFilterDraft,
    DatabaseFilterPropertyPickerTarget, DatabaseFilterRelationPage, DatabaseFilterUser,
    DatabaseViewFilterState, Datelike, Entity, Local, NotionCollectionViewId, Pixels, RefCell,
    TextInput,
};

pub(crate) struct DatabaseFilterUiState {
    pub(crate) session: Arc<()>,
    pub(crate) view_id: Option<NotionCollectionViewId>,
    pub(crate) temporary: Option<DatabaseViewFilterState>,
    pub(crate) revision: u64,
    pub(crate) applied_revision: u64,
    pub(crate) query_in_flight_revision: Option<u64>,
    pub(crate) query_debounce_revision: Option<u64>,
    pub(crate) pending_query: Option<(u64, DatabaseViewFilterState)>,
    pub(crate) query_failed_revision: Option<u64>,
    pub(crate) stage: DatabaseFilterDialogStage,
    pub(crate) property_picker_target: DatabaseFilterPropertyPickerTarget,
    pub(crate) draft: Option<DatabaseFilterDraft>,
    pub(crate) draft_advanced_path: Option<Vec<usize>>,
    pub(crate) advanced_add_parent_path: Vec<usize>,
    pub(crate) property_query: String,
    pub(crate) value_query: String,
    pub(crate) property_highlighted_index: usize,
    pub(crate) operator_highlighted_index: usize,
    pub(crate) property_input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) value_input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) property_focus_requested: Cell<bool>,
    pub(crate) value_focus_requested: Cell<bool>,
    pub(crate) editor_anchor: Option<Bounds<Pixels>>,
    pub(crate) operator_anchor: Option<Bounds<Pixels>>,
    pub(crate) date_mode_anchor: Option<Bounds<Pixels>>,
    pub(crate) date_value_anchor: Option<Bounds<Pixels>>,
    pub(crate) actions_anchor: Option<Bounds<Pixels>>,
    pub(crate) users: Option<Arc<[DatabaseFilterUser]>>,
    pub(crate) users_in_flight: bool,
    pub(crate) relation_pages: Arc<[DatabaseFilterRelationPage]>,
    pub(crate) relation_search_revision: u64,
    pub(crate) relation_search_in_flight_revision: Option<u64>,
    pub(crate) visible_date_month: chrono::NaiveDate,
    pub(crate) save_in_flight: bool,
}

impl DatabaseFilterUiState {
    /// The filters the active view carries on the server.
    pub(crate) fn persisted_state(&self, board: &BoardSnapshot) -> DatabaseViewFilterState {
        let Some(view_id) = self.view_id.as_ref() else {
            return DatabaseViewFilterState::default();
        };
        board
            .view_tabs
            .iter()
            .find(|view| view.active && &view.provider_view_id == view_id)
            .and_then(|view| view.filters.clone())
            .unwrap_or_default()
    }

    /// The filters in force: the unsaved edit when there is one, else the
    /// persisted set.
    pub(crate) fn effective_state(&self, board: &BoardSnapshot) -> DatabaseViewFilterState {
        self.temporary
            .clone()
            .unwrap_or_else(|| self.persisted_state(board))
    }

    pub(crate) fn for_board(board: &BoardSnapshot) -> Self {
        Self {
            session: Arc::new(()),
            view_id: board
                .view_tabs
                .iter()
                .find(|view| view.active)
                .map(|view| view.provider_view_id.clone()),
            temporary: None,
            revision: 0,
            applied_revision: 0,
            query_in_flight_revision: None,
            query_debounce_revision: None,
            pending_query: None,
            query_failed_revision: None,
            stage: DatabaseFilterDialogStage::PropertyPicker,
            property_picker_target: DatabaseFilterPropertyPickerTarget::Simple,
            draft: None,
            draft_advanced_path: None,
            advanced_add_parent_path: Vec::new(),
            property_query: String::new(),
            value_query: String::new(),
            property_highlighted_index: 0,
            operator_highlighted_index: 0,
            property_input: RefCell::new(None),
            value_input: RefCell::new(None),
            property_focus_requested: Cell::new(false),
            value_focus_requested: Cell::new(false),
            editor_anchor: None,
            operator_anchor: None,
            date_mode_anchor: None,
            date_value_anchor: None,
            actions_anchor: None,
            users: None,
            users_in_flight: false,
            relation_pages: Arc::from([]),
            relation_search_revision: 0,
            relation_search_in_flight_revision: None,
            visible_date_month: Local::now()
                .date_naive()
                .with_day(1)
                .expect("the first day exists in every Gregorian month"),
            save_in_flight: false,
        }
    }

    pub(crate) fn open_property_picker(&mut self) {
        self.stage = DatabaseFilterDialogStage::PropertyPicker;
        self.property_picker_target = DatabaseFilterPropertyPickerTarget::Simple;
        self.draft = None;
        self.draft_advanced_path = None;
        self.property_query.clear();
        self.property_highlighted_index = 0;
        self.property_input.borrow_mut().take();
        self.property_focus_requested.set(true);
    }

    pub(crate) fn open_advanced_property_picker(
        &mut self,
        target: DatabaseFilterPropertyPickerTarget,
    ) {
        debug_assert!(!matches!(
            target,
            DatabaseFilterPropertyPickerTarget::Simple
        ));
        self.stage = DatabaseFilterDialogStage::PropertyPicker;
        self.property_picker_target = target;
        self.draft = None;
        self.draft_advanced_path = None;
        self.property_query.clear();
        self.property_highlighted_index = 0;
        self.property_input.borrow_mut().take();
        self.property_focus_requested.set(true);
    }

    pub(crate) fn reset_temporary(&mut self) {
        self.temporary = None;
        self.draft = None;
        self.draft_advanced_path = None;
        self.property_picker_target = DatabaseFilterPropertyPickerTarget::Simple;
        self.advanced_add_parent_path.clear();
        self.stage = DatabaseFilterDialogStage::PropertyPicker;
        self.property_query.clear();
        self.value_query.clear();
        self.property_input.borrow_mut().take();
        self.value_input.borrow_mut().take();
        self.editor_anchor = None;
        self.operator_anchor = None;
        self.date_mode_anchor = None;
        self.date_value_anchor = None;
        self.actions_anchor = None;
    }

    pub(crate) fn advance_revision(&mut self) -> u64 {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("Notion database filter revision exhausted");
        self.revision
    }
}
