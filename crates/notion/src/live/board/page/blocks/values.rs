use super::super::{
    collection_database_name, collection_entry, loaded_record_value, page_collection_id,
    parse_card_page_text, title_property_allow_empty, CardPageBlockApiType, CardPageBlockColor,
    CardPageBlockKind, CardPageBlockLastEdited, CardPageCodeLanguage, CardPageCodeWrap,
    CardPageEditableBlock, CardPageQuoteSize, CardPageStructuralBlock, CardPageToDoState, Map,
    PageBlockParseContext, Value,
};
use super::ParsedPageBlocks;

pub(super) fn page_editable_block(
    context: &PageBlockParseContext<'_>,
    block_id: &str,
    block: &Value,
    api_type: &CardPageBlockApiType,
    parsed: &mut ParsedPageBlocks,
) -> Result<Option<CardPageEditableBlock>, String> {
    let Some(kind) = page_block_kind(api_type) else {
        return Ok(None);
    };
    let rich_text = block
        .get("properties")
        .and_then(|properties| properties.get("title"))
        .map(|title| {
            parse_card_page_text(
                title,
                context.property_lookup(),
                &mut parsed.page_title_cache,
            )
        })
        .transpose()?
        .unwrap_or_default();
    let mut editable = if kind == CardPageBlockKind::ToDoList {
        CardPageEditableBlock::to_do(
            rich_text.text,
            rich_text.annotations,
            page_to_do_state(block)?,
        )
    } else if kind == CardPageBlockKind::Quote {
        CardPageEditableBlock::quote(
            rich_text.text,
            rich_text.annotations,
            page_quote_size(block)?,
        )
    } else if kind == CardPageBlockKind::Code {
        CardPageEditableBlock::code(
            rich_text.text,
            rich_text.annotations,
            page_code_language(block, block_id)?,
            page_code_wrap(block, block_id)?,
        )
    } else {
        CardPageEditableBlock::new(kind, rich_text.text, rich_text.annotations)
    };
    editable.read_only = rich_text.read_only;
    Ok(Some(editable))
}

fn page_code_language(block: &Value, block_id: &str) -> Result<CardPageCodeLanguage, String> {
    let Some(properties) = block.get("properties") else {
        return Ok(CardPageCodeLanguage::default());
    };
    let properties = properties
        .as_object()
        .ok_or_else(|| format!("Notion code block {block_id} contains non-object properties"))?;
    let Some(language) = properties.get("language") else {
        return Ok(CardPageCodeLanguage::default());
    };
    let rows = language.as_array().ok_or_else(|| {
        format!("Notion code block {block_id} language property must be a single nested text value")
    })?;
    if rows.len() != 1 {
        return Err(format!(
            "Notion code block {block_id} language property must contain exactly one row"
        ));
    }
    let row = rows[0].as_array().ok_or_else(|| {
        format!("Notion code block {block_id} language property row must be an array")
    })?;
    if row.len() != 1 {
        return Err(format!(
            "Notion code block {block_id} language property row must contain exactly one value"
        ));
    }
    let language = row[0].as_str().ok_or_else(|| {
        format!("Notion code block {block_id} language property value must be a string")
    })?;
    CardPageCodeLanguage::from_api_value(language)
        .map_err(|error| format!("Notion code block {block_id}: {error}"))
}

fn page_code_wrap(block: &Value, block_id: &str) -> Result<CardPageCodeWrap, String> {
    let Some(format) = block.get("format") else {
        return Ok(CardPageCodeWrap::default());
    };
    let format = format
        .as_object()
        .ok_or_else(|| format!("Notion code block {block_id} contains non-object format data"))?;
    let Some(wrap) = format.get("code_wrap") else {
        return Ok(CardPageCodeWrap::default());
    };
    let wrap = wrap.as_bool().ok_or_else(|| {
        format!("Notion code block {block_id} contains a non-boolean code-wrap value")
    })?;
    Ok(CardPageCodeWrap::from_api_value(wrap))
}

fn page_quote_size(block: &Value) -> Result<CardPageQuoteSize, String> {
    let raw = match block
        .get("format")
        .and_then(|format| format.get("quote_size"))
    {
        None | Some(Value::Null) => None,
        Some(Value::String(size)) => Some(size.as_str()),
        Some(_) => return Err("Notion quote size is not a string or null".to_string()),
    };
    CardPageQuoteSize::from_api_value(raw)
}

