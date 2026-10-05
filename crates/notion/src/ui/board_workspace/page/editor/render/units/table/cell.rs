use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Role, Stateful, StatefulInteractiveElement, Styled,
};

use crate::ui::surface::PageSimpleTableCellFocusMode;
use crate::ui::{
    div, page_block_background, px, rgba, AnyElement, CardPageSimpleTableColumn, Div,
    FluentBuilder, LoadedCardPageSimpleTableCell, LoadedCardPageSimpleTableCellAccess,
};

use super::super::super::{PageBlockRenderer, PageRenderAction, PageTableRenderAction};
use super::{
    PageSimpleTableRowState, SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH, SIMPLE_TABLE_CELL_PADDING_X,
    SIMPLE_TABLE_CELL_PADDING_Y, SIMPLE_TABLE_FONT_SIZE, SIMPLE_TABLE_LINE_HEIGHT,
    SIMPLE_TABLE_ROW_MIN_HEIGHT,
};
use crate::ui::board_workspace::page::editor::rich_text::{
    page_static_rich_text, PageStaticTextFont,
};
use crate::ui::board_workspace::page::editor::simple_table::{
    PageSimpleTableCellInputRequest, PageSimpleTableColumnWidthSpec, PageSimpleTableEditAction,
};

/// A table cell with its column and that column's index.
#[derive(Clone, Copy)]
pub(super) struct PageSimpleTableCellSlot<'a> {
    pub(super) column_index: usize,
    pub(super) column: &'a CardPageSimpleTableColumn,
    pub(super) cell: &'a LoadedCardPageSimpleTableCell,
}

/// The mounted cell editor's generation and its text input.
type ActiveTableCellInput = (
    crate::ui::surface::PageSimpleTableCellGeneration,
    gpui::Entity<gpui_components::text_input::TextInput>,
);

impl PageBlockRenderer {
    pub(super) fn render_page_simple_table_cell(
        &self,
        state: &PageSimpleTableRowState<'_>,
        slot: PageSimpleTableCellSlot<'_>,
        cx: &mut App,
    ) -> Stateful<Div> {
        let PageSimpleTableCellSlot {
            column_index,
            column,
            cell,
        } = slot;
        let header = (state.ordinal == 0 && state.table.first_row_header())
            || (column_index == 0 && state.table.first_column_header());
        let active_input = self.page_simple_table_cell_active_input(state, cell, header, cx);
        let active_generation = active_input.as_ref().map(|(generation, _)| *generation);
        let content: AnyElement = active_input
            .map(|(_, input)| input.into_any_element())
            .unwrap_or_else(|| {
                self.render_static_simple_table_cell(cell, state, header)
                    .into_any_element()
            });
        let frame = simple_table_cell_frame(self.theme, self.appearance_mode, state, slot, header);
        let frame = self.simple_table_cell_mount_observer(frame, state, slot, active_generation);
        let rendered = frame
            .id(simple_table_cell_id(state, column))
            .role(Role::GridCell)
            .when(active_generation.is_none(), |cell_frame| {
                cell_frame
                    .px(px(SIMPLE_TABLE_CELL_PADDING_X))
                    .py(px(SIMPLE_TABLE_CELL_PADDING_Y))
            })
            .child(content);
        self.simple_table_cell_activation(rendered, state, cell, active_generation.is_none())
    }

    fn page_simple_table_cell_active_input(
        &self,
        state: &PageSimpleTableRowState<'_>,
        cell: &LoadedCardPageSimpleTableCell,
        header: bool,
        cx: &mut App,
    ) -> Option<ActiveTableCellInput> {
        let generation = self
            .tables
            .editor()
            .borrow()
            .as_ref()
            .filter(|active| active.page_id == state.page_id && active.address == cell.address)
            .map(|active| active.generation)?;
        let input = self.page_simple_table_cell_input_entity(
            PageSimpleTableCellInputRequest {
                page_id: state.page_id,
                address: &cell.address,
                color: state.color,
                header,
            },
            cx,
        );
        if input.is_none()
            && !self
                .tables
                .editor()
                .retains_local_composition(&cell.address, generation)
        {
            self.tables.editor().clear();
        }
        input.map(|input| (generation, input))
    }

    fn render_static_simple_table_cell(
        &self,
        cell: &LoadedCardPageSimpleTableCell,
        state: &PageSimpleTableRowState<'_>,
        header: bool,
    ) -> gpui::StyledText {
        page_static_rich_text(
            cell.text.clone(),
            &cell.annotation_runs,
            self.appearance_mode,
            state.color,
            PageStaticTextFont {
                size: SIMPLE_TABLE_FONT_SIZE,
                line_height: SIMPLE_TABLE_LINE_HEIGHT,
                weight: if header {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                },
            },
        )
    }

