use super::{
    block_requires_known_properties, last_editor_fields_are_complete_in, parse_page_block_value,
    PageBlockRecordState, PageBlockRequirement,
};
use crate::live::board::{
    loaded_record_value, record_is_deleted, record_map::record_value_mut, record_value_state, Map,
    RecordValueState, Value,
};

#[derive(Clone, Copy)]
pub(in crate::live::board::page_chunk) struct PageBlockOmissions {
    content: bool,
    format: bool,
    properties: bool,
    attribution: bool,
}

pub(in crate::live::board::page_chunk) enum AuthoritativePageBlockState {
    Complete(PageBlockOmissions),
    Unavailable(AuthoritativePageBlockUnavailable),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::live::board::page_chunk) enum AuthoritativePageBlockUnavailable {
    ExplicitRoleNone,
    Deleted,
    Missing,
    Fragment,
}

pub(in crate::live::board::page_chunk) fn normalize_authoritative_page_block_entry(
    entry: &mut Value,
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<AuthoritativePageBlockState, String> {
    if entry_has_role_none(entry) {
        match loaded_record_value(entry) {
            None => {
                return unavailable_authoritative_block(
                    block_id,
                    requirement,
                    AuthoritativePageBlockUnavailable::ExplicitRoleNone,
                );
            }
            Some(value) if record_is_deleted(value) => {
                return unavailable_authoritative_block(
                    block_id,
                    requirement,
                    AuthoritativePageBlockUnavailable::Deleted,
                );
            }
            Some(_) => {
                return Err(format!(
                    "authoritative Notion block {block_id} has role none and a live record value"
                ));
            }
        }
    }
    match record_value_state(Some(entry)) {
        RecordValueState::Unavailable => unreachable!("role none was classified above"),
        RecordValueState::Incomplete => unavailable_authoritative_fragment(
            block_id,
            requirement,
            "did not return a record value",
        ),
        RecordValueState::Present(value) if record_is_deleted(value) => {
            unavailable_authoritative_block(
                block_id,
                requirement,
                AuthoritativePageBlockUnavailable::Deleted,
            )
        }
        RecordValueState::Present(_) => {
            normalize_present_authoritative_page_block(entry, block_id, requirement)
        }
    }
}

fn normalize_present_authoritative_page_block(
    entry: &mut Value,
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<AuthoritativePageBlockState, String> {
    let value = loaded_record_value(entry)
        .ok_or_else(|| format!("authoritative Notion block {block_id} has no record value"))?;
    match value.get("type") {
        None => return incomplete_authoritative_block(block_id, requirement),
        Some(Value::String(_)) => {}
        Some(_) => {
            return Err(format!(
                "authoritative Notion block {block_id} contains a non-string block type"
            ));
        }
    }
    let mut normalized = entry.clone();
    let omissions = {
        let value = record_value_mut(&mut normalized)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("authoritative Notion block {block_id} has no object value"))?;
        let block_type = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("authoritative Notion block {block_id} has no block type"))?
            .to_string();
        let omissions = normalize_authoritative_fields(value, block_id, &block_type, requirement)?;
        normalize_optional_attribution(value);
        omissions
    };
    let value = loaded_record_value(&normalized)
        .ok_or_else(|| format!("authoritative Notion block {block_id} has no record value"))?;
    match parse_page_block_value(value, block_id, requirement)? {
        PageBlockRecordState::Complete(_) => {
            *entry = normalized;
            Ok(AuthoritativePageBlockState::Complete(omissions))
        }
        PageBlockRecordState::NoValue
        | PageBlockRecordState::Unavailable
        | PageBlockRecordState::Incomplete => incomplete_authoritative_block(block_id, requirement),
    }
}

