use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, Role, Stateful,
    StatefulInteractiveElement, Styled,
};

use crate::ui::{
    alpha, div, px, Arc, CardPageBlockColor, CardPageBlockColorValue, CardPageSimpleTableBlock,
    Div, FluentBuilder, LoadedCardPageData, LoadedCardPageDocumentUnit,
    LoadedCardPageSimpleTableCell, LoadedCardPageSimpleTableRowPosition, PageBlockDragScrollTarget,
    ScrollHandle,
};

use super::super::super::support::PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT;
use super::super::super::{
    PageBlockDragSelection, PAGE_BLOCK_GUTTER_WIDTH, PAGE_BLOCK_INDENT,
    SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH,
};
use super::super::row::PageBlockRenderContext;
use super::super::PageBlockRenderer;

mod cell;

use cell::PageSimpleTableCellSlot;

const SIMPLE_TABLE_OUTER_SPACING: f32 = 8.0;
const SIMPLE_TABLE_ROW_MIN_HEIGHT: f32 = 35.0;
const SIMPLE_TABLE_CELL_PADDING_X: f32 = 9.0;
const SIMPLE_TABLE_CELL_PADDING_Y: f32 = 7.0;
const SIMPLE_TABLE_FONT_SIZE: f32 = 14.0;
const SIMPLE_TABLE_LINE_HEIGHT: f32 = 20.0;

pub(super) struct PageSimpleTableRowRender<'a> {
    pub(super) data: &'a Arc<LoadedCardPageData>,
    pub(super) document_unit_index: usize,
    pub(super) drag_selection: &'a Arc<PageBlockDragSelection>,
    pub(super) scroll_target: PageBlockDragScrollTarget,
    pub(super) inherited_text_color: Option<CardPageBlockColorValue>,
    pub(super) render_depth: usize,
    pub(super) nesting_offset: f32,
    pub(super) render_gutter: bool,
    pub(super) flow_observation: Option<&'a crate::ui::surface::PageFlowObservationToken>,
}

struct PageSimpleTableRowState<'a> {
    page_id: &'a str,
    owner_block_index: usize,
    table_block: &'a crate::ui::CardPageBlock,
    table: &'a CardPageSimpleTableBlock,
    row_block_id: &'a str,
    ordinal: usize,
    position: LoadedCardPageSimpleTableRowPosition,
    cells: &'a [LoadedCardPageSimpleTableCell],
    color: CardPageBlockColor,
    selected: bool,
    show_controls: bool,
    scroll_handle: ScrollHandle,
    grid_min_width: f32,
    flow_mount: Option<PageSimpleTableFlowMount>,
}

#[derive(Clone)]
struct PageSimpleTableFlowMount {
    observation: crate::ui::surface::PageFlowObservationToken,
    unit_key: crate::ui::PageDocumentUnitKey,
}

struct PageSimpleTableRowInteraction {
    color: CardPageBlockColor,
    selected: bool,
    show_controls: bool,
    scroll_handle: ScrollHandle,
}

impl PageBlockRenderer {
    pub(super) fn render_page_simple_table_row(
        &self,
        render: PageSimpleTableRowRender<'_>,
        cx: &mut App,
    ) -> crate::ui::AnyElement {
        let state = self.page_simple_table_row_state(&render);
        let grid = self.render_page_simple_table_grid(&state, cx);
        let viewport = self.render_page_simple_table_viewport(&state, grid);
        self.render_page_simple_table_frame(&render, &state, viewport, cx)
    }
}

impl PageBlockRenderer {
    fn page_simple_table_row_state<'a>(
        &self,
        render: &PageSimpleTableRowRender<'a>,
    ) -> PageSimpleTableRowState<'a> {
        let unit = &render.data.document_units[render.document_unit_index];
        let LoadedCardPageDocumentUnit::SimpleTableRow {
            owner_visible_row_index,
            row_block_index,
            ordinal,
            position,
            cells,
            ..
        } = unit
        else {
            unreachable!("simple-table renderer requires a table-row document unit");
        };
        let owner_block_index = render.data.visible_rows[*owner_visible_row_index].block_index;
        let table_block = &render.data.page.blocks[owner_block_index];
        let table = table_block
            .simple_table_content()
            .expect("simple-table document-unit owner must contain a table");
        assert_eq!(
            table.columns().len(),
            cells.len(),
            "validated Notion table row must align with its ordered columns"
        );
        let row_block_id = render.data.page.blocks[*row_block_index].block_id.as_str();
        let interaction =
            self.page_simple_table_row_interaction(render, table_block, position.is_first());
        let grid_min_width = page_simple_table_grid_min_width(table);
        PageSimpleTableRowState {
            page_id: &render.data.page.block_id,
            owner_block_index,
            table_block,
            table,
            row_block_id,
            ordinal: *ordinal,
            position: *position,
            cells,
            color: interaction.color,
            selected: interaction.selected,
            show_controls: interaction.show_controls,
            scroll_handle: interaction.scroll_handle,
            grid_min_width,
            flow_mount: render
                .flow_observation
                .map(|observation| PageSimpleTableFlowMount {
                    observation: observation.clone(),
                    unit_key: unit.key().clone(),
                }),
        }
    }
}

impl PageBlockRenderer {
    fn page_simple_table_row_interaction(
        &self,
        render: &PageSimpleTableRowRender<'_>,
        table_block: &crate::ui::CardPageBlock,
        first: bool,
    ) -> PageSimpleTableRowInteraction {
        let color = simple_table_render_color(table_block.color, render.inherited_text_color);
        let selected = self
            .interaction
            .block_selected_without_drag(&table_block.block_id);
        let show_controls = first
            && (self.interaction.hovered_block.as_deref() == Some(table_block.block_id.as_str())
                || self.interaction.block_menu_is_open(&table_block.block_id));
        let scroll_handle = self
            .tables
            .scroll_handle(&render.data.page.block_id, &table_block.block_id);
        PageSimpleTableRowInteraction {
            color,
            selected,
            show_controls,
            scroll_handle,
        }
    }
}

