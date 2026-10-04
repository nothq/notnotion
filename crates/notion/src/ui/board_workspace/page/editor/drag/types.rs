use std::{
    cell::Cell,
    collections::HashSet,
    rc::Rc,
    sync::{Arc, OnceLock},
};

use gpui::{Bounds, Pixels, Point, RenderImage};

use crate::ui::{
    AppearanceMode, CachedNotionBlockImage, CardPageBlockColor, CardPageBlockContent,
    LoadedCardPageData, LoadedCardPageSimpleTableCell, NotionBlockImageCache,
    PageBlockDragScrollTarget, PageTextSelection,
};

use super::super::PageBlockRowSpacing;
use super::source::PageBlockDragSourceCountProof;

#[derive(Clone)]
pub(crate) struct PageBlockDragPayload {
    pub(in crate::ui::board_workspace::page::editor) source: PageBlockDragSource,
    pub(in crate::ui::board_workspace::page::editor) resolved:
        Arc<OnceLock<ResolvedPageBlockDragPayload>>,
    pub(in crate::ui::board_workspace::page::editor) preview_width: PageBlockDragPreviewWidth,
    pub(in crate::ui::board_workspace::page::editor) cursor_offset: Point<Pixels>,
    pub(in crate::ui::board_workspace::page::editor) appearance_mode: AppearanceMode,
    pub(in crate::ui::board_workspace::page::editor) toggle_markers: [Arc<RenderImage>; 10],
    pub(in crate::ui::board_workspace::page::editor) page_marker: Arc<RenderImage>,
    pub(in crate::ui::board_workspace::page::editor) checked_marker: Arc<RenderImage>,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) enum PageBlockDragPreviewWidth {
    ExistingLinear(f32),
    RecursiveFlow(Rc<Cell<f32>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ui::board_workspace::page::editor) enum PageBlockDragGeometryMode {
    ExistingLinear,
    RecursiveFlow,
}

impl PageBlockDragPreviewWidth {
    pub(in crate::ui::board_workspace::page::editor) fn for_data(
        data: &LoadedCardPageData,
        existing_linear: f32,
    ) -> Self {
        if data.has_column_structure() {
            Self::RecursiveFlow(Rc::new(Cell::new(0.0)))
        } else {
            Self::ExistingLinear(existing_linear)
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn current(&self) -> f32 {
        match self {
            Self::ExistingLinear(width) => *width,
            Self::RecursiveFlow(width) => width.get(),
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn set_recursive(&self, width: f32) -> bool {
        let Self::RecursiveFlow(current) = self else {
            return false;
        };
        if !width.is_finite() || width <= 0.0 {
            return false;
        }
        if current.get().to_bits() == width.to_bits() {
            return false;
        }
        current.set(width);
        true
    }
}

impl PageBlockDragGeometryMode {
    pub(in crate::ui::board_workspace::page::editor) fn for_data(
        data: &LoadedCardPageData,
    ) -> Self {
        if data.has_column_structure() {
            Self::RecursiveFlow
        } else {
            Self::ExistingLinear
        }
    }
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageBlockDragSource {
    pub(in crate::ui::board_workspace::page::editor) data: Arc<LoadedCardPageData>,
    pub(in crate::ui::board_workspace::page::editor) dragged_index: usize,
    pub(in crate::ui::board_workspace::page::editor) dragged_block_id: String,
    pub(in crate::ui::board_workspace::page::editor) selection: Arc<PageBlockDragSelection>,
    pub(in crate::ui::board_workspace::page::editor) block_images: Rc<NotionBlockImageCache>,
}

pub(in crate::ui::board_workspace::page::editor) struct PageBlockDragSelection {
    pub(in crate::ui::board_workspace::page::editor) block_selection: Vec<String>,
    pub(in crate::ui::board_workspace::page::editor) text_selection: Option<PageTextSelection>,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct ResolvedPageBlockDragPayload {
    pub(in crate::ui::board_workspace::page::editor) root_block_ids: Vec<String>,
    pub(super) subtree_membership: HashSet<String>,
    pub(super) source_count_proof: PageBlockDragSourceCountProof,
    pub(in crate::ui::board_workspace::page::editor) preview_rows: PageBlockDragPreviewRows,
    pub(in crate::ui::board_workspace::page::editor) preview_anchor_index: usize,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageBlockDragPreviewRows {
    pub(in crate::ui::board_workspace::page::editor) rows: Vec<PageBlockDragPreviewRow>,
    pub(in crate::ui::board_workspace::page::editor) overflow_count: usize,
}

#[derive(Clone)]
pub(crate) struct PageBlockDragPreviewRow {
    pub(in crate::ui::board_workspace::page::editor) content: CardPageBlockContent,
    pub(in crate::ui::board_workspace::page::editor) color: CardPageBlockColor,
    pub(in crate::ui::board_workspace::page::editor) numbered_index: usize,
    pub(in crate::ui::board_workspace::page::editor) depth: usize,
    pub(in crate::ui::board_workspace::page::editor) row_spacing: Option<PageBlockRowSpacing>,
    pub(in crate::ui::board_workspace::page::editor) format: crate::model::CardPageFormat,
    pub(in crate::ui::board_workspace::page::editor) cached_block_image:
        Option<CachedNotionBlockImage>,
    pub(in crate::ui::board_workspace::page::editor) simple_table_rows:
        PageBlockDragPreviewTableRows,
}

type PageBlockDragPreviewTableRows = Arc<[Arc<[LoadedCardPageSimpleTableCell]>]>;

#[derive(Default)]
pub(in crate::ui::board_workspace::page::editor) struct PageBlockDragLayout {
    pub(in crate::ui::board_workspace::page::editor) generation: u64,
    pub(in crate::ui::board_workspace::page::editor) container_bounds: Option<Bounds<Pixels>>,
    pub(in crate::ui::board_workspace::page::editor) document_unit_indices: Vec<usize>,
    pub(in crate::ui::board_workspace::page::editor) owner_visible_row_indices: Vec<usize>,
    pub(in crate::ui::board_workspace::page::editor) row_bounds: Vec<Bounds<Pixels>>,
    pub(in crate::ui::board_workspace::page::editor) owner_block_ids: Vec<String>,
    pub(in crate::ui::board_workspace::page::editor) observation: Option<PageBlockDragObservation>,
}

#[derive(Default)]
pub(crate) struct PageBlockDragLayouts {
    pub(in crate::ui::board_workspace::page::editor) standalone: PageBlockDragLayout,
    pub(in crate::ui::board_workspace::page::editor) selected_page: PageBlockDragLayout,
}

#[derive(Clone)]
pub(in crate::ui::board_workspace) struct PageBlockDragObservation {
    pub(in crate::ui::board_workspace::page::editor) scroll_target: PageBlockDragScrollTarget,
    pub(in crate::ui::board_workspace::page::editor) pointer: Point<Pixels>,
    pub(in crate::ui::board_workspace::page::editor) container_bounds: Bounds<Pixels>,
    pub(in crate::ui::board_workspace::page::editor) payload: PageBlockDragPayload,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PageBlockDragTarget {
    pub(in crate::ui::board_workspace::page::editor) scroll_target: PageBlockDragScrollTarget,
    pub(in crate::ui::board_workspace::page::editor) page_id: String,
    pub(in crate::ui::board_workspace::page::editor) dragged_root_block_ids: Vec<String>,
    pub(in crate::ui::board_workspace::page::editor) target_parent_block_id: String,
    pub(in crate::ui::board_workspace::page::editor) before_block_id: Option<String>,
    pub(in crate::ui::board_workspace::page::editor) destination_depth: usize,
    pub(in crate::ui::board_workspace::page::editor) indicator_left: f32,
    pub(in crate::ui::board_workspace::page::editor) indicator_top: f32,
    pub(in crate::ui::board_workspace::page::editor) indicator_width: f32,
    pub(in crate::ui::board_workspace::page::editor) wash: Option<PageBlockDragWash>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::ui::board_workspace::page::editor) struct PageBlockDragWash {
    pub(in crate::ui::board_workspace::page::editor) top: f32,
    pub(in crate::ui::board_workspace::page::editor) left: f32,
    pub(in crate::ui::board_workspace::page::editor) width: f32,
    pub(in crate::ui::board_workspace::page::editor) height: f32,
}
