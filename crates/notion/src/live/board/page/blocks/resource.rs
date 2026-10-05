use super::super::{CardPageBlockApiType, Value};
use crate::model::{
    CardPageAttachmentDisplaySource, CardPageHttpsUrl, CardPageImageBlock,
    CardPageImageDisplayHost, CardPageImageFetchKey, CardPageImageSizeHint, CardPageImageSource,
    CardPageImageWidthTier, CardPageNotionAttachmentPointer, CardPageResourceBlock,
};

pub(super) fn page_resource_block(
    block_id: &str,
    block: &Value,
    api_type: &CardPageBlockApiType,
) -> Result<Option<CardPageResourceBlock>, String> {
    if api_type.as_str() != "image" {
        return Ok(None);
    }
    validate_image_children(block_id, block)?;
    let format = required_image_format(block_id, block)?;
    let size_hint = image_size_hint(block_id, format)?;
    let source = image_source(block_id, block, format)?;
    Ok(Some(CardPageResourceBlock::Image {
        image: CardPageImageBlock::new(source, size_hint),
    }))
}

fn validate_image_children(block_id: &str, block: &Value) -> Result<(), String> {
    match block.get("content") {
        None => Ok(()),
        Some(Value::Array(children)) if children.is_empty() => Ok(()),
        Some(Value::Array(_)) => Err(format!(
            "Notion image block {block_id} unexpectedly contains child content"
        )),
        Some(_) => Err(format!(
            "Notion image block {block_id} contains non-array child content"
        )),
    }
}

fn required_image_format<'a>(
    block_id: &str,
    block: &'a Value,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    block
        .get("format")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Notion image block {block_id} has no object format"))
}

fn image_size_hint(
    block_id: &str,
    format: &serde_json::Map<String, Value>,
) -> Result<Option<CardPageImageSizeHint>, String> {
    let width = format
        .get("block_width")
        .map(|value| image_dimension(block_id, "width", value))
        .transpose()?;
    let height = format
        .get("block_height")
        .map(|value| image_dimension(block_id, "height", value))
        .transpose()?;
    let height_width_ratio = format
        .get("block_aspect_ratio")
        .map(|value| image_height_width_ratio(block_id, value))
        .transpose()?;
    let dimensions = match (width, height, height_width_ratio) {
        (Some(width), Some(height), _) => Some((width, height)),
        (Some(width), None, Some(ratio)) => Some((width, width * ratio)),
        (None, Some(height), Some(ratio)) => Some((height / ratio, height)),
        _ => None,
    };
    let Some((width, height)) = dimensions else {
        return Ok(None);
    };
    let width = rounded_image_dimension(block_id, "width", width)?;
    let height = rounded_image_dimension(block_id, "height", height)?;
    image_size_hint_from_dimensions(block_id, width, height)
}

fn image_size_hint_from_dimensions(
    block_id: &str,
    width: u32,
    height: u32,
) -> Result<Option<CardPageImageSizeHint>, String> {
    CardPageImageSizeHint::new(width, height)
        .map(Some)
        .map_err(|error| format!("Notion image block {block_id}: {error}"))
}

fn image_height_width_ratio(block_id: &str, value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
        .ok_or_else(|| format!("Notion image block {block_id} has an invalid aspect ratio"))
}

fn rounded_image_dimension(block_id: &str, name: &str, value: f64) -> Result<u32, String> {
    if !value.is_finite() || value <= 0.0 || value > f64::from(u32::MAX) {
        return Err(format!(
            "Notion image block {block_id} has an out-of-range derived {name}"
        ));
    }
    let rounded = value.round();
    if rounded < 1.0 || rounded > f64::from(u32::MAX) {
        return Err(format!(
            "Notion image block {block_id} has an out-of-range derived {name}"
        ));
    }
    Ok(rounded as u32)
}

fn image_dimension(block_id: &str, name: &str, value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= f64::from(u32::MAX))
        .ok_or_else(|| format!("Notion image block {block_id} has an invalid {name}"))
}

fn image_source(
    block_id: &str,
    block: &Value,
    format: &serde_json::Map<String, Value>,
) -> Result<CardPageImageSource, String> {
    let source = nested_image_source(block_id, block)?;
    let Some(source) = source else {
        if !format.is_empty() {
            return Err(format!(
                "Notion image block {block_id} has format data without a source"
            ));
        }
        return Ok(CardPageImageSource::Empty);
    };
    let space_id = block
        .get("space_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Notion image block {block_id} has no space ID"))?;
    if source.starts_with("attachment:") {
        return notion_attachment_source(block_id, space_id, source, format);
    }
    external_https_source(block_id, space_id, source, format)
}

