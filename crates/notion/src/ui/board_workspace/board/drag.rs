use super::super::{
    px, CardPlacement, ColumnState, DragState, DropTarget, Pixels, Point, BOARD_VIEWPORT_X,
    BOARD_VIEWPORT_Y, CARD_GAP, CARD_TOP, CARD_X_OFFSET, COLUMN_STRIDE, DRAG_AUTO_SCROLL_EDGE,
    DRAG_AUTO_SCROLL_STEP,
};

pub(crate) struct BoardDragLayout<'a> {
    pub(crate) columns: &'a [ColumnState],
    pub(crate) viewport_width: f32,
    pub(crate) scroll_x: f32,
    pub(crate) max_scroll_x: f32,
    pub(crate) surface_offset: f32,
}

impl BoardDragLayout<'_> {
    pub(crate) fn auto_scroll_target(&self, pointer_position: Point<Pixels>) -> Option<(f32, f32)> {
        let pointer_position = self.notion_board_pointer(pointer_position);
        if pointer_position.y < px(BOARD_VIEWPORT_Y)
            || pointer_position.y > px(BOARD_VIEWPORT_Y + 626.5)
        {
            return None;
        }

        let left_edge = BOARD_VIEWPORT_X + DRAG_AUTO_SCROLL_EDGE;
        let right_edge = self.viewport_width - DRAG_AUTO_SCROLL_EDGE;
        let max_scroll_x = self.max_scroll_x;
        let current_scroll_x = self.scroll_x;

        let next = if pointer_position.x.as_f32() <= left_edge {
            (current_scroll_x - DRAG_AUTO_SCROLL_STEP).max(0.0)
        } else if pointer_position.x.as_f32() >= right_edge {
            (current_scroll_x + DRAG_AUTO_SCROLL_STEP).min(max_scroll_x)
        } else {
            return None;
        };
        Some((next, max_scroll_x))
    }

    pub(crate) fn drop_target(
        &self,
        pointer: Point<Pixels>,
        dragged_block_id: &str,
    ) -> Option<DropTarget> {
        let pointer = self.notion_board_pointer(pointer);
        if pointer.x < px(0.0)
            || pointer.x > px(self.viewport_width)
            || pointer.y < px(CARD_TOP - 6.0)
            || pointer.y > px(BOARD_VIEWPORT_Y + 626.5)
        {
            return None;
        }

        let column_float = (pointer.x.as_f32() + self.scroll_x - BOARD_VIEWPORT_X) / COLUMN_STRIDE;
        let column_index = column_float
            .floor()
            .clamp(0.0, (self.columns.len().saturating_sub(1)) as f32)
            as usize;

        let placements = self.column_placements(column_index, Some(dragged_block_id));
        let mut card_index = placements.len();
        for placement in placements {
            let midpoint = placement.top + placement.height / 2.0;
            if pointer.y.as_f32() < midpoint {
                card_index = placement.card_index;
                break;
            }
        }

        Some(DropTarget {
            column_index,
            card_index,
        })
    }

    pub(crate) fn move_anchor_before(
        &self,
        target: DropTarget,
        dragged_block_id: &str,
    ) -> Option<String> {
        let target_cards = self.columns[target.column_index]
            .cards
            .iter()
            .filter(|card| card.block_id != dragged_block_id)
            .collect::<Vec<_>>();
        if let Some(card) = target_cards.get(target.card_index) {
            return Some(card.block_id.clone());
        }

        self.columns
            .iter()
            .skip(target.column_index + 1)
            .find_map(|column| {
                column
                    .cards
                    .iter()
                    .find(|card| card.block_id != dragged_block_id)
                    .map(|card| card.block_id.clone())
            })
    }

    pub(crate) fn move_anchor_after(
        &self,
        target: DropTarget,
        dragged_block_id: &str,
    ) -> Option<String> {
        let target_cards = self.columns[target.column_index]
            .cards
            .iter()
            .filter(|card| card.block_id != dragged_block_id)
            .collect::<Vec<_>>();
        if target.card_index > 0 {
            if let Some(card) = target_cards.get(target.card_index - 1) {
                return Some(card.block_id.clone());
            }
        }

        self.columns
            .iter()
            .take(target.column_index)
            .rev()
            .find_map(|column| {
                column
                    .cards
                    .iter()
                    .rev()
                    .find(|card| card.block_id != dragged_block_id)
                    .map(|card| card.block_id.clone())
            })
    }

    pub(crate) fn card_left(&self, column_index: usize) -> f32 {
        BOARD_VIEWPORT_X - self.scroll_x + column_index as f32 * COLUMN_STRIDE + CARD_X_OFFSET
    }

    fn notion_board_pointer(&self, pointer: Point<Pixels>) -> Point<Pixels> {
        Point {
            x: pointer.x - px(self.surface_offset),
            y: pointer.y,
        }
    }

    pub(crate) fn card_top(&self, column_index: usize, card_index: usize) -> f32 {
        let mut top = CARD_TOP;
        for card in self.columns[column_index].cards.iter().take(card_index) {
            top += card.height + CARD_GAP;
        }
        top
    }

    pub(crate) fn column_placements(
        &self,
        column_index: usize,
        dragged_block_id: Option<&str>,
    ) -> Vec<CardPlacement> {
        let mut top = CARD_TOP;
        self.columns[column_index]
            .cards
            .iter()
            .enumerate()
            .filter_map(|(card_index, card)| {
                if dragged_block_id.is_some() && dragged_block_id == Some(card.block_id.as_str()) {
                    return None;
                }

                let placement = CardPlacement {
                    card_index,
                    top,
                    height: card.height,
                };
                top += card.height + CARD_GAP;
                Some(placement)
            })
            .collect()
    }
}

pub(crate) fn apply_board_drop(columns: &mut [ColumnState], drag: DragState, target: DropTarget) {
    let mut card = columns[drag.source_column].cards.remove(drag.source_index);
    if let Some(fill_override) = card.fill_override {
        if fill_override == 0x32475b && drag.source_column != target.column_index {
            card.fill_override = None;
        }
    }

    let mut insert_index = target.card_index;
    if drag.source_column == target.column_index && insert_index > drag.source_index {
        insert_index -= 1;
    }
    insert_index = insert_index.min(columns[target.column_index].cards.len());
    columns[target.column_index]
        .cards
        .insert(insert_index, card);
}
