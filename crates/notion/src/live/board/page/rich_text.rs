use super::super::{
    plain_text_from_property_value_with_page_titles, HashMap, PropertyLookup, Value,
};
use crate::live::board::simple_table_rich_text::{
    simple_table_annotation_tuple, simple_table_cell_round_trip,
};
use crate::model::{
    CardPageEditableReadOnlyReason, CardPageSimpleTableCell, CardPageSimpleTableCellReadOnlyReason,
    CardPageSimpleTableCellRoundTrip, CardPageTextAnnotationSpan, PageMention, PageMentionDate,
    PageTextAnnotation, PAGE_MENTION_TOKEN_STR,
};

/// Notion's inline equation token (U+204D). Equations are not modelled yet, so
/// a title carrying one stays read-only in notnotion.
const NOTION_EQUATION_TOKEN: &str = "\u{204d}";

#[derive(Default)]
pub(super) struct ParsedCardPageText {
    pub(super) text: String,
    pub(super) annotations: Vec<CardPageTextAnnotationSpan>,
    pub(super) read_only: Option<CardPageEditableReadOnlyReason>,
}

struct PendingAnnotationSpan {
    start_utf8: usize,
    end_utf8: usize,
    annotation: PageTextAnnotation,
}

#[derive(Default)]
struct PendingCardPageText {
    text: String,
    spans: Vec<PendingAnnotationSpan>,
    read_only: Option<CardPageEditableReadOnlyReason>,
}

pub(super) fn parse_card_page_text(
    value: &Value,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<ParsedCardPageText, String> {
    let chunks = value
        .as_array()
        .ok_or_else(|| "Notion block title must be a rich-text array".to_string())?;
    let mut pending = PendingCardPageText::default();
    for (chunk_index, chunk) in chunks.iter().enumerate() {
        append_card_page_text_chunk(chunk, chunk_index, lookup, page_title_cache, &mut pending)?;
    }
    let PendingCardPageText {
        text,
        spans,
        read_only,
    } = pending;
    let annotations = spans
        .into_iter()
        .map(|span| {
            CardPageTextAnnotationSpan::new(&text, span.start_utf8, span.end_utf8, span.annotation)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ParsedCardPageText {
        text,
        annotations,
        read_only,
    })
}

fn append_card_page_text_chunk(
    chunk: &Value,
    chunk_index: usize,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
    pending: &mut PendingCardPageText,
) -> Result<(), String> {
    let PendingCardPageText {
        text,
        spans,
        read_only,
    } = pending;
    let parts = chunk
        .as_array()
        .ok_or_else(|| format!("Notion title chunk {chunk_index} must be an array"))?;
    let token = parts
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion title chunk {chunk_index} must start with text"))?;
    let start_utf8 = text.len();
    let mention = if token == PAGE_MENTION_TOKEN_STR {
        mention_from_chunk(chunk, parts, chunk_index, lookup, page_title_cache)?
    } else {
        None
    };
    match mention {
        Some(mention) => {
            text.push_str(PAGE_MENTION_TOKEN_STR);
            spans.push(PendingAnnotationSpan {
                start_utf8,
                end_utf8: text.len(),
                annotation: PageTextAnnotation::Mention(mention),
            });
        }
        None => {
            if token == PAGE_MENTION_TOKEN_STR || token == NOTION_EQUATION_TOKEN {
                *read_only = Some(CardPageEditableReadOnlyReason::UnsupportedInlineToken);
            }
            text.push_str(&resolved_chunk_text(
                chunk,
                token,
                lookup,
                page_title_cache,
            )?);
        }
    }
    append_chunk_annotations(parts, chunk_index, start_utf8, text.len(), spans)
}

pub(super) fn parse_card_page_table_cell(
    value: &Value,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<CardPageSimpleTableCell, String> {
    let round_trip = simple_table_cell_round_trip(value);
    match parse_card_page_text(value, lookup, page_title_cache) {
        Ok(parsed) => match round_trip {
            CardPageSimpleTableCellRoundTrip::ReadOnly(reason) => {
                CardPageSimpleTableCell::read_only(parsed.text, parsed.annotations, reason)
            }
            CardPageSimpleTableCellRoundTrip::Writable => {
                CardPageSimpleTableCell::writable(parsed.text, parsed.annotations)
            }
        },
        Err(_) => CardPageSimpleTableCell::read_only(
            plain_text_from_property_value_with_page_titles(value, lookup, page_title_cache)?,
            Vec::new(),
            match round_trip {
                CardPageSimpleTableCellRoundTrip::ReadOnly(reason) => reason,
                CardPageSimpleTableCellRoundTrip::Writable => {
                    CardPageSimpleTableCellReadOnlyReason::UnsupportedShape
                }
            },
        ),
    }
}

/// Resolve the mention a `‣` chunk references, when notnotion models that kind.
///
/// Dates come from the annotation object; people and pages keep the display
/// text Notion's flattening resolves (a user name or a page title).
fn mention_from_chunk(
    chunk: &Value,
    parts: &[Value],
    chunk_index: usize,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<Option<PageMention>, String> {
    let Some(annotations) = parts.get(1).and_then(Value::as_array) else {
        return Ok(None);
    };
    for tuple in annotations.iter().filter_map(Value::as_array) {
        let Some(key) = tuple.first().and_then(Value::as_str) else {
            continue;
        };
        match key {
            "d" => {
                let value = tuple.get(1).ok_or_else(|| {
                    format!("Notion title chunk {chunk_index} date mention has no date object")
                })?;
                return PageMentionDate::from_notion_value(value)
                    .map(|date| Some(PageMention::Date(date)))
                    .map_err(|error| format!("Notion title chunk {chunk_index}: {error}"));
            }
            "u" => {
                let user_id = tuple
                    .get(1)
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        format!("Notion title chunk {chunk_index} user mention has no user id")
                    })?
                    .to_string();
                let display_name = resolved_chunk_text(chunk, "‣", lookup, page_title_cache)?;
                return Ok(Some(PageMention::User {
                    user_id,
                    display_name,
                }));
            }
            "p" => {
                let block_id = tuple
                    .get(1)
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        format!("Notion title chunk {chunk_index} page mention has no block id")
                    })?
                    .to_string();
                let title = resolved_chunk_text(chunk, "‣", lookup, page_title_cache)?;
                return Ok(Some(PageMention::Page { block_id, title }));
            }
            _ => {}
        }
    }
    Ok(None)
}