pub(super) fn page_block_last_edited(
    block: &Value,
    users: Option<&Map<String, Value>>,
) -> Option<CardPageBlockLastEdited> {
    let timestamp_ms = block.get("last_edited_time")?.as_u64()?;
    if block.get("last_edited_by_table").and_then(Value::as_str) != Some("notion_user") {
        return None;
    }
    let editor_id = block.get("last_edited_by_id")?.as_str()?;
    let editor_name = users?
        .get(editor_id)
        .and_then(loaded_record_value)?
        .get("name")?
        .as_str()?
        .to_string();
    Some(CardPageBlockLastEdited {
        editor_name,
        timestamp_ms,
    })
}

fn page_to_do_state(block: &Value) -> Result<CardPageToDoState, String> {
    let Some(checked) = block
        .get("properties")
        .and_then(|properties| properties.get("checked"))
    else {
        return Ok(CardPageToDoState::Unchecked);
    };
    let value = checked
        .as_array()
        .and_then(|rows| rows.as_slice().first())
        .and_then(Value::as_array)
        .and_then(|row| row.as_slice().first())
        .and_then(Value::as_str)
        .ok_or_else(|| "Notion to-do checked property is not a nested text value".to_string())?;
    match value {
        "Yes" => Ok(CardPageToDoState::Checked),
        "No" => Ok(CardPageToDoState::Unchecked),
        _ => Err(format!("unsupported Notion to-do checked value {value}")),
    }
}

pub(super) fn page_block_color(
    block: &Value,
    block_id: &str,
) -> Result<CardPageBlockColor, String> {
    let raw = match block
        .get("format")
        .and_then(|format| format.get("block_color"))
    {
        Some(Value::String(color)) => Some(color.as_str()),
        Some(_) => {
            return Err(format!(
                "Notion block {block_id} contains a non-string block color"
            ))
        }
        None => None,
    };
    CardPageBlockColor::from_api_value(raw)
        .map_err(|error| format!("Notion block {block_id}: {error}"))
}

fn page_block_kind(api_type: &CardPageBlockApiType) -> Option<CardPageBlockKind> {
    match api_type.as_str() {
        "text" => Some(CardPageBlockKind::Text),
        "header" => Some(CardPageBlockKind::SubHeader),
        "sub_header" => Some(CardPageBlockKind::SubSubHeader),
        "sub_sub_header" => Some(CardPageBlockKind::Heading3),
        "header_4" => Some(CardPageBlockKind::Heading4),
        "bulleted_list" => Some(CardPageBlockKind::BulletedList),
        "numbered_list" => Some(CardPageBlockKind::NumberedList),
        "to_do" => Some(CardPageBlockKind::ToDoList),
        "toggle" => Some(CardPageBlockKind::ToggleList),
        "page" => Some(CardPageBlockKind::PageLink),
        "callout" => Some(CardPageBlockKind::Callout),
        "quote" => Some(CardPageBlockKind::Quote),
        "code" => Some(CardPageBlockKind::Code),
        _ => None,
    }
}

pub(super) fn page_structural_block(
    block: &Value,
    api_type: &CardPageBlockApiType,
    collections: Option<&Map<String, Value>>,
) -> Result<Option<CardPageStructuralBlock>, String> {
    Ok(match api_type.as_str() {
        "divider" => Some(CardPageStructuralBlock::Divider),
        "collection_view" => Some(CardPageStructuralBlock::CollectionView {
            title: collection_block_title(block, collections)?,
        }),
        "collection_view_page" => Some(CardPageStructuralBlock::CollectionViewPage {
            title: collection_block_title(block, collections)?,
        }),
        _ => None,
    })
}

pub(super) fn collection_block_title(
    block: &Value,
    collections: Option<&Map<String, Value>>,
) -> Result<Option<String>, String> {
    if let Some(title) = title_property_allow_empty(block)?.filter(|title| !title.trim().is_empty())
    {
        return Ok(Some(title));
    }
    let Some(collection_id) = page_collection_id(block) else {
        return Ok(None);
    };
    let Some(collection) =
        collections.and_then(|records| collection_entry(records, collection_id).ok())
    else {
        return Ok(None);
    };
    collection_database_name(collection).map(Some)
}
