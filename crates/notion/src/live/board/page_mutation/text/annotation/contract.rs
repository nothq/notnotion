use serde_json::Value;

use crate::model::{PageTextAnnotation, PageTextAnnotationKind, PageTextColor};

pub(super) fn removal_expands(kind: PageTextAnnotationKind) -> bool {
    matches!(
        kind,
        PageTextAnnotationKind::Bold
            | PageTextAnnotationKind::Italic
            | PageTextAnnotationKind::Underline
            | PageTextAnnotationKind::Strike
            | PageTextAnnotationKind::Code
    )
}

pub(super) fn addition_end_expands(annotation: &PageTextAnnotation) -> bool {
    !matches!(
        annotation,
        PageTextAnnotation::Link(_) | PageTextAnnotation::Mention(_)
    )
}

pub(in crate::live::board::page_mutation) fn annotation_key(
    kind: PageTextAnnotationKind,
) -> &'static str {
    match kind {
        PageTextAnnotationKind::Bold => "b",
        PageTextAnnotationKind::Italic => "i",
        PageTextAnnotationKind::Underline => "_",
        PageTextAnnotationKind::Strike => "s",
        PageTextAnnotationKind::Code => "c",
        PageTextAnnotationKind::Link => "a",
        PageTextAnnotationKind::TextColor => "hf",
        PageTextAnnotationKind::BackgroundColor => "hb",
        PageTextAnnotationKind::Mention(mention) => mention.annotation_key(),
    }
}

pub(in crate::live::board::page_mutation) fn annotation_tuple(
    annotation: &PageTextAnnotation,
) -> Vec<Value> {
    let strings: Vec<String> = match annotation {
        PageTextAnnotation::Bold => vec!["b".to_string()],
        PageTextAnnotation::Italic => vec!["i".to_string()],
        PageTextAnnotation::Underline => vec!["_".to_string()],
        PageTextAnnotation::Strike => vec!["s".to_string()],
        PageTextAnnotation::Code => vec!["c".to_string()],
        PageTextAnnotation::Link(url) => vec!["a".to_string(), url.clone()],
        PageTextAnnotation::TextColor(color) => {
            vec!["h".to_string(), foreground_color(*color).to_string()]
        }
        PageTextAnnotation::BackgroundColor(color) => vec![
            "h".to_string(),
            format!("{}_background", foreground_color(*color)),
        ],
        PageTextAnnotation::Mention(mention) => return mention.annotation_tuple(),
    };
    strings.into_iter().map(Value::String).collect()
}

fn foreground_color(color: PageTextColor) -> &'static str {
    match color {
        PageTextColor::Default => "default",
        PageTextColor::Gray => "gray",
        PageTextColor::Brown => "brown",
        PageTextColor::Orange => "orange",
        PageTextColor::Yellow => "yellow",
        PageTextColor::Green => "teal",
        PageTextColor::Blue => "blue",
        PageTextColor::Purple => "purple",
        PageTextColor::Pink => "pink",
        PageTextColor::Red => "red",
    }
}