fn resolved_chunk_text(
    chunk: &Value,
    token: &str,
    lookup: PropertyLookup<'_>,
    page_title_cache: &mut HashMap<String, String>,
) -> Result<String, String> {
    if token != PAGE_MENTION_TOKEN_STR {
        return Ok(token.to_string());
    }
    plain_text_from_property_value_with_page_titles(
        &Value::Array(vec![chunk.clone()]),
        lookup,
        page_title_cache,
    )
}

fn append_chunk_annotations(
    parts: &[Value],
    chunk_index: usize,
    start_utf8: usize,
    end_utf8: usize,
    spans: &mut Vec<PendingAnnotationSpan>,
) -> Result<(), String> {
    let Some(serialized) = parts.get(1) else {
        return Ok(());
    };
    let annotations = serialized
        .as_array()
        .ok_or_else(|| format!("Notion title chunk {chunk_index} annotations must be an array"))?;
    for (annotation_index, serialized) in annotations.iter().enumerate() {
        let tuple = serialized.as_array().ok_or_else(|| {
            format!(
                "Notion title chunk {chunk_index} annotation {annotation_index} must be an array"
            )
        })?;
        let key = tuple.first().and_then(Value::as_str).ok_or_else(|| {
            format!("Notion title chunk {chunk_index} annotation {annotation_index} requires a key")
        })?;
        if let Some(annotation) = simple_table_annotation_tuple(tuple, key)? {
            append_normalized_span(spans, start_utf8, end_utf8, annotation);
        }
    }
    Ok(())
}

fn append_normalized_span(
    spans: &mut Vec<PendingAnnotationSpan>,
    start_utf8: usize,
    end_utf8: usize,
    annotation: PageTextAnnotation,
) {
    if start_utf8 == end_utf8 {
        return;
    }
    if let Some(previous) = spans
        .iter_mut()
        .rev()
        .find(|span| span.annotation == annotation)
    {
        if previous.start_utf8 == start_utf8 && previous.end_utf8 == end_utf8 {
            return;
        }
        if previous.end_utf8 == start_utf8 && annotation.merges_with_adjacent() {
            previous.end_utf8 = end_utf8;
            return;
        }
    }
    spans.push(PendingAnnotationSpan {
        start_utf8,
        end_utf8,
        annotation,
    });
}
