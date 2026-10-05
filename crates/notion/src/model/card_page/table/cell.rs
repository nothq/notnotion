use serde::{Deserialize, Serialize};

use super::super::CardPageTextAnnotationSpan;
use super::CardPageSimpleTableColumnId;
use crate::model::PageTextAnnotation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageSimpleTableCellReadOnlyReason {
    SemanticToken,
    UnknownAnnotation,
    UnsupportedShape,
    UnverifiedLegacyValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardPageSimpleTableCellRoundTrip {
    Writable,
    ReadOnly(CardPageSimpleTableCellReadOnlyReason),
}

impl Default for CardPageSimpleTableCellRoundTrip {
    fn default() -> Self {
        Self::ReadOnly(CardPageSimpleTableCellReadOnlyReason::UnverifiedLegacyValue)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "CardPageSimpleTableCellFields")]
pub struct CardPageSimpleTableCell {
    text: String,
    annotations: Vec<CardPageTextAnnotationSpan>,
    #[serde(default)]
    round_trip: CardPageSimpleTableCellRoundTrip,
}

#[derive(Deserialize)]
struct CardPageSimpleTableCellFields {
    text: String,
    annotations: Vec<CardPageTextAnnotationSpan>,
    #[serde(default)]
    round_trip: CardPageSimpleTableCellRoundTrip,
}

impl CardPageSimpleTableCell {
    pub fn writable(
        text: String,
        annotations: Vec<CardPageTextAnnotationSpan>,
    ) -> Result<Self, String> {
        validate_annotation_spans(&text, &annotations)?;
        Ok(Self {
            text,
            annotations,
            round_trip: CardPageSimpleTableCellRoundTrip::Writable,
        })
    }

    pub(crate) fn read_only(
        text: String,
        annotations: Vec<CardPageTextAnnotationSpan>,
        reason: CardPageSimpleTableCellReadOnlyReason,
    ) -> Result<Self, String> {
        validate_annotation_spans(&text, &annotations)?;
        Ok(Self {
            text,
            annotations,
            round_trip: CardPageSimpleTableCellRoundTrip::ReadOnly(reason),
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn annotations(&self) -> &[CardPageTextAnnotationSpan] {
        &self.annotations
    }

    pub const fn round_trip(&self) -> CardPageSimpleTableCellRoundTrip {
        self.round_trip
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardPageWritableSimpleTableCell(CardPageSimpleTableCell);

impl CardPageWritableSimpleTableCell {
    pub fn new(text: String, annotations: Vec<CardPageTextAnnotationSpan>) -> Result<Self, String> {
        Ok(Self(CardPageSimpleTableCell::writable(text, annotations)?))
    }

    pub fn as_cell(&self) -> &CardPageSimpleTableCell {
        &self.0
    }

    pub fn into_cell(self) -> CardPageSimpleTableCell {
        self.0
    }
}

impl TryFrom<CardPageSimpleTableCell> for CardPageWritableSimpleTableCell {
    type Error = String;

    fn try_from(cell: CardPageSimpleTableCell) -> Result<Self, Self::Error> {
        match cell.round_trip {
            CardPageSimpleTableCellRoundTrip::Writable => Ok(Self(cell)),
            CardPageSimpleTableCellRoundTrip::ReadOnly(reason) => Err(format!(
                "Notion table cell is read-only because its rich text is {reason:?}"
            )),
        }
    }
}

impl TryFrom<CardPageSimpleTableCellFields> for CardPageSimpleTableCell {
    type Error = String;

    fn try_from(fields: CardPageSimpleTableCellFields) -> Result<Self, Self::Error> {
        validate_annotation_spans(&fields.text, &fields.annotations)?;
        Ok(Self {
            text: fields.text,
            annotations: fields.annotations,
            round_trip: fields.round_trip,
        })
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CardPageSimpleTableCellAddress {
    table_block_id: String,
    row_block_id: String,
    column_id: CardPageSimpleTableColumnId,
}

impl CardPageSimpleTableCellAddress {
    pub fn new(
        table_block_id: impl Into<String>,
        row_block_id: impl Into<String>,
        column_id: CardPageSimpleTableColumnId,
    ) -> Result<Self, String> {
        let table_block_id = table_block_id.into();
        let row_block_id = row_block_id.into();
        if table_block_id.trim().is_empty() || row_block_id.trim().is_empty() {
            return Err("a Notion table-cell address requires table and row blocks".to_string());
        }
        if table_block_id == row_block_id {
            return Err("a Notion table row cannot be its table root".to_string());
        }
        Ok(Self {
            table_block_id,
            row_block_id,
            column_id,
        })
    }

    pub fn table_block_id(&self) -> &str {
        &self.table_block_id
    }

    pub fn row_block_id(&self) -> &str {
        &self.row_block_id
    }

    pub fn column_id(&self) -> &CardPageSimpleTableColumnId {
        &self.column_id
    }
}

fn validate_annotation_spans(
    text: &str,
    annotations: &[CardPageTextAnnotationSpan],
) -> Result<(), String> {
    for span in annotations {
        if matches!(&span.annotation, PageTextAnnotation::Link(url) if url.trim().is_empty()) {
            return Err("a Notion table-cell link annotation requires a URL".to_string());
        }
        CardPageTextAnnotationSpan::new(
            text,
            span.start_utf8,
            span.end_utf8,
            span.annotation.clone(),
        )?;
    }
    Ok(())
}
