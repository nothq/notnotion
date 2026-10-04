use std::collections::{HashMap, HashSet};

use super::{NotionBlockRecord, PageMutationState};
use crate::live::board::simple_table_rich_text::simple_table_cell_round_trip;
use crate::model::{
    CardPageSimpleTableCellAddress, CardPageSimpleTableCellRoundTrip, CardPageSimpleTableColumnId,
};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(super) enum NotionSimpleTableRecord {
    Root {
        column_ids: Box<[CardPageSimpleTableColumnId]>,
    },
    Row {
        cell_round_trips: HashMap<CardPageSimpleTableColumnId, CardPageSimpleTableCellRoundTrip>,
    },
}

pub(in crate::live::board) struct ResolvedNotionSimpleTableCell<'a> {
    pub(in crate::live::board) row: &'a NotionBlockRecord,
}

impl PageMutationState {
    pub(in crate::live::board) fn simple_table_cell(
        &self,
        address: &CardPageSimpleTableCellAddress,
    ) -> Result<ResolvedNotionSimpleTableCell<'_>, String> {
        let table = self.block(address.table_block_id())?;
        let row = self.block(address.row_block_id())?;
        let NotionSimpleTableRecord::Root { column_ids } = table
            .simple_table
            .as_ref()
            .ok_or_else(|| address_error(address, "root is not a simple table"))?
        else {
            return Err(address_error(address, "root is a table row"));
        };
        let NotionSimpleTableRecord::Row { cell_round_trips } = row
            .simple_table
            .as_ref()
            .ok_or_else(|| address_error(address, "row is not a simple-table row"))?
        else {
            return Err(address_error(address, "row resolves to a table root"));
        };
        validate_row_ownership(table, row, address)?;
        if !column_ids.contains(address.column_id()) {
            return Err(address_error(address, "root does not declare the column"));
        }
        if let CardPageSimpleTableCellRoundTrip::ReadOnly(reason) = cell_round_trips
            .get(address.column_id())
            .copied()
            .unwrap_or(CardPageSimpleTableCellRoundTrip::Writable)
        {
            return Err(address_error(
                address,
                &format!("source rich text is read-only because it is {reason:?}"),
            ));
        }
        Ok(ResolvedNotionSimpleTableCell { row })
    }
}

pub(super) fn parse_simple_table_record(
    value: &Value,
    block_id: &str,
    block_type: &str,
) -> Result<Option<NotionSimpleTableRecord>, String> {
    match block_type {
        "table" => parse_table_root(value, block_id).map(Some),
        "table_row" => parse_table_row(value, block_id).map(Some),
        _ => Ok(None),
    }
}

fn parse_table_root(value: &Value, block_id: &str) -> Result<NotionSimpleTableRecord, String> {
    let order = value
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("table_block_column_order"))
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Notion table {block_id} has no column order"))?;
    let mut unique = HashSet::with_capacity(order.len());
    let mut column_ids = Vec::with_capacity(order.len());
    for (index, id) in order.iter().enumerate() {
        let id = id
            .as_str()
            .ok_or_else(|| format!("Notion table {block_id} column {index} has a non-string ID"))?;
        let id = CardPageSimpleTableColumnId::new(id.to_string())?;
        if !unique.insert(id.clone()) {
            return Err(format!(
                "Notion table {block_id} contains duplicate column {}",
                id.as_str()
            ));
        }
        column_ids.push(id);
    }
    if column_ids.is_empty() {
        return Err(format!("Notion table {block_id} contains no columns"));
    }
    Ok(NotionSimpleTableRecord::Root {
        column_ids: column_ids.into_boxed_slice(),
    })
}

fn parse_table_row(value: &Value, block_id: &str) -> Result<NotionSimpleTableRecord, String> {
    let properties = value
        .get("properties")
        .map(|properties| {
            properties
                .as_object()
                .ok_or_else(|| format!("Notion table row {block_id} has non-object properties"))
        })
        .transpose()?;
    let cell_round_trips = properties
        .into_iter()
        .flat_map(|properties| properties.iter())
        .map(|(column_id, value)| {
            Ok((
                CardPageSimpleTableColumnId::new(column_id.clone())?,
                simple_table_cell_round_trip(value),
            ))
        })
        .collect::<Result<HashMap<_, _>, String>>()?;
    Ok(NotionSimpleTableRecord::Row { cell_round_trips })
}

fn validate_row_ownership(
    table: &NotionBlockRecord,
    row: &NotionBlockRecord,
    address: &CardPageSimpleTableCellAddress,
) -> Result<(), String> {
    if row.parent_table != "block" || row.parent_id != table.id {
        return Err(address_error(
            address,
            "table does not directly own the row",
        ));
    }
    let occurrences = table
        .content_ids
        .iter()
        .filter(|id| id.as_str() == row.id.as_str())
        .count();
    if occurrences != 1 {
        return Err(address_error(
            address,
            "table does not declare the row exactly once",
        ));
    }
    Ok(())
}

fn address_error(address: &CardPageSimpleTableCellAddress, detail: &str) -> String {
    format!(
        "Notion table {} {detail} for row {} column {}",
        address.table_block_id(),
        address.row_block_id(),
        address.column_id().as_str()
    )
}
