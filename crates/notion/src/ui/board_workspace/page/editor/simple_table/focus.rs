use gpui::{point, px, App, Context, ScrollHandle, Window};
use gpui_components::text_input::floor_grapheme_boundary;

use crate::model::CardPageSimpleTableCellAddress;
use crate::ui::surface::{
    PageEditorState, PageSimpleTableCellFocusMode, PageSimpleTableCellGeneration,
};
use crate::ui::SurfaceState;

use super::super::render::{handle_page_render_action, PageRenderAction, PageTableRenderAction};
use super::super::SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH;

const SIMPLE_TABLE_OUTER_SPACING: f32 = 8.0;
const SIMPLE_TABLE_OVERFLOW_EPSILON: f32 = 0.5;

#[derive(Clone, Copy)]
pub(in crate::ui::board_workspace) enum PageSimpleTableColumnWidthSpec {
    Explicit(f32),
    Auto,
}

pub(in crate::ui::board_workspace::page::editor) struct PageSimpleTableCellMountRequest<'a> {
    address: &'a CardPageSimpleTableCellAddress,
    generation: PageSimpleTableCellGeneration,
    scroll_handle: &'a ScrollHandle,
    column_widths: &'a [PageSimpleTableColumnWidthSpec],
    column_index: usize,
}

impl<'a> PageSimpleTableCellMountRequest<'a> {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        address: &'a CardPageSimpleTableCellAddress,
        generation: PageSimpleTableCellGeneration,
        scroll_handle: &'a ScrollHandle,
        column_widths: &'a [PageSimpleTableColumnWidthSpec],
        column_index: usize,
    ) -> Self {
        Self {
            address,
            generation,
            scroll_handle,
            column_widths,
            column_index,
        }
    }
}

impl PageEditorState {
    pub(in crate::ui::board_workspace::page::editor) fn complete_page_simple_table_cell_mount(
        &mut self,
        request: PageSimpleTableCellMountRequest<'_>,
        window: &mut Window,
        cx: &mut Context<SurfaceState>,
    ) {
        let PageSimpleTableCellMountRequest {
            address,
            generation,
            scroll_handle,
            column_widths,
            column_index,
        } = request;
        let (input, pending) = {
            let active = self.tables.editor().borrow();
            let Some(active) = active
                .as_ref()
                .filter(|active| active.address == *address && active.generation == generation)
            else {
                return;
            };
            let (Some(input), Some(pending)) = (active.input.clone(), active.pending_focus.clone())
            else {
                return;
            };
            (input, pending)
        };
        let horizontal_revealed = pending.horizontal_revealed
            || reveal_simple_table_column(scroll_handle, column_widths, column_index);
        let caret_applied =
            pending.caret_applied || apply_simple_table_cell_focus(&input, &pending.mode, cx);
        let complete = horizontal_revealed && caret_applied;
        self.register_page_simple_table_cell_blur(&request, &input, window, cx);
        {
            let mut active = self.tables.editor().borrow_mut();
            let Some(active) = active
                .as_mut()
                .filter(|active| active.address == *address && active.generation == generation)
            else {
                return;
            };
            if complete {
                active.pending_focus = None;
            } else if let Some(pending) = active.pending_focus.as_mut() {
                pending.horizontal_revealed = horizontal_revealed;
                pending.caret_applied = caret_applied;
            }
        }
        if complete {
            let focus = input.read(cx).focus_handle_clone();
            window.focus(&focus, cx);
        } else {
            cx.notify();
        }
    }
    fn register_page_simple_table_cell_blur(
        &self,
        request: &PageSimpleTableCellMountRequest<'_>,
        input: &gpui::Entity<gpui_components::text_input::TextInput>,
        window: &mut Window,
        cx: &mut Context<SurfaceState>,
    ) {
        let PageSimpleTableCellMountRequest {
            address,
            generation,
            ..
        } = *request;
        let should_register = {
            let mut active = self.tables.editor().borrow_mut();
            let Some(active) = active
                .as_mut()
                .filter(|active| active.address == *address && active.generation == generation)
            else {
                return;
            };
            if active.blur_registered {
                false
            } else {
                active.blur_registered = true;
                true
            }
        };
        if !should_register {
            return;
        }
        let focus = input.read(cx).focus_handle_clone();
        let address = address.clone();
        cx.on_blur(&focus, window, move |surface, window, cx| {
            handle_page_render_action(
                surface,
                PageRenderAction::Table(PageTableRenderAction::Edit(
                    super::PageSimpleTableEditAction::FinishBlur {
                        address: address.clone(),
                        generation,
                    },
                )),
                window,
                cx,
            );
        })
        .detach();
    }
}

