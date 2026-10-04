use serde_json::Value;

use super::super::super::{record_is_deleted, record_value_state, RecordValueState};
use super::super::{explicit_page_shell_icon, title_property_allow_empty, PageBlockParseContext};
use super::values::collection_block_title;
use crate::model::{CardPageAliasBlock, PageShellIcon};

pub(super) fn page_alias_block(
    block: &Value,
    block_id: &str,
    context: &PageBlockParseContext<'_>,
) -> Result<CardPageAliasBlock, String> {
    let (target_block_id, target_space_id) = page_alias_pointer(block, block_id)?;
    let (title, icon) =
        resolved_alias_presentation(context, block_id, target_block_id, target_space_id)?;
    Ok(CardPageAliasBlock {
        target_block_id: target_block_id.to_string(),
        target_space_id: target_space_id.to_string(),
        copied_from_block_id: alias_copy_source_block_id(block, block_id)?,
        title,
        icon,
    })
}

fn resolved_alias_presentation(
    context: &PageBlockParseContext<'_>,
    block_id: &str,
    target_block_id: &str,
    target_space_id: &str,
) -> Result<(String, PageShellIcon), String> {
    let target = match record_value_state(context.blocks.get(target_block_id)) {
        RecordValueState::Unavailable => {
            return Ok(("Untitled".to_string(), PageShellIcon::named("page")));
        }
        RecordValueState::Incomplete => {
            if context.allow_partial_preview {
                return Ok(("Untitled".to_string(), PageShellIcon::named("page")));
            }
            return Err(format!(
                "Notion alias block {block_id} target {target_block_id} was not hydrated"
            ));
        }
        RecordValueState::Present(target) if record_is_deleted(target) => {
            return Ok(("Untitled".to_string(), PageShellIcon::named("page")));
        }
        RecordValueState::Present(target) => target,
    };
    if target.get("space_id").and_then(Value::as_str) != Some(target_space_id) {
        return Err(format!(
            "Notion alias block {block_id} target {target_block_id} belongs to a different space"
        ));
    }
    let target_type = target
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Notion alias target {target_block_id} has no block type"))?;
    let (title, fallback_icon) =
        alias_target_presentation(target, target_type, context, block_id, target_block_id)?;
    let icon = explicit_page_shell_icon(target, fallback_icon)
        .unwrap_or_else(|| PageShellIcon::named(fallback_icon));
    Ok((title, icon))
}

fn page_alias_pointer<'a>(block: &'a Value, block_id: &str) -> Result<(&'a str, &'a str), String> {
    let pointer = block
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("alias_pointer"))
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Notion alias block {block_id} is missing its alias pointer"))?;
    if pointer.get("table").and_then(Value::as_str) != Some("block") {
        return Err(format!(
            "Notion alias block {block_id} points to a non-block record"
        ));
    }
    let target_block_id = pointer
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion alias block {block_id} has an empty target block ID"))?;
    let target_space_id = pointer
        .get("spaceId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion alias block {block_id} has an empty target space ID"))?;
    Ok((target_block_id, target_space_id))
}

fn alias_target_presentation(
    target: &Value,
    target_type: &str,
    context: &PageBlockParseContext<'_>,
    block_id: &str,
    target_block_id: &str,
) -> Result<(String, &'static str), String> {
    match target_type {
        "page" | "transcription" => Ok((
            title_property_allow_empty(target)?
                .filter(|title| !title.trim().is_empty())
                .unwrap_or_else(|| "Untitled".to_string()),
            "page",
        )),
        "collection_view_page" => Ok((
            collection_block_title(target, context.collections)?
                .unwrap_or_else(|| "Untitled".to_string()),
            "database",
        )),
        _ => Err(format!(
            "Notion alias block {block_id} points to unsupported {target_type} block {target_block_id}"
        )),
    }
}

fn alias_copy_source_block_id(block: &Value, block_id: &str) -> Result<Option<String>, String> {
    let copied_from = block.get("copied_from");
    let Some(raw_pointer) = block
        .get("format")
        .and_then(Value::as_object)
        .and_then(|format| format.get("copied_from_pointer"))
    else {
        if copied_from.is_some() {
            return Err(format!(
                "Notion alias block {block_id} has copy metadata without a source pointer"
            ));
        }
        return Ok(None);
    };
    let pointer = raw_pointer.as_object().ok_or_else(|| {
        format!("Notion alias block {block_id} has an invalid copied-from pointer")
    })?;
    if pointer.get("table").and_then(Value::as_str) != Some("block") {
        return Err(format!(
            "Notion alias block {block_id} was copied from a non-block record"
        ));
    }
    let source_block_id = pointer
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion alias block {block_id} has an empty copy source ID"))?;
    let source_space_id = pointer
        .get("spaceId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| format!("Notion alias block {block_id} has an empty copy source space"))?;
    if block.get("space_id").and_then(Value::as_str) != Some(source_space_id) {
        return Err(format!(
            "Notion alias block {block_id} was copied from another space"
        ));
    }
    if copied_from.and_then(Value::as_str) != Some(source_block_id) {
        return Err(format!(
            "Notion alias block {block_id} has inconsistent copy provenance"
        ));
    }
    Ok(Some(source_block_id.to_string()))
}
