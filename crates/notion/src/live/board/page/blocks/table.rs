use std::collections::HashSet;

use crate::model::{
    CardPageSimpleTableBlock, CardPageSimpleTableCell, CardPageSimpleTableColumn,
    CardPageSimpleTableColumnId, CardPageSimpleTableColumnWidth, CardPageSimpleTableRowBlock,
};

use super::super::{parse_card_page_table_cell, Map, PageBlockParseContext, Value};
use super::ParsedPageBlocks;

pub(super) fn page_simple_table_block(
    block_id: &str,
    block: &Value,
) -> Result<CardPageSimpleTableBlock, String> {
    let format = block
        .get("format")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Notion table {block_id} has no object format"))?;
    let columns = simple_table_columns(block_id, format)?;
    let row_block_ids = table_row_ids(block_id, block)?;
    CardPageSimpleTableBlock::new(
        columns,
        row_block_ids,
        optional_header_flag(block_id, format, "table_block_column_header")?,
        optional_header_flag(block_id, format, "table_block_row_header")?,
    )
    .map_err(|error| format!("Notion table {block_id}: {error}"))
}

fn simple_table_columns(
    block_id: &str,
    format: &Map<String, Value>,
) -> Result<Vec<CardPageSimpleTableColumn>, String> {
    let column_order = format
        .get("table_block_column_order")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Notion table {block_id} has no column order"))?;
    let column_format = format
        .get("table_block_column_format")
        .map(|column_format| {
            column_format
                .as_object()
                .ok_or_else(|| format!("Notion table {block_id} has non-object column format"))
        })
        .transpose()?;
    let mut columns = Vec::with_capacity(column_order.len());
    let mut column_ids = HashSet::with_capacity(column_order.len());
    for (column_index, raw_id) in column_order.iter().enumerate() {
        let raw_id = raw_id.as_str().ok_or_else(|| {
            format!("Notion table {block_id} column {column_index} has a non-string ID")
        })?;
        if !column_ids.insert(raw_id) {
            return Err(format!(
                "Notion table {block_id} contains duplicate column {raw_id}",
            ));
        }
        let id = CardPageSimpleTableColumnId::new(raw_id.to_string())
            .map_err(|error| format!("Notion table {block_id}: {error}"))?;
        let width = simple_table_column_width(block_id, &id, column_format)?;
        columns.push(CardPageSimpleTableColumn::new(id, width));
    }
    Ok(columns)
}

fn simple_table_column_width(
    block_id: &str,
    column_id: &CardPageSimpleTableColumnId,
    column_format: Option<&Map<String, Value>>,
) -> Result<CardPageSimpleTableColumnWidth, String> {
    let Some(format) = column_format.and_then(|format| format.get(column_id.as_str())) else {
        return Ok(CardPageSimpleTableColumnWidth::auto());
    };
    let format = format.as_object().ok_or_else(|| {
        format!(
            "Notion table {block_id} column {} has non-object format",
            column_id.as_str()
        )
    })?;
    let Some(width) = format.get("width") else {
        return Ok(CardPageSimpleTableColumnWidth::auto());
    };
    let width = width.as_f64().ok_or_else(|| {
        format!(
            "Notion table {block_id} column {} has non-numeric width",
            column_id.as_str()
        )
    })?;
    CardPageSimpleTableColumnWidth::from_pixels(width).map_err(|error| {
        format!(
            "Notion table {block_id} column {} has an invalid width: {error}",
            column_id.as_str()
        )
    })
}

pub(super) struct SimpleTableRowInput<'a> {
    pub(super) table_id: &'a str,
    pub(super) table: &'a CardPageSimpleTableBlock,
    pub(super) row_id: &'a str,
    pub(super) row: &'a Value,
}

pub(super) fn page_simple_table_row_block(
    context: &PageBlockParseContext<'_>,
    input: SimpleTableRowInput<'_>,
    parsed: &mut ParsedPageBlocks,
) -> Result<CardPageSimpleTableRowBlock, String> {
    let SimpleTableRowInput {
        table_id,
        table,
        row_id,
        row,
    } = input;
    validate_table_row_content(row_id, row)?;
    let properties = row
        .get("properties")
        .map(|properties| {
            properties
                .as_object()
                .ok_or_else(|| format!("Notion table row {row_id} has non-object properties"))
        })
        .transpose()?;
    let lookup = context.property_lookup();
    let mut cells = Vec::with_capacity(table.columns().len());
    for column in table.columns() {
        let cell = properties
            .and_then(|properties| properties.get(column.id().as_str()))
            .map(|value| parse_card_page_table_cell(value, lookup, &mut parsed.page_title_cache))
            .transpose()
            .map_err(|error| {
                format!(
                    "Notion table {table_id} row {row_id} column {}: {error}",
                    column.id().as_str()
                )
            })?
            .unwrap_or(CardPageSimpleTableCell::writable(
                String::new(),
                Vec::new(),
            )?);
        cells.push(cell);
    }
    Ok(CardPageSimpleTableRowBlock::new(cells))
}

fn table_row_ids(block_id: &str, block: &Value) -> Result<Vec<String>, String> {
    let rows = block
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Notion table {block_id} has no row content"))?;
    rows.iter()
        .enumerate()
        .map(|(row_index, row_id)| {
            row_id
                .as_str()
                .filter(|row_id| !row_id.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    format!("Notion table {block_id} row {row_index} has an invalid block ID")
                })
        })
        .collect()
}

fn optional_header_flag(
    block_id: &str,
    format: &Map<String, Value>,
    field: &str,
) -> Result<bool, String> {
    match format.get(field) {
        None => Ok(false),
        Some(value) => value
            .as_bool()
            .ok_or_else(|| format!("Notion table {block_id} has non-boolean {field}")),
    }
}

fn validate_table_row_content(row_id: &str, row: &Value) -> Result<(), String> {
    match row.get("content") {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Array(children)) if children.is_empty() => Ok(()),
        Some(Value::Array(_)) => Err(format!(
            "Notion table row {row_id} unexpectedly contains child content"
        )),
        Some(_) => Err(format!(
            "Notion table row {row_id} contains non-array child content"
        )),
    }
}
