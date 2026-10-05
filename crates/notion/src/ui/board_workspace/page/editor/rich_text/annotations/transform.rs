use std::ops::Range;

use crate::model::{CardPageEditableBlock, CardPageTextAnnotationSpan};

use super::spans::merge_adjacent_annotations;

#[derive(Clone, Debug)]
pub(in crate::ui::board_workspace::page::editor) struct AnnotatedText {
    pub(in crate::ui::board_workspace::page::editor) text: String,
    pub(in crate::ui::board_workspace::page::editor) annotations: Vec<CardPageTextAnnotationSpan>,
}

impl AnnotatedText {
    pub(in crate::ui::board_workspace::page::editor) fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            annotations: Vec::new(),
        }
    }
}

pub(in crate::ui::board_workspace::page::editor) fn annotated_text_slice(
    editable: &CardPageEditableBlock,
    range: Range<usize>,
) -> AnnotatedText {
    validate_range(&editable.text, &range);
    let annotations = editable
        .annotations
        .iter()
        .filter_map(|span| clipped_annotation_span(span, &range))
        .collect();
    AnnotatedText {
        text: editable.text[range].to_string(),
        annotations,
    }
}

pub(in crate::ui::board_workspace::page::editor) fn concatenate_annotated_text(
    parts: impl IntoIterator<Item = AnnotatedText>,
) -> AnnotatedText {
    let mut text = String::new();
    let mut annotations = Vec::new();
    for mut part in parts {
        let offset = text.len();
        text.push_str(&part.text);
        for span in &mut part.annotations {
            span.start_utf8 += offset;
            span.end_utf8 += offset;
        }
        annotations.extend(part.annotations);
    }
    merge_adjacent_annotations(&mut annotations);
    validate_annotations(&text, &annotations);
    AnnotatedText { text, annotations }
}

pub(in crate::ui::board_workspace::page::editor) fn apply_annotated_text(
    editable: &mut CardPageEditableBlock,
    annotated: AnnotatedText,
) {
    validate_annotations(&annotated.text, &annotated.annotations);
    editable.text = annotated.text;
    editable.annotations = annotated.annotations;
}

pub(in crate::ui::board_workspace::page::editor) fn replace_annotated_text_range(
    editable: &mut CardPageEditableBlock,
    range: Range<usize>,
    replacement: &str,
) {
    let prefix = annotated_text_slice(editable, 0..range.start);
    let suffix = annotated_text_slice(editable, range.end..editable.text.len());
    let annotated = concatenate_annotated_text([prefix, AnnotatedText::plain(replacement), suffix]);
    apply_annotated_text(editable, annotated);
}

fn clipped_annotation_span(
    span: &CardPageTextAnnotationSpan,
    range: &Range<usize>,
) -> Option<CardPageTextAnnotationSpan> {
    let start = span.start_utf8.max(range.start);
    let end = span.end_utf8.min(range.end);
    (start < end).then(|| CardPageTextAnnotationSpan {
        start_utf8: start - range.start,
        end_utf8: end - range.start,
        annotation: span.annotation.clone(),
    })
}

fn validate_range(text: &str, range: &Range<usize>) {
    assert!(
        range.start <= range.end
            && range.end <= text.len()
            && text.is_char_boundary(range.start)
            && text.is_char_boundary(range.end),
        "annotated-text range must fit the text and follow UTF-8 boundaries"
    );
}

fn validate_annotations(text: &str, annotations: &[CardPageTextAnnotationSpan]) {
    assert!(
        annotations.iter().all(|span| {
            span.start_utf8 < span.end_utf8
                && span.end_utf8 <= text.len()
                && text.is_char_boundary(span.start_utf8)
                && text.is_char_boundary(span.end_utf8)
        }),
        "annotated-text spans must fit the text and follow UTF-8 boundaries"
    );
}