/// Notion's authoritative record is final: requesting it again returns the
/// same fragment.
fn incomplete_authoritative_block(
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<AuthoritativePageBlockState, String> {
    unavailable_authoritative_fragment(block_id, requirement, "returned an incomplete record")
}

fn unavailable_authoritative_fragment(
    block_id: &str,
    requirement: PageBlockRequirement,
    fragment: &str,
) -> Result<AuthoritativePageBlockState, String> {
    match requirement {
        PageBlockRequirement::ReferenceTarget => Ok(AuthoritativePageBlockState::Unavailable(
            AuthoritativePageBlockUnavailable::Fragment,
        )),
        PageBlockRequirement::Root | PageBlockRequirement::ContentChild => {
            Err(format!("required Notion block {block_id} {fragment}"))
        }
    }
}

fn unavailable_authoritative_block(
    block_id: &str,
    requirement: PageBlockRequirement,
    unavailable: AuthoritativePageBlockUnavailable,
) -> Result<AuthoritativePageBlockState, String> {
    match requirement {
        PageBlockRequirement::ContentChild | PageBlockRequirement::ReferenceTarget => {
            Ok(AuthoritativePageBlockState::Unavailable(unavailable))
        }
        PageBlockRequirement::Root => {
            Err(format!("required Notion block {block_id} is unavailable"))
        }
    }
}

pub(in crate::live::board::page_chunk) fn missing_authoritative_page_block(
    block_id: &str,
    requirement: PageBlockRequirement,
) -> Result<AuthoritativePageBlockState, String> {
    match requirement {
        PageBlockRequirement::ContentChild | PageBlockRequirement::ReferenceTarget => Ok(
            AuthoritativePageBlockState::Unavailable(AuthoritativePageBlockUnavailable::Missing),
        ),
        PageBlockRequirement::Root => Err(format!(
            "Notion did not hydrate required page block {block_id}"
        )),
    }
}

pub(in crate::live::board::page_chunk) fn unavailable_page_block_kind(
    entry: &Value,
    block_id: &str,
) -> Result<AuthoritativePageBlockUnavailable, String> {
    if entry_has_role_none(entry) && loaded_record_value(entry).is_none() {
        return Ok(AuthoritativePageBlockUnavailable::ExplicitRoleNone);
    }
    if loaded_record_value(entry).is_some_and(record_is_deleted) {
        return Ok(AuthoritativePageBlockUnavailable::Deleted);
    }
    Err(format!(
        "unavailable Notion block {block_id} has neither role none nor a deleted record"
    ))
}

fn entry_has_role_none(entry: &Value) -> bool {
    entry.get("role").and_then(Value::as_str) == Some("none")
        || entry
            .get("value")
            .and_then(Value::as_object)
            .and_then(|wrapper| wrapper.get("role"))
            .and_then(Value::as_str)
            == Some("none")
}

pub(in crate::live::board::page_chunk) fn apply_authoritative_page_block_omissions(
    existing_entry: &mut Value,
    authoritative_entry: &Value,
    omissions: PageBlockOmissions,
    block_id: &str,
) -> Result<(), String> {
    if record_version(existing_entry, block_id)? != record_version(authoritative_entry, block_id)? {
        return Ok(());
    }
    let Some(existing) = record_value_mut(existing_entry).and_then(Value::as_object_mut) else {
        return Ok(());
    };
    for (field, omitted) in [
        ("content", omissions.content),
        ("format", omissions.format),
        ("properties", omissions.properties),
    ] {
        if omitted {
            existing.remove(field);
        }
    }
    if omissions.attribution {
        existing.remove("last_edited_by_table");
        existing.remove("last_edited_by_id");
    }
    Ok(())
}

fn normalize_authoritative_fields(
    value: &mut Map<String, Value>,
    block_id: &str,
    block_type: &str,
    requirement: PageBlockRequirement,
) -> Result<PageBlockOmissions, String> {
    let attribution = !has_complete_attribution(value);
    let content = normalize_content(value, block_id, requirement.requires_content(block_type))?;
    let format = normalize_object(value, block_id, "format")?;
    let properties = if block_requires_known_properties(block_type) {
        normalize_object(value, block_id, "properties")?
    } else {
        false
    };
    Ok(PageBlockOmissions {
        content,
        format,
        properties,
        attribution,
    })
}

fn normalize_content(
    value: &mut Map<String, Value>,
    block_id: &str,
    required: bool,
) -> Result<bool, String> {
    match value.get("content") {
        Some(Value::Array(_)) | None | Some(Value::Null) if !required => return Ok(false),
        Some(Value::Array(_)) => return Ok(false),
        None | Some(Value::Null) => {}
        Some(_) => {
            return Err(format!(
                "authoritative Notion block {block_id} contains non-array field content"
            ));
        }
    }
    value.insert("content".to_string(), Value::Array(Vec::new()));
    Ok(true)
}

fn normalize_object(
    value: &mut Map<String, Value>,
    block_id: &str,
    field: &str,
) -> Result<bool, String> {
    match value.get(field) {
        Some(Value::Object(_)) => return Ok(false),
        None | Some(Value::Null) => {}
        Some(_) => {
            return Err(format!(
                "authoritative Notion block {block_id} contains non-object field {field}"
            ));
        }
    }
    value.insert(field.to_string(), Value::Object(Map::new()));
    Ok(true)
}

fn normalize_optional_attribution(value: &mut Map<String, Value>) {
    if last_editor_fields_are_complete_in(value) {
        return;
    }
    value.remove("last_edited_by_table");
    value.remove("last_edited_by_id");
}

fn has_complete_attribution(value: &Map<String, Value>) -> bool {
    matches!(
        (
            value.get("last_edited_by_table"),
            value.get("last_edited_by_id")
        ),
        (Some(Value::String(table)), Some(Value::String(id)))
            if !table.trim().is_empty() && !id.trim().is_empty()
    )
}

fn record_version(entry: &Value, block_id: &str) -> Result<Option<u64>, String> {
    let Some(value) = loaded_record_value(entry) else {
        return Ok(None);
    };
    super::optional_u64_field(value, block_id, "version")
}
