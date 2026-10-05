use crate::model::{BoardItem, BoardSnapshot, ViewTab};
use crate::ui::{Arc, CivilDate, ViewTabKind};
use gpui::{Bounds, Entity, FocusHandle, Pixels, SharedString, UniformListScrollHandle};
use gpui_components::text_input::TextInput;
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
};

impl BoardSnapshot {
    pub(crate) fn active_date_view(&self) -> Option<&ViewTab> {
        self.view_tabs.iter().find(|tab| {
            tab.active && matches!(tab.kind, ViewTabKind::Calendar | ViewTabKind::Timeline)
        })
    }
}

pub(crate) struct NotionDateViewState {
    pub(crate) visible_month: CivilDate,
    pub(crate) query_session: Arc<()>,
    pub(crate) query_schedule_pending: Cell<bool>,
    pub(crate) calendar_items: CalendarItemsState,
    pub(crate) range_resize_preview: Option<CalendarRangeResizePreview>,
    pub(crate) undated: DateUndatedItemsState,
    pub(crate) undated_badge_bounds: Option<Bounds<Pixels>>,
}

impl NotionDateViewState {
    pub(crate) fn new(items: Vec<BoardItem>, visible_month: CivilDate) -> Self {
        Self {
            visible_month,
            query_session: Arc::new(()),
            query_schedule_pending: Cell::default(),
            calendar_items: CalendarItemsState::new(items, visible_month),
            range_resize_preview: None,
            undated: DateUndatedItemsState::default(),
            undated_badge_bounds: None,
        }
    }

    pub(crate) fn reset_calendar_items(
        &mut self,
        view_identity: Option<String>,
        board_items: &[BoardItem],
    ) {
        self.range_resize_preview = None;
        self.calendar_items.day_focus_handles.borrow_mut().clear();
        self.calendar_items.generation = self
            .calendar_items
            .generation
            .checked_add(1)
            .expect("Notion Calendar request generation exhausted");
        self.calendar_items.view_identity = view_identity;
        self.calendar_items.initialized = false;
        self.calendar_items.pending = false;
        self.calendar_items.requested_month = None;
        self.calendar_items.loaded_month = Some(self.visible_month);
        self.calendar_items.items = board_items.to_vec().into();
    }

    pub(crate) fn reset_for_route(&mut self, board_items: &[BoardItem]) {
        self.query_session = Arc::new(());
        self.query_schedule_pending.set(false);
        self.reset_calendar_items(None, board_items);
        self.undated = DateUndatedItemsState::default();
        self.undated_badge_bounds = None;
    }

    pub(crate) fn reset_for_projection(&mut self, board_items: Vec<BoardItem>) {
        *self = Self::new(board_items, self.visible_month);
    }
}

pub(crate) struct CalendarItemsState {
    pub(crate) view_identity: Option<String>,
    pub(crate) generation: u64,
    pub(crate) initialized: bool,
    pub(crate) pending: bool,
    pub(crate) requested_month: Option<CivilDate>,
    pub(crate) loaded_month: Option<CivilDate>,
    pub(crate) items: Arc<[BoardItem]>,
    pub(crate) creations_in_flight: HashSet<String>,
    pub(crate) day_focus_handles: RefCell<HashMap<String, FocusHandle>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CalendarRangeResizePreview {
    pub(crate) block_id: String,
    pub(crate) inclusive_start: CivilDate,
    pub(crate) inclusive_end: CivilDate,
}

impl CalendarItemsState {
    pub(crate) fn new(items: Vec<BoardItem>, visible_month: CivilDate) -> Self {
        Self {
            view_identity: None,
            generation: 0,
            initialized: false,
            pending: false,
            requested_month: None,
            loaded_month: Some(visible_month),
            items: items.into(),
            creations_in_flight: HashSet::new(),
            day_focus_handles: RefCell::new(HashMap::new()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DateUndatedRow {
    pub(crate) item: BoardItem,
    pub(crate) element_id: SharedString,
    pub(crate) title: SharedString,
}

pub(crate) struct DateUndatedItemsState {
    pub(crate) view_identity: Option<String>,
    pub(crate) count_generation: u64,
    pub(crate) count_initialized: bool,
    pub(crate) count_retry_attempts: u8,
    pub(crate) count_retry_scheduled: bool,
    pub(crate) open: bool,
    pub(crate) query_generation: u64,
    pub(crate) query: String,
    pub(crate) limit: usize,
    pub(crate) returned_block_count: usize,
    pub(crate) pending: bool,
    pub(crate) pagination_armed: bool,
    pub(crate) loaded: bool,
    pub(crate) rows: Arc<[DateUndatedRow]>,
    pub(crate) assignments_in_flight: HashSet<String>,
    pub(crate) input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) scroll_handle: UniformListScrollHandle,
}

impl Default for DateUndatedItemsState {
    fn default() -> Self {
        Self {
            view_identity: None,
            count_generation: 0,
            count_initialized: false,
            count_retry_attempts: 0,
            count_retry_scheduled: false,
            open: false,
            query_generation: 0,
            query: String::new(),
            limit: 20,
            returned_block_count: 0,
            pending: false,
            pagination_armed: false,
            loaded: false,
            rows: Default::default(),
            assignments_in_flight: Default::default(),
            input: Default::default(),
            scroll_handle: UniformListScrollHandle::new(),
        }
    }
}