fn apply_simple_table_cell_focus(
    input: &gpui::Entity<gpui_components::text_input::TextInput>,
    mode: &PageSimpleTableCellFocusMode,
    cx: &mut App,
) -> bool {
    let text_len = input.read(cx).text().len();
    let offset = match mode {
        PageSimpleTableCellFocusMode::Pointer(position) => {
            match input.read(cx).byte_offset_for_window_position(*position) {
                Some(offset) => offset,
                None => return false,
            }
        }
        PageSimpleTableCellFocusMode::Start => 0,
        PageSimpleTableCellFocusMode::End => text_len,
        PageSimpleTableCellFocusMode::Offset(offset) => {
            floor_grapheme_boundary(input.read(cx).text(), (*offset).min(text_len))
        }
        PageSimpleTableCellFocusMode::Vertical { window_x, line } => {
            let Some(offset) = input
                .read(cx)
                .outer_visual_line_offset_for_window_x(*line, *window_x)
            else {
                return false;
            };
            offset
        }
    };
    input.update(cx, |input, cx| {
        input.set_selection(offset..offset, false, cx);
        if let PageSimpleTableCellFocusMode::Vertical { window_x, .. } = mode {
            input.set_vertical_navigation_window_x(*window_x);
        }
    });
    true
}

fn reveal_simple_table_column(
    handle: &ScrollHandle,
    column_specs: &[PageSimpleTableColumnWidthSpec],
    column_index: usize,
) -> bool {
    let bounds = handle.bounds();
    let viewport_width = bounds.size.width.as_f32();
    if viewport_width <= 0.0 {
        return false;
    }
    let column_widths = resolved_simple_table_column_widths(column_specs, viewport_width);
    let expected_width = column_widths.iter().sum::<f32>() + SIMPLE_TABLE_OUTER_SPACING * 2.0;
    let max_scroll_x = handle.max_offset().x.as_f32().max(0.0);
    if expected_width > viewport_width + SIMPLE_TABLE_OVERFLOW_EPSILON && max_scroll_x == 0.0 {
        return false;
    }
    if max_scroll_x == 0.0 {
        return true;
    }
    let left = SIMPLE_TABLE_OUTER_SPACING + column_widths.iter().take(column_index).sum::<f32>();
    let right = left + column_widths.get(column_index).copied().unwrap_or_default();
    let current = (-handle.offset().x.as_f32()).clamp(0.0, max_scroll_x);
    let target = if right - left > viewport_width || left < current {
        left
    } else if right > current + viewport_width {
        right - viewport_width
    } else {
        current
    }
    .clamp(0.0, max_scroll_x);
    let offset = handle.offset();
    handle.set_offset(point(px(-target), offset.y));
    true
}

fn resolved_simple_table_column_widths(
    specs: &[PageSimpleTableColumnWidthSpec],
    viewport_width: f32,
) -> Vec<f32> {
    let explicit_sum = specs
        .iter()
        .filter_map(|spec| match spec {
            PageSimpleTableColumnWidthSpec::Explicit(width) => Some(*width),
            PageSimpleTableColumnWidthSpec::Auto => None,
        })
        .sum::<f32>();
    let auto_count = specs
        .iter()
        .filter(|spec| matches!(spec, PageSimpleTableColumnWidthSpec::Auto))
        .count();
    let inner_width = (viewport_width - SIMPLE_TABLE_OUTER_SPACING * 2.0).max(0.0);
    let grid_width =
        inner_width.max(explicit_sum + SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH * auto_count as f32);
    let auto_width = if auto_count == 0 {
        0.0
    } else {
        (grid_width - explicit_sum) / auto_count as f32
    };
    specs
        .iter()
        .map(|spec| match spec {
            PageSimpleTableColumnWidthSpec::Explicit(width) => *width,
            PageSimpleTableColumnWidthSpec::Auto => auto_width,
        })
        .collect()
}