impl PageBlockRenderer {
    fn render_page_simple_table_grid(
        &self,
        state: &PageSimpleTableRowState<'_>,
        cx: &mut App,
    ) -> Stateful<Div> {
        div()
            .id(ElementId::Name(
                format!(
                    "notion-page-simple-table-grid-row-{}-{}",
                    state.table_block.block_id, state.row_block_id
                )
                .into(),
            ))
            .w_full()
            .min_w(px(state.grid_min_width))
            .min_h(px(SIMPLE_TABLE_ROW_MIN_HEIGHT))
            .flex()
            .items_stretch()
            .role(Role::Row)
            .children(
                state
                    .table
                    .columns()
                    .iter()
                    .zip(state.cells)
                    .enumerate()
                    .map(|(index, (column, cell))| {
                        self.render_page_simple_table_cell(
                            state,
                            PageSimpleTableCellSlot {
                                column_index: index,
                                column,
                                cell,
                            },
                            cx,
                        )
                    }),
            )
    }

    fn render_page_simple_table_viewport(
        &self,
        state: &PageSimpleTableRowState<'_>,
        grid: Stateful<Div>,
    ) -> Div {
        let scrolling_grid = div()
            .id(ElementId::Name(
                format!(
                    "notion-simple-table-scroll-{}-{}",
                    state.table_block.block_id, state.row_block_id
                )
                .into(),
            ))
            .relative()
            .w_full()
            .overflow_x_scroll()
            .role(Role::Grid)
            .track_scroll(&state.scroll_handle)
            .child(
                div()
                    .px(px(SIMPLE_TABLE_OUTER_SPACING))
                    .when(state.position.is_first(), |row| {
                        row.pt(px(SIMPLE_TABLE_OUTER_SPACING))
                    })
                    .when(state.position.is_last(), |row| {
                        row.pb(px(SIMPLE_TABLE_OUTER_SPACING))
                    })
                    .w_full()
                    .min_w(px(state.grid_min_width + SIMPLE_TABLE_OUTER_SPACING * 2.0))
                    .child(grid),
            );
        div()
            .relative()
            .w_full()
            .child(scrolling_grid)
            .when(state.selected, |viewport| {
                viewport.child(div().absolute().inset_0().bg(alpha(0x2383e2, 0.14)))
            })
    }

    fn render_page_simple_table_frame(
        &self,
        render: &PageSimpleTableRowRender<'_>,
        state: &PageSimpleTableRowState<'_>,
        viewport: Div,
        cx: &mut App,
    ) -> crate::ui::AnyElement {
        let first = state.position.is_first();
        let context = page_simple_table_render_context(render, state);
        let body = div()
            .relative()
            .ml(px(PAGE_BLOCK_GUTTER_WIDTH))
            .min_w(px(0.0))
            .flex_grow(1.0)
            .child(viewport)
            .when(
                first
                    && self
                        .interaction
                        .block_menu_is_open(&state.table_block.block_id),
                |body| {
                    body.child(self.render_page_block_context_menu(
                        state.table_block,
                        PAGE_BLOCK_GUTTER_LAYOUT_HEIGHT / 2.0,
                        cx,
                    ))
                },
            );
        div()
            .id(ElementId::Name(
                format!(
                    "notion-page-simple-table-row-{}-{}",
                    state.table_block.block_id, state.row_block_id
                )
                .into(),
            ))
            .relative()
            .flex()
            .top(px(-render.nesting_offset))
            .ml(px(
                render.render_depth as f32 * PAGE_BLOCK_INDENT - PAGE_BLOCK_GUTTER_WIDTH
            ))
            .when(context.render_gutter, |row| {
                row.child(self.render_page_block_gutter(&context, state.show_controls, cx))
            })
            .child(body)
            .into_any_element()
    }
}

fn page_simple_table_grid_min_width(table: &crate::model::CardPageSimpleTableBlock) -> f32 {
    table
        .columns()
        .iter()
        .map(|column| {
            column
                .width()
                .explicit_pixels()
                .unwrap_or(SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH)
        })
        .sum()
}

fn page_simple_table_render_context<'a>(
    render: &PageSimpleTableRowRender<'a>,
    state: &PageSimpleTableRowState<'a>,
) -> PageBlockRenderContext<'a> {
    let first = state.position.is_first();
    PageBlockRenderContext {
        data: render.data,
        index: state.owner_block_index,
        block: state.table_block,
        drag_selection: render.drag_selection,
        scroll_target: render.scroll_target,
        inherited_text_color: render.inherited_text_color,
        render_depth: render.render_depth,
        nesting_offset: render.nesting_offset,
        row_spacing: None,
        render_gutter: render.render_gutter && first,
        gutter_center: Some(
            SIMPLE_TABLE_ROW_MIN_HEIGHT / 2.0
                + if first {
                    SIMPLE_TABLE_OUTER_SPACING
                } else {
                    0.0
                },
        ),
        observation: render
            .flow_observation
            .expect("simple-table rows require a document-flow observation"),
    }
}

fn simple_table_render_color(
    color: CardPageBlockColor,
    inherited_text_color: Option<CardPageBlockColorValue>,
) -> CardPageBlockColor {
    match (color, inherited_text_color) {
        (
            CardPageBlockColor::Text(CardPageBlockColorValue::Default),
            Some(inherited_text_color),
        ) => CardPageBlockColor::text(inherited_text_color),
        _ => color,
    }
}