fn nested_image_source<'a>(block_id: &str, block: &'a Value) -> Result<Option<&'a str>, String> {
    let Some(properties) = block.get("properties") else {
        return Ok(None);
    };
    let properties = properties
        .as_object()
        .ok_or_else(|| format!("Notion image block {block_id} has non-object properties"))?;
    let Some(source) = properties.get("source") else {
        return Ok(None);
    };
    let rows = source.as_array().ok_or_else(|| {
        format!("Notion image block {block_id} source must be a nested text value")
    })?;
    let [row] = rows.as_slice() else {
        return Err(format!(
            "Notion image block {block_id} source must contain exactly one row"
        ));
    };
    let row = row
        .as_array()
        .ok_or_else(|| format!("Notion image block {block_id} source row must be an array"))?;
    let [value] = row.as_slice() else {
        return Err(format!(
            "Notion image block {block_id} source row must contain exactly one value"
        ));
    };
    value
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(Some)
        .ok_or_else(|| format!("Notion image block {block_id} has an empty source"))
}

fn external_https_source(
    block_id: &str,
    space_id: &str,
    source: &str,
    format: &serde_json::Map<String, Value>,
) -> Result<CardPageImageSource, String> {
    let source_url = CardPageHttpsUrl::new(source.to_string())
        .map_err(|error| format!("Notion image block {block_id}: {error}"))?;
    if let Some(display_source) = format.get("display_source") {
        let display_source = display_source.as_str().ok_or_else(|| {
            format!("Notion image block {block_id} has a non-string display source")
        })?;
        CardPageHttpsUrl::new(display_source.to_string())
            .map_err(|error| format!("Notion image block {block_id}: {error}"))?;
    }
    let fetch_key = image_fetch_key(block_id, space_id, source_url.as_str())?;
    let display_host = CardPageImageDisplayHost::from_url(&source_url);
    Ok(CardPageImageSource::ExternalHttps {
        source: source_url,
        display_host,
        fetch_key,
    })
}

fn notion_attachment_source(
    block_id: &str,
    space_id: &str,
    source: &str,
    format: &serde_json::Map<String, Value>,
) -> Result<CardPageImageSource, String> {
    let pointer = CardPageNotionAttachmentPointer::try_from(source.to_string())
        .map_err(|error| format!("Notion image block {block_id}: {error}"))?;
    let display_source = format
        .get("display_source")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion image block {block_id} has no attachment display source"))?;
    let (display_source, display_host) = attachment_display_source(block_id, display_source)?;
    let fetch_key = image_fetch_key(block_id, space_id, pointer.as_str())?;
    Ok(CardPageImageSource::NotionAttachment {
        pointer,
        display_source,
        display_host,
        fetch_key,
    })
}

type AttachmentDisplay = (
    CardPageAttachmentDisplaySource,
    Option<CardPageImageDisplayHost>,
);

fn attachment_display_source(
    block_id: &str,
    display_source: &str,
) -> Result<AttachmentDisplay, String> {
    if display_source.starts_with("attachment:") {
        let pointer = CardPageNotionAttachmentPointer::try_from(display_source.to_string())
            .map_err(|error| format!("Notion image block {block_id}: {error}"))?;
        return Ok((
            CardPageAttachmentDisplaySource::NotionAttachment(pointer),
            None,
        ));
    }
    let source = CardPageHttpsUrl::new(display_source.to_string())
        .map_err(|error| format!("Notion image block {block_id}: {error}"))?;
    let display_host = CardPageImageDisplayHost::from_url(&source);
    Ok((
        CardPageAttachmentDisplaySource::Https(source),
        Some(display_host),
    ))
}

fn image_fetch_key(
    block_id: &str,
    space_id: &str,
    source: &str,
) -> Result<CardPageImageFetchKey, String> {
    let width_tier = CardPageImageWidthTier::Px2000;
    let url = crate::live::notion_block_image_render_url(
        source,
        block_id,
        space_id,
        width_tier.pixels(),
    )?;
    let url = CardPageHttpsUrl::new(url)
        .map_err(|error| format!("Notion image block {block_id}: {error}"))?;
    Ok(CardPageImageFetchKey::new(url, width_tier))
}
