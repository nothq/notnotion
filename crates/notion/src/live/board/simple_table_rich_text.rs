use crate::model::{
    CardPageSimpleTableCellReadOnlyReason, CardPageSimpleTableCellRoundTrip, PageTextAnnotation,
    PageTextColor,
};
use serde_json::Value;

pub(in crate::live::board) fn simple_table_cell_round_trip(
    value: &Value,
) -> CardPageSimpleTableCellRoundTrip {
    let Some(chunks) = value.as_array() else {
        return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
    };
    for chunk in chunks {
        let Some(parts) = chunk.as_array() else {
            return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
        };
        let Some(token) = parts.first().and_then(Value::as_str) else {
            return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
        };
        if token == "‣" {
            return read_only(CardPageSimpleTableCellReadOnlyReason::SemanticToken);
        }
        if parts.len() > 2 {
            return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
        }
        let Some(serialized) = parts.get(1) else {
            continue;
        };
        let Some(annotations) = serialized.as_array() else {
            return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
        };
        for serialized in annotations {
            let Some(tuple) = serialized.as_array() else {
                return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
            };
            let Some(key) = tuple.first().and_then(Value::as_str) else {
                return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape);
            };
            match simple_table_annotation_tuple(tuple, key) {
                Ok(Some(_)) => {}
                Ok(None) => {
                    return read_only(CardPageSimpleTableCellReadOnlyReason::UnknownAnnotation)
                }
                Err(_) => {
                    return read_only(CardPageSimpleTableCellReadOnlyReason::UnsupportedShape)
                }
            }
        }
    }
    CardPageSimpleTableCellRoundTrip::Writable
}

pub(in crate::live::board) fn simple_table_annotation_tuple(
    tuple: &[Value],
    key: &str,
) -> Result<Option<PageTextAnnotation>, String> {
    let annotation = match key {
        "b" => unit_annotation(tuple, PageTextAnnotation::Bold)?,
        "i" => unit_annotation(tuple, PageTextAnnotation::Italic)?,
        "_" => unit_annotation(tuple, PageTextAnnotation::Underline)?,
        "s" => unit_annotation(tuple, PageTextAnnotation::Strike)?,
        "c" => unit_annotation(tuple, PageTextAnnotation::Code)?,
        "a" => PageTextAnnotation::Link(annotation_value(tuple, "link")?.to_string()),
        "h" => parse_color_annotation(annotation_value(tuple, "color")?)?,
        _ => return Ok(None),
    };
    Ok(Some(annotation))
}

fn read_only(reason: CardPageSimpleTableCellReadOnlyReason) -> CardPageSimpleTableCellRoundTrip {
    CardPageSimpleTableCellRoundTrip::ReadOnly(reason)
}

fn unit_annotation(
    tuple: &[Value],
    annotation: PageTextAnnotation,
) -> Result<PageTextAnnotation, String> {
    if tuple.len() != 1 {
        return Err(format!(
            "Notion {:?} annotation must contain only its key",
            annotation.kind()
        ));
    }
    Ok(annotation)
}

fn annotation_value<'a>(tuple: &'a [Value], label: &str) -> Result<&'a str, String> {
    if tuple.len() != 2 {
        return Err(format!(
            "Notion {label} annotation must contain one string value"
        ));
    }
    let value = tuple[1]
        .as_str()
        .ok_or_else(|| format!("Notion {label} annotation value must be a string"))?;
    if value.trim().is_empty() {
        return Err(format!("Notion {label} annotation value cannot be blank"));
    }
    Ok(value)
}

fn parse_color_annotation(value: &str) -> Result<PageTextAnnotation, String> {
    let (name, background) = value
        .strip_suffix("_background")
        .map_or((value, false), |name| (name, true));
    let color = match name {
        "default" => PageTextColor::Default,
        "gray" => PageTextColor::Gray,
        "brown" => PageTextColor::Brown,
        "orange" => PageTextColor::Orange,
        "yellow" => PageTextColor::Yellow,
        "teal" => PageTextColor::Green,
        "blue" => PageTextColor::Blue,
        "purple" => PageTextColor::Purple,
        "pink" => PageTextColor::Pink,
        "red" => PageTextColor::Red,
        _ => return Err(format!("unsupported Notion text color {value}")),
    };
    Ok(if background {
        PageTextAnnotation::BackgroundColor(color)
    } else {
        PageTextAnnotation::TextColor(color)
    })
}
