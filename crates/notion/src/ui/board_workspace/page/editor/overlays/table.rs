use gpui::prelude::FluentBuilder;
use gpui::{div, px, Div, FontWeight, ParentElement, Styled};

use crate::model::{CardPageSimpleTableBlock, CardPageSimpleTableColumn};
use crate::ui::{
    page_block_background, rgba, AppearanceMode, CardPageBlockColor, LoadedCardPageSimpleTableCell,
    Theme,
};

use super::super::{
    rich_text::{page_static_rich_text, PageStaticTextFont},
    PageBlockDragPreviewRow, SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH,
};

#[derive(Clone, Copy)]
struct SimpleTablePreviewCellPosition {
    row_index: usize,
    column_index: usize,
}

#[derive(Clone, Copy)]
struct SimpleTablePreviewStyle {
    theme: Theme,
    appearance_mode: AppearanceMode,
    block_color: CardPageBlockColor,
}

impl PageBlockDragPreviewRow {
    pub(super) fn render_simple_table_content(
        &self,
        table: &CardPageSimpleTableBlock,
        theme: Theme,
        appearance_mode: AppearanceMode,
    ) -> Div {
        let style = SimpleTablePreviewStyle {
            theme,
            appearance_mode,
            block_color: self.color,
        };
        div()
            .max_w(px(512.0))
            .p(px(8.0))
            .overflow_hidden()
            .flex()
            .flex_col()
            .children(
                self.simple_table_rows
                    .iter()
                    .enumerate()
                    .map(|(row_index, cells)| {
                        render_simple_table_preview_row(table, row_index, cells, style)
                    }),
            )
    }
}

fn render_simple_table_preview_row(
    table: &CardPageSimpleTableBlock,
    row_index: usize,
    cells: &[LoadedCardPageSimpleTableCell],
    style: SimpleTablePreviewStyle,
) -> Div {
    div()
        .flex()
        .children(table.columns().iter().zip(cells).take(3).enumerate().map(
            |(column_index, (column, cell))| {
                render_simple_table_preview_cell(
                    table,
                    SimpleTablePreviewCellPosition {
                        row_index,
                        column_index,
                    },
                    column,
                    cell,
                    style,
                )
            },
        ))
}

fn render_simple_table_preview_cell(
    table: &CardPageSimpleTableBlock,
    position: SimpleTablePreviewCellPosition,
    column: &CardPageSimpleTableColumn,
    cell: &LoadedCardPageSimpleTableCell,
    style: SimpleTablePreviewStyle,
) -> Div {
    let SimpleTablePreviewCellPosition {
        row_index,
        column_index,
    } = position;
    let header = (row_index == 0 && table.first_row_header())
        || (column_index == 0 && table.first_column_header());
    let text = page_static_rich_text(
        cell.text.clone(),
        &cell.annotation_runs,
        style.appearance_mode,
        style.block_color,
        PageStaticTextFont {
            size: 14.0,
            line_height: 20.0,
            weight: if header {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            },
        },
    );
    div()
        .w(px(column
            .width()
            .explicit_pixels()
            .unwrap_or(SIMPLE_TABLE_AUTO_COLUMN_MIN_WIDTH)
            .min(160.0)))
        .min_w(px(0.0))
        .min_h(px(35.0))
        .flex_none()
        .overflow_hidden()
        .border_r_1()
        .border_b_1()
        .when(column_index == 0, |cell| cell.border_l_1())
        .when(row_index == 0, |cell| cell.border_t_1())
        .border_color(rgba(style.theme.surface_border))
        .when(header, |cell| cell.bg(rgba(style.theme.page_icon_bg)))
        .when_some(
            (!header)
                .then(|| page_block_background(style.block_color, style.appearance_mode))
                .flatten(),
            |cell, background| cell.bg(background),
        )
        .px(px(9.0))
        .py(px(7.0))
        .child(text)
}
