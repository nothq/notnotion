use serde::Serialize;
use serde_json::Value;

use super::super::text::annotation_tuple;
use crate::model::{
    CardPageSimpleTableColumnId, CardPageTextAnnotationSpan, CardPageWritableSimpleTableCell,
};

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub(in crate::live::board::page_mutation) struct SimpleTableCellPropertyPath([String; 2]);

impl SimpleTableCellPropertyPath {
    pub(in crate::live::board::page_mutation) fn new(
        column_id: &CardPageSimpleTableColumnId,
    ) -> Self {
        Self(["properties".to_string(), column_id.as_str().to_string()])
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::live::board::page_mutation) struct SimpleTableCellPropertyValueArgs {
    primitive_op: SimpleTableCellSetOperation,
}

impl SimpleTableCellPropertyValueArgs {
    pub(in crate::live::board::page_mutation) fn new(
        cell: &CardPageWritableSimpleTableCell,
    ) -> Self {
        Self {
            primitive_op: SimpleTableCellSetOperation {
                command: "set",
                args: SimpleTableRichText::new(cell),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct SimpleTableCellSetOperation {
    command: &'static str,
    args: SimpleTableRichText,
}

#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
struct SimpleTableRichText(Vec<SimpleTableRichTextChunk>);

impl SimpleTableRichText {
    fn new(cell: &CardPageWritableSimpleTableCell) -> Self {
        let cell = cell.as_cell();
        if cell.text().is_empty() {
            return Self(Vec::new());
        }
        let boundaries = rich_text_boundaries(cell.text(), cell.annotations());
        let chunks = boundaries
            .windows(2)
            .map(|boundary| {
                rich_text_chunk(cell.text(), cell.annotations(), boundary[0], boundary[1])
            })
            .collect();
        Self(chunks)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum SimpleTableRichTextChunk {
    Plain([String; 1]),
    Annotated((String, Vec<Vec<Value>>)),
}

fn rich_text_boundaries(text: &str, spans: &[CardPageTextAnnotationSpan]) -> Vec<usize> {
    let mut boundaries = Vec::with_capacity(spans.len() * 2 + 2);
    boundaries.push(0);
    boundaries.push(text.len());
    for span in spans {
        boundaries.push(span.start_utf8);
        boundaries.push(span.end_utf8);
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
}

fn rich_text_chunk(
    text: &str,
    spans: &[CardPageTextAnnotationSpan],
    start: usize,
    end: usize,
) -> SimpleTableRichTextChunk {
    let segment = text[start..end].to_string();
    let mut annotations = spans
        .iter()
        .filter(|span| span.start_utf8 <= start && span.end_utf8 >= end)
        .map(|span| annotation_tuple(&span.annotation))
        .collect::<Vec<_>>();
    annotations.sort_by_cached_key(|tuple| Value::Array(tuple.clone()).to_string());
    if annotations.is_empty() {
        SimpleTableRichTextChunk::Plain([segment])
    } else {
        SimpleTableRichTextChunk::Annotated((segment, annotations))
    }
}
