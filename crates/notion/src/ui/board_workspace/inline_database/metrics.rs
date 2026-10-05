use crate::ui::{SurfaceState, ViewTabKind};

use super::super::{
    calendar_view::CalendarLayout, list_gallery_view::DatabaseViewGeometry,
    table_view::TableViewGeometry,
};
use super::{
    INLINE_DATABASE_BOARD_TOP_INSET, INLINE_DATABASE_COLUMN_HEADER_HEIGHT,
    INLINE_DATABASE_CONTROLS_HEIGHT,
};

pub(crate) struct InlineDatabaseMetrics {
    pub(super) controls_height: f32,
    pub(super) active_view_height: f32,
}

impl InlineDatabaseMetrics {
    pub(super) fn new(surface: &SurfaceState, content_width: f32) -> Self {
        let board = &surface.board;
        let columns = &surface.columns;
        let search = &surface.database_search;
        let filter = &surface.database_filter;
        let rows = &surface.database_view_rows;
        let date_view = &surface.date_view;
        let active_view_height = match board.active_view_kind() {
            ViewTabKind::Board | ViewTabKind::Unknown => {
                let lane_height = columns
                    .iter()
                    .map(|column| search.visible_lane_height(&column.cards))
                    .fold(0.0_f32, f32::max);
                INLINE_DATABASE_BOARD_TOP_INSET + INLINE_DATABASE_COLUMN_HEADER_HEIGHT + lane_height
            }
            ViewTabKind::Table => TableViewGeometry::new(board, search).inline_height(),
            ViewTabKind::List => DatabaseViewGeometry::inline_list_height(
                search.visible_database_view_row_count(rows),
            ),
            ViewTabKind::Gallery => DatabaseViewGeometry::inline_gallery_height(
                search.visible_database_view_row_count(rows),
                content_width,
            ),
            ViewTabKind::Timeline => super::super::ACTIVE_VIEW_HEIGHT,
            ViewTabKind::Calendar => CalendarLayout::build(
                board
                    .calendar_view
                    .as_ref()
                    .expect("active calendar view must include calendar configuration"),
                date_view,
                search,
            )
            .active_view_height(),
        };
        let controls_height = INLINE_DATABASE_CONTROLS_HEIGHT
            + if filter.bar_is_visible(board) {
                super::super::dialogs::filter::DATABASE_FILTER_BAR_HEIGHT
            } else {
                0.0
            };
        Self {
            controls_height,
            active_view_height,
        }
    }

    pub(crate) fn total_height(&self) -> f32 {
        self.controls_height + self.active_view_height
    }
}