    fn simple_table_cell_activation(
        &self,
        rendered: Stateful<Div>,
        state: &PageSimpleTableRowState<'_>,
        cell: &LoadedCardPageSimpleTableCell,
        inactive: bool,
    ) -> Stateful<Div> {
        if !inactive || cell.access != LoadedCardPageSimpleTableCellAccess::Writable {
            return rendered;
        }
        let page_id = state.page_id.to_owned();
        let address = cell.address.clone();
        let actions = self.actions.clone();
        rendered.cursor_text().on_mouse_down(
            MouseButton::Left,
            move |event: &MouseDownEvent, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
                actions.emit(
                    PageRenderAction::Table(PageTableRenderAction::Edit(
                        PageSimpleTableEditAction::Activate {
                            page_id: page_id.clone(),
                            address: address.clone(),
                            focus: PageSimpleTableCellFocusMode::Pointer(event.position),
                        },
                    )),
                    window,
                    cx,
                );
            },
        )
    }

    fn simple_table_cell_mount_observer(
        &self,
        rendered: Div,
        state: &PageSimpleTableRowState<'_>,
        slot: PageSimpleTableCellSlot<'_>,
        generation: Option<crate::ui::surface::PageSimpleTableCellGeneration>,
    ) -> Div {
        let PageSimpleTableCellSlot {
            column_index, cell, ..
        } = slot;
        let Some(generation) = generation else {
            return rendered;
        };
        let address = cell.address.clone();
        let handle = state.scroll_handle.clone();
        let widths = state
            .table
            .columns()
            .iter()
            .map(|column| match column.width().explicit_pixels() {
                Some(width) => PageSimpleTableColumnWidthSpec::Explicit(width),
                None => PageSimpleTableColumnWidthSpec::Auto,
            })
            .collect::<Vec<_>>();
        let actions = self.actions.clone();
        let flow = self.flow.clone();
        let tables = self.tables.clone();
        let flow_mount = state.flow_mount.clone();
        rendered.on_children_prepainted(move |_, window, cx| {
            if let Some(flow_mount) = &flow_mount {
                if !flow.committed_focus().as_ref().is_some_and(|focus| {
                    focus.authority.matches(&flow_mount.observation)
                        && focus.target
                            == crate::ui::surface::PageFlowPinTarget::Unit(
                                flow_mount.unit_key.clone(),
                            )
                }) {
                    return;
                }
            }
            actions.emit(
                PageRenderAction::Table(PageTableRenderAction::CompleteMount {
                    address: address.clone(),
                    generation,
                    scroll_handle: handle.clone(),
                    column_widths: widths.clone(),
                    column_index,
                }),
                window,
                cx,
            );
            if flow_mount.is_some() && !tables.editor().pending_focus_matches(&address, generation)
            {
                flow.set_committed_focus(None);
            }
        })
    }
}

fn simple_table_cell_frame(
    theme: crate::ui::Theme,
    appearance_mode: crate::ui::AppearanceMode,
    state: &PageSimpleTableRowState<'_>,
    slot: PageSimpleTableCellSlot<'_>,
    header: bool,
) -> Div {
    let PageSimpleTableCellSlot {
        column_index,
        column,
        ..
    } = slot;
    let frame = div()
        .min_h(px(SIMPLE_TABLE_ROW_MIN_HEIGHT))
        .overflow_hidden()
        .border_r_1()
        .border_b_1()
        .when(column_index == 0, |cell| cell.border_l_1())
        .when(state.position.is_first(), |cell| cell.border_t_1())
        .border_color(rgba(theme.surface_border))
        .when(header, |cell| cell.bg(rgba(theme.page_icon_bg)))
        .when_some(
            (!header)
                .then(|| page_block_background(state.color, appearance_mode))
                .flatten(),
            |cell, background| cell.bg(background),
        );
    match column.width().explicit_pixels() {
        Some(width) => frame.w(px(width)).min_w(px(width)).flex_none(),
        None => frame
            .flex_basis(px(0.0))
            .flex_grow(1.0)
            .min_w(px(SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH)),
    }
}

fn simple_table_cell_id(
    state: &PageSimpleTableRowState<'_>,
    column: &CardPageSimpleTableColumn,
) -> ElementId {
    ElementId::Name(
        format!(
            "notion-page-simple-table-cell-{}-{}-{}",
            state.table_block.block_id,
            state.row_block_id,
            column.id().as_str()
        )
        .into(),
    )
}
