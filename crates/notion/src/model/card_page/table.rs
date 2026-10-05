use std::collections::HashSet;

use serde::{Deserialize, Serialize};

mod cell;
mod resolution;

pub use cell::{
    CardPageSimpleTableCell, CardPageSimpleTableCellAddress, CardPageSimpleTableCellReadOnlyReason,
    CardPageSimpleTableCellRoundTrip, CardPageWritableSimpleTableCell,
};
pub(crate) use resolution::CardPageSimpleTableCellIndex;

const TABLE_COLUMN_SUBPIXELS_PER_PIXEL: f64 = 1024.0;
const TABLE_COLUMN_MAX_WIDTH_PIXELS: f64 = 65_535.0;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CardPageSimpleTableColumnId(String);

impl CardPageSimpleTableColumnId {
    pub fn new(value: String) -> Result<Self, String> {
        if value.trim().is_empty() {
            return Err("Notion table column ID cannot be blank".to_string());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CardPageSimpleTableColumnId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CardPageSimpleTableColumnId> for String {
    fn from(id: CardPageSimpleTableColumnId) -> Self {
        id.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "CardPageSimpleTableColumnWidthFields",
    into = "CardPageSimpleTableColumnWidthFields"
)]
pub struct CardPageSimpleTableColumnWidth(Option<u32>);

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "layout", rename_all = "snake_case")]
enum CardPageSimpleTableColumnWidthFields {
    Auto,
    Pixels { subpixels: u32 },
}

impl CardPageSimpleTableColumnWidth {
    pub const fn auto() -> Self {
        Self(None)
    }

    pub fn from_pixels(width: f64) -> Result<Self, String> {
        if !width.is_finite() || width <= 0.0 || width > TABLE_COLUMN_MAX_WIDTH_PIXELS {
            return Err(format!(
                "Notion table column width must be finite and in (0, {TABLE_COLUMN_MAX_WIDTH_PIXELS}], received {width}"
            ));
        }
        let subpixels = (width * TABLE_COLUMN_SUBPIXELS_PER_PIXEL).round();
        if subpixels < 1.0 || subpixels > f64::from(u32::MAX) {
            return Err(format!(
                "Notion table column width {width} cannot be represented at 1/1024px precision"
            ));
        }
        Ok(Self(Some(subpixels as u32)))
    }

    pub fn explicit_pixels(self) -> Option<f32> {
        self.0
            .map(|subpixels| (f64::from(subpixels) / TABLE_COLUMN_SUBPIXELS_PER_PIXEL) as f32)
    }
}

impl TryFrom<CardPageSimpleTableColumnWidthFields> for CardPageSimpleTableColumnWidth {
    type Error = String;

    fn try_from(fields: CardPageSimpleTableColumnWidthFields) -> Result<Self, Self::Error> {
        match fields {
            CardPageSimpleTableColumnWidthFields::Auto => Ok(Self::auto()),
            CardPageSimpleTableColumnWidthFields::Pixels { subpixels } => {
                let width = f64::from(subpixels) / TABLE_COLUMN_SUBPIXELS_PER_PIXEL;
                Self::from_pixels(width)
            }
        }
    }
}

impl From<CardPageSimpleTableColumnWidth> for CardPageSimpleTableColumnWidthFields {
    fn from(width: CardPageSimpleTableColumnWidth) -> Self {
        match width.0 {
            Some(subpixels) => Self::Pixels { subpixels },
            None => Self::Auto,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageSimpleTableColumn {
    id: CardPageSimpleTableColumnId,
    width: CardPageSimpleTableColumnWidth,
}

impl CardPageSimpleTableColumn {
    pub fn new(id: CardPageSimpleTableColumnId, width: CardPageSimpleTableColumnWidth) -> Self {
        Self { id, width }
    }

    pub fn id(&self) -> &CardPageSimpleTableColumnId {
        &self.id
    }

    pub const fn width(&self) -> CardPageSimpleTableColumnWidth {
        self.width
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    try_from = "CardPageSimpleTableBlockFields",
    into = "CardPageSimpleTableBlockFields"
)]
pub struct CardPageSimpleTableBlock {
    columns: Box<[CardPageSimpleTableColumn]>,
    row_block_ids: Box<[String]>,
    first_row_header: bool,
    first_column_header: bool,
}

#[derive(Serialize, Deserialize)]
struct CardPageSimpleTableBlockFields {
    columns: Vec<CardPageSimpleTableColumn>,
    row_block_ids: Vec<String>,
    first_row_header: bool,
    first_column_header: bool,
}

impl CardPageSimpleTableBlock {
    pub fn new(
        columns: Vec<CardPageSimpleTableColumn>,
        row_block_ids: Vec<String>,
        first_row_header: bool,
        first_column_header: bool,
    ) -> Result<Self, String> {
        if columns.is_empty() {
            return Err("Notion table must contain at least one column".to_string());
        }
        let mut column_ids = HashSet::with_capacity(columns.len());
        for column in &columns {
            if !column_ids.insert(column.id().as_str()) {
                return Err(format!(
                    "Notion table contains duplicate column {}",
                    column.id().as_str()
                ));
            }
        }
        if row_block_ids.is_empty() {
            return Err("Notion table must contain at least one row".to_string());
        }
        let mut unique_row_ids = HashSet::with_capacity(row_block_ids.len());
        for row_id in &row_block_ids {
            if row_id.trim().is_empty() {
                return Err("Notion table row ID cannot be blank".to_string());
            }
            if !unique_row_ids.insert(row_id.as_str()) {
                return Err(format!("Notion table contains duplicate row {row_id}"));
            }
        }
        Ok(Self {
            columns: columns.into_boxed_slice(),
            row_block_ids: row_block_ids.into_boxed_slice(),
            first_row_header,
            first_column_header,
        })
    }

    pub fn columns(&self) -> &[CardPageSimpleTableColumn] {
        &self.columns
    }

    pub fn row_block_ids(&self) -> &[String] {
        &self.row_block_ids
    }

    pub const fn first_row_header(&self) -> bool {
        self.first_row_header
    }

    pub const fn first_column_header(&self) -> bool {
        self.first_column_header
    }
}

impl TryFrom<CardPageSimpleTableBlockFields> for CardPageSimpleTableBlock {
    type Error = String;

    fn try_from(fields: CardPageSimpleTableBlockFields) -> Result<Self, Self::Error> {
        Self::new(
            fields.columns,
            fields.row_block_ids,
            fields.first_row_header,
            fields.first_column_header,
        )
    }
}

impl From<CardPageSimpleTableBlock> for CardPageSimpleTableBlockFields {
    fn from(table: CardPageSimpleTableBlock) -> Self {
        Self {
            columns: table.columns.into_vec(),
            row_block_ids: table.row_block_ids.into_vec(),
            first_row_header: table.first_row_header,
            first_column_header: table.first_column_header,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardPageSimpleTableRowBlock {
    cells: Box<[CardPageSimpleTableCell]>,
}

impl CardPageSimpleTableRowBlock {
    pub fn new(cells: Vec<CardPageSimpleTableCell>) -> Self {
        Self {
            cells: cells.into_boxed_slice(),
        }
    }

    pub fn cells(&self) -> &[CardPageSimpleTableCell] {
        &self.cells
    }

    pub(super) fn replace_cell(
        &mut self,
        column_index: usize,
        cell: CardPageSimpleTableCell,
    ) -> Result<(), String> {
        let target = self.cells.get_mut(column_index).ok_or_else(|| {
            format!("Notion table row has no cell at resolved column index {column_index}")
        })?;
        *target = cell;
        Ok(())
    }
}
