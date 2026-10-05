use crate::ui::board_workspace::PageDocumentAction;
use std::sync::Arc;

use gpui::Window;

use crate::ui::{
    surface::{DatabaseSearchState, DatabaseViewRow},
    view_actions::ViewActionSink,
    Card, Context, SurfaceState,
};

mod gallery;
mod list;
mod renderer;

use renderer::DatabaseViewRenderer;

const LIST_ROW_HEIGHT: f32 = 52.0;
const LIST_TITLE_WIDTH: f32 = 280.0;
const GALLERY_CARD_MIN_WIDTH: f32 = 220.0;
const GALLERY_CARD_HEIGHT: f32 = 224.0;
const GALLERY_GAP: f32 = 12.0;
const GALLERY_ROW_HEIGHT: f32 = GALLERY_CARD_HEIGHT + GALLERY_GAP;
const INLINE_DATABASE_LIST_MAX_HEIGHT: f32 = 416.0;
const INLINE_DATABASE_GALLERY_MAX_HEIGHT: f32 = 520.0;
const DATABASE_EMPTY_VIEW_HEIGHT: f32 = 160.0;

#[derive(Clone)]
pub(super) enum DatabaseViewAction {
    OpenCard(Card),
}

#[derive(Clone)]
pub(super) struct VisibleDatabaseRows {
    rows: Arc<[DatabaseViewRow]>,
    source_indices: Arc<[usize]>,
}

impl VisibleDatabaseRows {
    pub(super) fn len(&self) -> usize {
        self.source_indices.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.source_indices.is_empty()
    }

    pub(super) fn row(&self, visible_index: usize) -> Option<&DatabaseViewRow> {
        let source_index = *self.source_indices.get(visible_index)?;
        self.rows.get(source_index)
    }
}

#[derive(Clone, Copy)]
pub(super) struct DatabaseViewViewport {
    pub(super) element_id: &'static str,
    pub(super) width: f32,
    pub(super) height: f32,
}

impl DatabaseSearchState {
    pub(super) fn visible_database_view_rows(
        &self,
        rows: Arc<[DatabaseViewRow]>,
    ) -> VisibleDatabaseRows {
        let source_indices = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| self.matches(row.block_id.as_ref()))
            .map(|(index, _)| index)
            .collect::<Vec<_>>()
            .into();
        VisibleDatabaseRows {
            rows,
            source_indices,
        }
    }

    pub(super) fn visible_database_view_row_count(&self, rows: &[DatabaseViewRow]) -> usize {
        rows.iter()
            .filter(|row| self.matches(row.block_id.as_ref()))
            .count()
    }
}

pub(super) struct DatabaseViewGeometry;

impl DatabaseViewGeometry {
    pub(super) fn inline_list_height(visible_item_count: usize) -> f32 {
        Self::bounded_inline_height(
            visible_item_count as f32 * LIST_ROW_HEIGHT,
            INLINE_DATABASE_LIST_MAX_HEIGHT,
        )
    }

    pub(super) fn inline_gallery_height(visible_item_count: usize, width: f32) -> f32 {
        let row_count =
            Self::gallery_row_count(visible_item_count, Self::gallery_cards_per_row(width));
        Self::bounded_inline_height(
            row_count as f32 * GALLERY_ROW_HEIGHT,
            INLINE_DATABASE_GALLERY_MAX_HEIGHT,
        )
    }

    pub(super) fn gallery_cards_per_row(width: f32) -> usize {
        (((width + GALLERY_GAP) / (GALLERY_CARD_MIN_WIDTH + GALLERY_GAP)).floor() as usize).max(1)
    }

    pub(super) fn gallery_card_width(width: f32, cards_per_row: usize) -> f32 {
        ((width - GALLERY_GAP * cards_per_row.saturating_sub(1) as f32) / cards_per_row as f32)
            .max(0.0)
    }

    pub(super) fn gallery_row_count(item_count: usize, cards_per_row: usize) -> usize {
        item_count.div_ceil(cards_per_row)
    }

    fn bounded_inline_height(content_height: f32, maximum: f32) -> f32 {
        if content_height <= 0.0 {
            DATABASE_EMPTY_VIEW_HEIGHT
        } else {
            content_height.min(maximum)
        }
    }
}

pub(super) fn database_view_card(row: &DatabaseViewRow) -> Card {
    Card {
        title: row.title.to_string(),
        block_id: row.block_id.to_string(),
        height: 40.0,
        has_content: true,
        icon: row.icon.clone(),
        fill_override: None,
    }
}

pub(super) fn database_view_renderer(
    surface: &SurfaceState,
    cx: &Context<SurfaceState>,
) -> DatabaseViewRenderer {
    DatabaseViewRenderer {
        theme: surface.theme,
        appearance_mode: surface.appearance_mode,
        page_icons: surface.page_shell_icon_renderer(cx),
        actions: ViewActionSink::new(cx, handle_database_view_action),
        visible_rows: surface
            .database_search
            .visible_database_view_rows(surface.database_view_rows.clone()),
        content_width: (surface.board_viewport().width
            - crate::ui::BOARD_VIEWPORT_X
            - crate::ui::BOARD_VIEWPORT_RIGHT_GUTTER)
            .max(0.0),
    }
}

pub(super) fn handle_database_view_action(
    surface: &mut SurfaceState,
    action: DatabaseViewAction,
    _window: &mut Window,
    cx: &mut Context<SurfaceState>,
) {
    match action {
        DatabaseViewAction::OpenCard(card) => {
            surface.dispatch_page_document_action(PageDocumentAction::OpenCard(card), cx)
        }
    }
}
