use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
};

use gpui::Entity;
use gpui_components::text_input::TextInput;

use crate::model::BoardItem;
use crate::ui::{BoardSnapshot, Card, ColumnState, CARD_GAP};

pub(crate) struct DatabaseSearchState {
    pub(crate) open: bool,
    pub(crate) input: RefCell<Option<Entity<TextInput>>>,
    pub(crate) clear_focus: RefCell<Option<gpui::FocusHandle>>,
    pub(crate) query: String,
    search_keys_by_block_id: HashMap<String, String>,
    matching_block_ids: Option<HashSet<String>>,
    focus_requested: Cell<bool>,
}

impl Default for DatabaseSearchState {
    fn default() -> Self {
        Self {
            open: false,
            input: RefCell::new(None),
            clear_focus: RefCell::new(None),
            query: String::new(),
            search_keys_by_block_id: HashMap::new(),
            matching_block_ids: None,
            focus_requested: Cell::new(false),
        }
    }
}

impl DatabaseSearchState {
    /// Whether a board item survives the current search.
    pub(crate) fn item_is_visible(&self, item: &BoardItem) -> bool {
        self.matches(&item.block_id)
    }

    pub(crate) fn card_is_visible(&self, card: &Card) -> bool {
        self.matches(&card.block_id)
    }

    pub(crate) fn visible_card_count(&self, cards: &[Card]) -> usize {
        cards
            .iter()
            .filter(|card| self.card_is_visible(card))
            .count()
    }

    /// Height of a lane once the hidden cards are taken out of it.
    pub(crate) fn visible_lane_height(&self, cards: &[Card]) -> f32 {
        let (height, count) = cards
            .iter()
            .filter(|card| self.card_is_visible(card))
            .fold((0.0, 0_usize), |(height, count), card| {
                (height + card.height, count + 1)
            });
        height + count as f32 * CARD_GAP + 51.0
    }

    pub(crate) fn rebuild_index(&mut self, board: &BoardSnapshot, columns: &[ColumnState]) {
        self.search_keys_by_block_id.clear();
        self.search_keys_by_block_id.extend(
            board
                .items
                .iter()
                .map(|item| (item.block_id.clone(), item.title.to_lowercase())),
        );
        for card in columns.iter().flat_map(|column| &column.cards) {
            self.search_keys_by_block_id
                .entry(card.block_id.clone())
                .or_insert_with(|| card.title.to_lowercase());
        }
        self.rebuild_matches();
    }

    pub(crate) fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.matching_block_ids = None;
        self.input.borrow_mut().take();
        self.focus_requested.set(true);
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.matching_block_ids = None;
        self.input.borrow_mut().take();
        self.focus_requested.set(false);
    }

    pub(crate) fn set_query(&mut self, query: String) {
        self.query = query;
        self.rebuild_matches();
    }

    pub(crate) fn clear_query(&mut self) {
        self.query.clear();
        self.matching_block_ids = None;
        self.focus_requested.set(false);
    }

    pub(crate) fn take_focus_request(&self) -> bool {
        self.focus_requested.replace(false)
    }

    pub(crate) fn is_filtering(&self) -> bool {
        self.open && self.matching_block_ids.is_some()
    }

    pub(crate) fn matches(&self, block_id: &str) -> bool {
        !self.open
            || self
                .matching_block_ids
                .as_ref()
                .is_none_or(|matches| matches.contains(block_id))
    }

    fn rebuild_matches(&mut self) {
        let normalized_query = self.query.trim().to_lowercase();
        self.matching_block_ids = (!normalized_query.is_empty()).then(|| {
            self.search_keys_by_block_id
                .iter()
                .filter(|(_, search_key)| search_key.contains(&normalized_query))
                .map(|(block_id, _)| block_id.clone())
                .collect()
        });
    }
}
